//! wgpu 2D textured-quad renderer: tilemap, sprites, UI panels, bitmap text.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::assets::{AssetStore, LoadedSheet};
use crate::ecs::{
    AnimationKind, AnimationState, Facing, Player, Sprite, SpriteSheet, Transform, World,
};
use crate::map::Tilemap;
use crate::skills::SkillRegistry;
use crate::render::{
    self, build_font_atlas_rgba, facing_sprite_row, tilemap_is_binary_collision_only,
    wang_wall_sheet_index, RenderScale,
};
use crate::gpu::color::{packed_rgb_to_linear, sprite_color_to_linear};
use crate::ui::{layout, UiTheme};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenUniform {
    screen_size: [f32; 2],
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

struct TextureBind {
    bind_group: wgpu::BindGroup,
}

pub struct GpuRenderer {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    bind_group_layout_tex: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    white: TextureBind,
    font: TextureBind,
    tileset: Option<TextureBind>,
    characters: HashMap<String, TextureBind>,
    screen_w: u32,
    screen_h: u32,
}

#[derive(Default)]
struct SubBatch {
    verts: Vec<GpuVertex>,
    indices: Vec<u32>,
}

impl SubBatch {
    fn push_quad(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        color: [f32; 4],
    ) {
        let base = self.verts.len() as u32;
        self.verts.extend_from_slice(&[
            GpuVertex {
                position: [x0, y0],
                uv: [u0, v0],
                color,
            },
            GpuVertex {
                position: [x1, y0],
                uv: [u1, v0],
                color,
            },
            GpuVertex {
                position: [x1, y1],
                uv: [u1, v1],
                color,
            },
            GpuVertex {
                position: [x0, y1],
                uv: [u0, v1],
                color,
            },
        ]);
        self.indices.extend_from_slice(&[
            base,
            base + 1,
            base + 2,
            base,
            base + 2,
            base + 3,
        ]);
    }

    fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

fn upload_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn sheet_to_rgba(sheet: &LoadedSheet) -> Vec<u8> {
    let n = (sheet.width * sheet.height * 4) as usize;
    let mut v = vec![0u8; n];
    for i in 0..sheet.pixels.len() {
        let p = sheet.pixels[i];
        let r = ((p >> 16) & 0xff) as u8;
        let g = ((p >> 8) & 0xff) as u8;
        let b = (p & 0xff) as u8;
        let a = if p == 0 { 0 } else { 255 };
        let o = i * 4;
        v[o] = r;
        v[o + 1] = g;
        v[o + 2] = b;
        v[o + 3] = a;
    }
    v
}

fn make_bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    view: &wgpu::TextureView,
) -> TextureBind {
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    TextureBind { bind_group }
}

fn font_cell_uv(c: u8) -> (f32, f32, f32, f32) {
    use crate::render::text::{FONT_ATLAS_CELL, FONT_ATLAS_COLS, FONT_ATLAS_ROWS};
    let aw = (FONT_ATLAS_COLS * FONT_ATLAS_CELL) as f32;
    let ah = (FONT_ATLAS_ROWS * FONT_ATLAS_CELL) as f32;
    let idx = if (32..127).contains(&c) {
        (c - 32) as u32
    } else {
        0
    };
    let col = idx % FONT_ATLAS_COLS;
    let row = idx / FONT_ATLAS_COLS;
    let u0 = (col * FONT_ATLAS_CELL) as f32 / aw;
    let u1 = ((col + 1) * FONT_ATLAS_CELL) as f32 / aw;
    let v0 = (row * FONT_ATLAS_CELL) as f32 / ah;
    let v1 = ((row + 1) * FONT_ATLAS_CELL) as f32 / ah;
    (u0, v0, u1, v1)
}

fn draw_text_sub(
    target: &mut SubBatch,
    mut x: f32,
    y: f32,
    text: &str,
    color: [f32; 4],
    scale: f32,
) {
    const GLYPH_W: f32 = 5.0;
    const GLYPH_H: f32 = 7.0;
    const GLYPH_STEP: f32 = 6.0;
    let step = GLYPH_STEP * scale;
    let gw = GLYPH_W * scale;
    let gh = GLYPH_H * scale;
    for b in text.bytes() {
        let (u0, v0, u1, v1) = font_cell_uv(b);
        target.push_quad(x, y, x + gw, y + gh, u0, v0, u1, v1, color);
        x += step;
    }
}

impl GpuRenderer {
    pub fn new(window: Arc<Window>) -> Result<Self, String> {
        let size = window.inner_size();
        let (screen_w, screen_h) = (size.width.max(1), size.height.max(1));

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| "no suitable wgpu adapter".to_string())?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
            },
            None,
        ))
        .map_err(|e| e.to_string())?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: screen_w,
            height: screen_h,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bangbang_sprite"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("screen_uniform"),
            contents: bytemuck::bytes_of(&ScreenUniform {
                screen_size: [screen_w as f32, screen_h as f32],
                _pad: [0.0, 0.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout_uniform =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("uniform_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout_uniform,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let bind_group_layout_tex = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tex_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout_uniform, &bind_group_layout_tex],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                        2 => Float32x4,
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let white_view = upload_rgba(&device, &queue, "white", 1, 1, &[255, 255, 255, 255]);
        let white = make_bind(&device, &bind_group_layout_tex, &sampler, &white_view);

        let (font_rgba, fw, fh) = build_font_atlas_rgba();
        let font_view = upload_rgba(&device, &queue, "font", fw, fh, &font_rgba);
        let font = make_bind(&device, &bind_group_layout_tex, &sampler, &font_view);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            bind_group_layout_tex,
            sampler,
            white,
            font,
            tileset: None,
            characters: HashMap::new(),
            screen_w,
            screen_h,
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.screen_w = width;
        self.screen_h = height;
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        let u = ScreenUniform {
            screen_size: [width as f32, height as f32],
            _pad: [0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&u));
    }

    fn ensure_tileset(&mut self, sheet: &LoadedSheet) {
        if self.tileset.is_some() {
            return;
        }
        let rgba = sheet_to_rgba(sheet);
        let view = upload_rgba(
            &self.device,
            &self.queue,
            "tileset",
            sheet.width,
            sheet.height,
            &rgba,
        );
        let bind = make_bind(
            &self.device,
            &self.bind_group_layout_tex,
            &self.sampler,
            &view,
        );
        self.tileset = Some(bind);
    }

    fn ensure_character(&mut self, id: &str, sheet: &LoadedSheet) {
        if self.characters.contains_key(id) {
            return;
        }
        let rgba = sheet_to_rgba(sheet);
        let view = upload_rgba(
            &self.device,
            &self.queue,
            id,
            sheet.width,
            sheet.height,
            &rgba,
        );
        let bind = make_bind(
            &self.device,
            &self.bind_group_layout_tex,
            &self.sampler,
            &view,
        );
        self.characters.insert(id.to_string(), bind);
    }

    /// Full frame: world + UI.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    fn draw_tilemap_pass(
        &mut self,
        tilemap: &Tilemap,
        tileset: Option<&LoadedSheet>,
        cam_x: f32,
        cam_y: f32,
        half_w: f32,
        half_h: f32,
        rs: f32,
        white_under: &mut SubBatch,
        tiles: &mut SubBatch,
    ) {
        let world_to_screen_x = |wx: f32| -> f32 { (wx - cam_x) * rs + half_w };
        let world_to_screen_y = |wy: f32| -> f32 { (wy - cam_y) * rs + half_h };
        let ts = tilemap.tile_size;

        if let Some(sheet) = tileset {
            self.ensure_tileset(sheet);
            let tw = sheet.width as f32;
            let th = sheet.height as f32;

            for y in 0..tilemap.height {
                for x in 0..tilemap.width {
                    let wx = x as f32 * ts;
                    let wy = y as f32 * ts;
                    let sx0 = world_to_screen_x(wx);
                    let sy0 = world_to_screen_y(wy);
                    let sx1 = world_to_screen_x(wx + ts);
                    let sy1 = world_to_screen_y(wy + ts);
                    if let Some(td) = &tilemap.tileset_draw {
                        let logical = tilemap.tile_at(x, y).unwrap_or(0);
                        let tile_id = if logical == 0 {
                            td.floor
                        } else if td.wang_autotile {
                            wang_wall_sheet_index(tilemap, x, y)
                        } else {
                            td.wall
                        };
                        let max_id = sheet.cols * sheet.rows;
                        let tid = tile_id.min(max_id.saturating_sub(1));
                        let col = tid % sheet.cols;
                        let row = tid / sheet.cols;
                        let src_x = col * sheet.frame_width;
                        let src_y = row * sheet.frame_height;
                        let u0 = src_x as f32 / tw;
                        let u1 = (src_x + sheet.frame_width) as f32 / tw;
                        let v0 = src_y as f32 / th;
                        let v1 = (src_y + sheet.frame_height) as f32 / th;
                        tiles.push_quad(sx0, sy0, sx1, sy1, u0, v0, u1, v1, [1.0, 1.0, 1.0, 1.0]);
                    } else if tilemap_is_binary_collision_only(tilemap) {
                        let logical = tilemap.tile_at(x, y).unwrap_or(0);
                        let rgb = tilemap.fill_rgb_for_tile(logical);
                        let c = packed_rgb_to_linear(crate::render::to_u32(rgb[0], rgb[1], rgb[2]));
                        white_under.push_quad(sx0, sy0, sx1, sy1, 0.0, 0.0, 1.0, 1.0, c);
                    } else {
                        let logical = tilemap.tile_at(x, y).unwrap_or(0);
                        let max_id = sheet.cols * sheet.rows;
                        let tid = logical.min(max_id.saturating_sub(1));
                        let col = tid % sheet.cols;
                        let row = tid / sheet.cols;
                        let src_x = col * sheet.frame_width;
                        let src_y = row * sheet.frame_height;
                        let u0 = src_x as f32 / tw;
                        let u1 = (src_x + sheet.frame_width) as f32 / tw;
                        let v0 = src_y as f32 / th;
                        let v1 = (src_y + sheet.frame_height) as f32 / th;
                        tiles.push_quad(sx0, sy0, sx1, sy1, u0, v0, u1, v1, [1.0, 1.0, 1.0, 1.0]);
                    }
                }
            }
        } else {
            for y in 0..tilemap.height {
                for x in 0..tilemap.width {
                    let wx = x as f32 * ts;
                    let wy = y as f32 * ts;
                    let sx0 = world_to_screen_x(wx);
                    let sy0 = world_to_screen_y(wy);
                    let sx1 = world_to_screen_x(wx + ts);
                    let sy1 = world_to_screen_y(wy + ts);
                    let logical = tilemap.tile_at(x, y).unwrap_or(0);
                    let rgb = tilemap.fill_rgb_for_tile(logical);
                    let c = packed_rgb_to_linear(crate::render::to_u32(rgb[0], rgb[1], rgb[2]));
                    white_under.push_quad(sx0, sy0, sx1, sy1, 0.0, 0.0, 1.0, 1.0, c);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_entities_pass(
        &mut self,
        world: &World,
        asset_store: &mut AssetStore,
        cam_x: f32,
        cam_y: f32,
        half_w: f32,
        half_h: f32,
        rs: f32,
        chars: &mut BTreeMap<String, SubBatch>,
        white_over: &mut SubBatch,
    ) {
        let world_to_screen_x = |wx: f32| -> f32 { (wx - cam_x) * rs + half_w };
        let world_to_screen_y = |wy: f32| -> f32 { (wy - cam_y) * rs + half_h };

        for (_, (transform, sprite, sprite_sheet, facing, anim_state)) in world
            .query::<(
                &Transform,
                &Sprite,
                Option<&SpriteSheet>,
                Option<&Facing>,
                Option<&AnimationState>,
            )>()
            .iter()
        {
            let row = facing.map(|f| facing_sprite_row(f.0)).unwrap_or(0);
            let (anim_kind, frame_index, walk_bob) = anim_state
                .map(|a| {
                    (
                        a.kind,
                        a.frame_index,
                        matches!(a.kind, AnimationKind::Walk) && (a.frame_index % 2 == 1),
                    )
                })
                .unwrap_or((AnimationKind::Idle, 0, false));

            let (size_w, size_h, char_draw) =
                if let Some(ss) = sprite_sheet {
                    if let Some(sheet) = asset_store.get_sheet(&ss.character_id) {
                        self.ensure_character(&ss.character_id, sheet);
                        let cols = sheet.cols;
                        let c = match anim_kind {
                            AnimationKind::Idle => 0,
                            AnimationKind::Walk => {
                                let walk_cols = (cols - 1).max(1);
                                (1 + (frame_index % walk_cols)).min(cols.saturating_sub(1))
                            }
                        };
                        (
                            sheet.frame_width as f32 * transform.scale.x,
                            sheet.frame_height as f32 * transform.scale.y,
                            Some((ss.character_id.clone(), sheet, row.min(sheet.rows.saturating_sub(1)), c)),
                        )
                    } else {
                        (16.0 * transform.scale.x, 16.0 * transform.scale.y, None)
                    }
                } else {
                    (16.0 * transform.scale.x, 16.0 * transform.scale.y, None)
                };

            let bob_off = if walk_bob { -1.0 } else { 0.0 };
            let wx = transform.position.x - size_w * 0.5;
            let wy = transform.position.y - size_h * 0.5 + bob_off;
            let sx0 = world_to_screen_x(wx);
            let sy0 = world_to_screen_y(wy);
            let sx1 = world_to_screen_x(wx + size_w);
            let sy1 = world_to_screen_y(wy + size_h);

            if let Some((cid, sheet, r, col)) = char_draw {
                let src_x = col * sheet.frame_width;
                let src_y = r * sheet.frame_height;
                let tw = sheet.width as f32;
                let th = sheet.height as f32;
                let u0 = src_x as f32 / tw;
                let u1 = (src_x + sheet.frame_width) as f32 / tw;
                let v0 = src_y as f32 / th;
                let v1 = (src_y + sheet.frame_height) as f32 / th;
                chars
                    .entry(cid)
                    .or_default()
                    .push_quad(sx0, sy0, sx1, sy1, u0, v0, u1, v1, [1.0, 1.0, 1.0, 1.0]);
            } else {
                let c = sprite_color_to_linear(sprite.color);
                white_over.push_quad(
                    sx0,
                    sy0,
                    sx1,
                    sy1,
                    0.0,
                    0.0,
                    1.0,
                    1.0,
                    c,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_ui_pass(
        theme: &UiTheme,
        dialogue_message: Option<&str>,
        backpack_open: bool,
        panel_lines: Option<&crate::ui::BackpackPanelLines>,
        w: u32,
        h: u32,
        ui_scale: u32,
        white_over: &mut SubBatch,
        font: &mut SubBatch,
    ) {
        let ui_s = ui_scale.max(1) as f32;
        let us = ui_scale.max(1) as i32;

        if let Some(msg) = dialogue_message {
            let (left, top, right, bottom) = layout::dialogue_box_rect(w, h, theme, us);
            let fill = packed_rgb_to_linear(render::to_u32(
                theme.dialogue_panel_fill[0],
                theme.dialogue_panel_fill[1],
                theme.dialogue_panel_fill[2],
            ));
            let border = packed_rgb_to_linear(render::to_u32(
                theme.dialogue_panel_border[0],
                theme.dialogue_panel_border[1],
                theme.dialogue_panel_border[2],
            ));
            let text_c = packed_rgb_to_linear(render::to_u32(
                theme.dialogue_text[0],
                theme.dialogue_text[1],
                theme.dialogue_text[2],
            ));
            white_over.push_quad(
                left as f32,
                top as f32,
                right as f32,
                bottom as f32,
                0.0,
                0.0,
                1.0,
                1.0,
                fill,
            );
            let border_px = theme.dialogue_border_top_px * us;
            if border_px > 0 {
                white_over.push_quad(
                    left as f32,
                    top as f32,
                    right as f32,
                    (top + border_px) as f32,
                    0.0,
                    0.0,
                    1.0,
                    1.0,
                    border,
                );
            }
            let (tx, ty) = layout::dialogue_text_pos(w, h, top, theme, us);
            draw_text_sub(font, tx as f32, ty as f32, msg, text_c, ui_s);
        }

        if backpack_open {
            if let Some(panel) = panel_lines {
                let us = ui_scale.max(1) as i32;
                let (left, top, right, bottom) = layout::backpack_panel_rect(w, h, theme, us);
                let fill = packed_rgb_to_linear(render::to_u32(
                    theme.backpack_panel_fill[0],
                    theme.backpack_panel_fill[1],
                    theme.backpack_panel_fill[2],
                ));
                let border = packed_rgb_to_linear(render::to_u32(
                    theme.backpack_panel_border[0],
                    theme.backpack_panel_border[1],
                    theme.backpack_panel_border[2],
                ));
                let empty_c = packed_rgb_to_linear(render::to_u32(
                    theme.backpack_slot_empty[0],
                    theme.backpack_slot_empty[1],
                    theme.backpack_slot_empty[2],
                ));
                let section_usable_c = packed_rgb_to_linear(render::to_u32(
                    theme.backpack_section_usable[0],
                    theme.backpack_section_usable[1],
                    theme.backpack_section_usable[2],
                ));
                let section_weapon_c = packed_rgb_to_linear(render::to_u32(
                    theme.backpack_section_weapon[0],
                    theme.backpack_section_weapon[1],
                    theme.backpack_section_weapon[2],
                ));
                let section_passive_c = packed_rgb_to_linear(render::to_u32(
                    theme.backpack_section_passive[0],
                    theme.backpack_section_passive[1],
                    theme.backpack_section_passive[2],
                ));
                let row_weapon_c = packed_rgb_to_linear(render::to_u32(
                    theme.backpack_row_weapon[0],
                    theme.backpack_row_weapon[1],
                    theme.backpack_row_weapon[2],
                ));
                let row_passive_c = packed_rgb_to_linear(render::to_u32(
                    theme.backpack_row_passive[0],
                    theme.backpack_row_passive[1],
                    theme.backpack_row_passive[2],
                ));
                let row_equipped_c = packed_rgb_to_linear(render::to_u32(
                    theme.backpack_row_equipped[0],
                    theme.backpack_row_equipped[1],
                    theme.backpack_row_equipped[2],
                ));
                white_over.push_quad(
                    left as f32,
                    top as f32,
                    right as f32,
                    bottom as f32,
                    0.0,
                    0.0,
                    1.0,
                    1.0,
                    fill,
                );
                let bt = theme.backpack_border_top_px * us;
                if bt > 0 {
                    white_over.push_quad(
                        left as f32,
                        top as f32,
                        right as f32,
                        (top + bt) as f32,
                        0.0,
                        0.0,
                        1.0,
                        1.0,
                        border,
                    );
                }
                let cx = layout::backpack_content_x(left, theme, us) as f32;
                let indent = layout::backpack_slot_indent(us) as f32;
                let u_ty = layout::backpack_usable_title_y(top, theme, us) as f32;
                draw_text_sub(font, cx, u_ty, "Usable", section_usable_c, ui_s);
                let max_usable = layout::BACKPACK_MAX_USABLE_SLOTS;
                let usable_count = panel.usable.len().min(max_usable);
                for i in 0..max_usable {
                    let slot_y = layout::backpack_usable_slot_y(top, theme, i, us) as f32;
                    let label = panel.usable.get(i).map(|s| s.as_str()).unwrap_or("—");
                    let c = if i < usable_count {
                        section_usable_c
                    } else {
                        empty_c
                    };
                    draw_text_sub(font, cx + indent, slot_y, label, c, ui_s);
                }
                let w_ty = layout::backpack_weapon_title_y(top, theme, us) as f32;
                draw_text_sub(font, cx, w_ty, "Weapons", section_weapon_c, ui_s);
                let max_weapon = layout::BACKPACK_MAX_WEAPON_SLOTS;
                let weapon_count = panel.weapons.len().min(max_weapon);
                for i in 0..max_weapon {
                    let slot_y = layout::backpack_weapon_slot_y(top, theme, i, us) as f32;
                    let (label, equipped) = panel
                        .weapons
                        .get(i)
                        .map(|(s, b)| (s.as_str(), *b))
                        .unwrap_or(("—", false));
                    let c = if i < weapon_count {
                        if equipped {
                            row_equipped_c
                        } else {
                            row_weapon_c
                        }
                    } else {
                        empty_c
                    };
                    draw_text_sub(font, cx + indent, slot_y, label, c, ui_s);
                }
                let p_ty = layout::backpack_passive_title_y(top, theme, us) as f32;
                draw_text_sub(font, cx, p_ty, "Passives", section_passive_c, ui_s);
                let max_passive = layout::BACKPACK_MAX_PASSIVE_SLOTS;
                let passive_count = panel.passives.len().min(max_passive);
                for i in 0..max_passive {
                    let slot_y = layout::backpack_passive_slot_y(top, theme, i, us) as f32;
                    let label = panel.passives.get(i).map(|s| s.as_str()).unwrap_or("—");
                    let c = if i < passive_count {
                        row_passive_c
                    } else {
                        empty_c
                    };
                    draw_text_sub(font, cx + indent, slot_y, label, c, ui_s);
                }
            }
        }
    }

    fn draw_debug_pass(fps_overlay: Option<f32>, ui_scale: u32, font: &mut SubBatch) {
        if let Some(fps) = fps_overlay {
            let label = format!("FPS:{fps:.0}");
            let fg = packed_rgb_to_linear(render::to_u32(0.95, 0.9, 0.35));
            let off = (6 * ui_scale.max(1)) as f32;
            let ui_s = ui_scale.max(1) as f32;
            draw_text_sub(font, off, off, &label, fg, ui_s);
        }
    }

    pub fn draw_frame(
        &mut self,
        tilemap: &Tilemap,
        tileset: Option<&LoadedSheet>,
        world: &World,
        dialogue_message: Option<&str>,
        backpack_open: bool,
        _skill_registry: &SkillRegistry,
        asset_store: &mut AssetStore,
        theme: &UiTheme,
        fps_overlay: Option<f32>,
        render_scale: RenderScale,
        ui_scale: u32,
        panel_lines: Option<&crate::ui::BackpackPanelLines>,
    ) -> Result<(), String> {
        if let Some(s) = tileset {
            self.ensure_tileset(s);
        }

        let sw = self.screen_w as f32;
        let sh = self.screen_h as f32;
        let rs = render_scale.0.max(0.001);

        let cam_x = world
            .query::<(&Player, &Transform)>()
            .iter()
            .next()
            .map(|(_, (_, t))| t.position.x)
            .unwrap_or(0.0);
        let cam_y = world
            .query::<(&Player, &Transform)>()
            .iter()
            .next()
            .map(|(_, (_, t))| t.position.y)
            .unwrap_or(0.0);
        let half_w = sw * 0.5;
        let half_h = sh * 0.5;

        let bg = packed_rgb_to_linear(crate::render::to_u32(0.15, 0.12, 0.18));

        // Solid clear + solid-color tiles (below textured layer).
        let mut white_under = SubBatch::default();
        let mut tiles = SubBatch::default();
        let mut chars: BTreeMap<String, SubBatch> = BTreeMap::new();
        // Colored rects without sheet + UI panels (above world layers).
        let mut white_over = SubBatch::default();
        let mut font = SubBatch::default();

        white_under.push_quad(0.0, 0.0, sw, sh, 0.0, 0.0, 1.0, 1.0, bg);

        self.draw_tilemap_pass(
            tilemap,
            tileset,
            cam_x,
            cam_y,
            half_w,
            half_h,
            rs,
            &mut white_under,
            &mut tiles,
        );

        self.draw_entities_pass(
            world,
            asset_store,
            cam_x,
            cam_y,
            half_w,
            half_h,
            rs,
            &mut chars,
            &mut white_over,
        );

        Self::draw_ui_pass(
            theme,
            dialogue_message,
            backpack_open,
            panel_lines,
            self.screen_w,
            self.screen_h,
            ui_scale,
            &mut white_over,
            &mut font,
        );

        Self::draw_debug_pass(fps_overlay, ui_scale, &mut font);

        // --- Upload & draw ---
        let surface_texture = self
            .surface
            .get_current_texture()
            .map_err(|e| e.to_string())?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            // Concatenate batches into one buffer with offsets — or separate buffers per batch
            fn upload_batch(
                device: &wgpu::Device,
                label: &str,
                batch: &SubBatch,
            ) -> Option<(wgpu::Buffer, wgpu::Buffer, u32)> {
                if batch.is_empty() {
                    return None;
                }
                let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytemuck::cast_slice(&batch.verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{label}_idx")),
                    contents: bytemuck::cast_slice(&batch.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
                Some((vb, ib, batch.indices.len() as u32))
            }

            if let Some((vb, ib, n)) = upload_batch(&self.device, "white_under", &white_under) {
                pass.set_bind_group(1, &self.white.bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..n, 0, 0..1);
            }

            if let (Some(ts), Some((vb, ib, n))) = (&self.tileset, upload_batch(&self.device, "tiles", &tiles)) {
                pass.set_bind_group(1, &ts.bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..n, 0, 0..1);
            }

            for (id, batch) in chars.iter() {
                if batch.is_empty() {
                    continue;
                }
                let Some(bg) = self.characters.get(id) else { continue };
                if let Some((vb, ib, n)) = upload_batch(&self.device, "char", batch) {
                    pass.set_bind_group(1, &bg.bind_group, &[]);
                    pass.set_vertex_buffer(0, vb.slice(..));
                    pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..n, 0, 0..1);
                }
            }

            if let Some((vb, ib, n)) = upload_batch(&self.device, "white_over", &white_over) {
                pass.set_bind_group(1, &self.white.bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..n, 0, 0..1);
            }

            if let Some((vb, ib, n)) = upload_batch(&self.device, "font", &font) {
                pass.set_bind_group(1, &self.font.bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..n, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        Ok(())
    }
}
