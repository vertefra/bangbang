//! wgpu 2D textured-quad renderer: tilemap, sprites, UI panels, vector UI text (`fontdue` + atlas).

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::assets::{skill_image_key, AssetStore, LoadedSheet};
use crate::gpu::frame_context::{FrameContext, RenderScales, UiFrameState};
use crate::gpu::pass_common::{GpuVertex, PassFrameParams, SubBatch};
use crate::gpu::pass_debug::draw_debug_pass;
use crate::gpu::pass_entities::{build_dialogue_portrait_batch, draw_entities_pass, EntityDrawChunk};
use crate::gpu::pass_tilemap::draw_tilemap_pass;
use crate::gpu::pass_ui::draw_ui_pass;
use crate::gpu::text_atlas::{TextQuad, UiFontAtlas, UI_FONT_ATLAS_DIM};
use crate::render::color::packed_rgb_to_linear;
use crate::ui::layout;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenUniform {
    screen_size: [f32; 2],
    _pad: [f32; 2],
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
    font_texture: wgpu::Texture,
    ui_font: UiFontAtlas,
    text_quads_scratch: Vec<TextQuad>,
    tileset: Option<TextureBind>,
    /// FNV-1a tag of the last uploaded map tileset pixels; map transitions must re-upload when this changes.
    tileset_sheet_tag: Option<u64>,
    characters: HashMap<String, TextureBind>,
    screen_w: u32,
    screen_h: u32,
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

        let bind_group_layout_tex =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let font_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui_font_linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let white_view = upload_rgba(&device, &queue, "white", 1, 1, &[255, 255, 255, 255]);
        let white = make_bind(
            &device,
            &bind_group_layout_tex,
            &sampler,
            &white_view,
            "white_bind_group",
        );

        let font_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ui_font_atlas"),
            size: wgpu::Extent3d {
                width: UI_FONT_ATLAS_DIM,
                height: UI_FONT_ATLAS_DIM,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let font_view = font_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let font = make_bind(
            &device,
            &bind_group_layout_tex,
            &font_sampler,
            &font_view,
            "ui_font_bind_group",
        );

        let ui_font = UiFontAtlas::new().map_err(|e| e.to_string())?;

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
            font_texture,
            ui_font,
            text_quads_scratch: Vec::new(),
            tileset: None,
            tileset_sheet_tag: None,
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

    pub(crate) fn push_ui_text(
        &mut self,
        font: &mut SubBatch,
        text: &str,
        x: f32,
        y: f32,
        color: [f32; 4],
        text_scale: f32,
        max_width: Option<f32>,
    ) {
        self.ui_font.layout_text_quads(
            &self.queue,
            &self.font_texture,
            text,
            x,
            y,
            text_scale,
            max_width,
            &mut self.text_quads_scratch,
        );
        for q in &self.text_quads_scratch {
            font.push_quad(q.x0, q.y0, q.x1, q.y1, q.u0, q.v0, q.u1, q.v1, color);
        }
    }

    /// Debug HUD: **Noto Sans Bold** via `layout_debug_text_quads` (separate from regular UI).
    pub(crate) fn push_ui_debug_text(
        &mut self,
        font: &mut SubBatch,
        text: &str,
        x: f32,
        y: f32,
        color: [f32; 4],
        text_scale: f32,
        max_width: Option<f32>,
    ) {
        self.ui_font.layout_debug_text_quads(
            &self.queue,
            &self.font_texture,
            text,
            x,
            y,
            text_scale,
            max_width,
            &mut self.text_quads_scratch,
        );
        for q in &self.text_quads_scratch {
            font.push_quad(q.x0, q.y0, q.x1, q.y1, q.u0, q.v0, q.u1, q.v1, color);
        }
    }

    fn loaded_sheet_tag(sheet: &LoadedSheet) -> u64 {
        const OFFSET: u64 = 14695981039346656037;
        const PRIME: u64 = 1099511628211;
        let mut h = OFFSET;
        for &x in &[
            sheet.width,
            sheet.height,
            sheet.frame_width,
            sheet.frame_height,
            sheet.cols,
            sheet.rows,
        ] {
            h ^= x as u64;
            h = h.wrapping_mul(PRIME);
        }
        for &p in &sheet.pixels {
            h ^= p as u64;
            h = h.wrapping_mul(PRIME);
        }
        h
    }

    pub(crate) fn ensure_tileset(&mut self, sheet: &LoadedSheet) {
        let tag = Self::loaded_sheet_tag(sheet);
        if self.tileset_sheet_tag == Some(tag) {
            return;
        }
        self.tileset_sheet_tag = Some(tag);
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
            "tileset_bind_group",
        );
        self.tileset = Some(bind);
    }

    pub(crate) fn ensure_character(&mut self, id: &str, sheet: &LoadedSheet) {
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
            "character_bind_group",
        );
        self.characters.insert(id.to_string(), bind);
    }

    /// Upload skill icon texture (if not yet cached) and queue a quad in `skill_icons`.
    pub(crate) fn push_skill_icon(
        &mut self,
        asset_store: &mut AssetStore,
        skill_id: &str,
        x: f32,
        y: f32,
        size: f32,
        skill_icons: &mut BTreeMap<String, SubBatch>,
    ) {
        let key = skill_image_key(skill_id);
        if !self.characters.contains_key(&key) {
            if let Some(sheet) = asset_store.get_skill_image(skill_id) {
                let rgba = sheet_to_rgba(sheet);
                let view = upload_rgba(
                    &self.device,
                    &self.queue,
                    &key,
                    sheet.width,
                    sheet.height,
                    &rgba,
                );
                let bind = make_bind(
                    &self.device,
                    &self.bind_group_layout_tex,
                    &self.sampler,
                    &view,
                    &key,
                );
                self.characters.insert(key.clone(), bind);
            }
        }
        if self.characters.contains_key(&key) {
            skill_icons.entry(key).or_default().push_quad(
                x,
                y,
                x + size,
                y + size,
                0.0,
                0.0,
                1.0,
                1.0,
                [1.0, 1.0, 1.0, 1.0],
            );
        }
    }

    pub fn draw_frame(&mut self, frame: FrameContext<'_>) -> Result<(), String> {
        let FrameContext {
            tilemap,
            tileset,
            world,
            asset_store,
            theme,
            scales,
            ui,
            debug: debug_overlay,
        } = frame;
        let RenderScales {
            render: render_scale,
            ui: ui_scale,
            font: font_scale,
        } = scales;
        let UiFrameState {
            dialogue_message,
            dialogue_npc_id,
            overworld_toast,
            backpack_open,
            backpack_lines: panel_lines,
        } = ui;

        if let Some(s) = tileset {
            self.ensure_tileset(s);
        }

        let sw = self.screen_w as f32;
        let sh = self.screen_h as f32;
        let rs = render_scale.0.max(0.001);

        let (cam_x, cam_y, player_hp) = world
            .query::<(&crate::ecs::Player, &crate::ecs::Transform, &crate::ecs::Health)>()
            .iter()
            .next()
            .map(|(_, (_, t, h))| {
                let hp = if h.max > 0 {
                    Some((h.current.clamp(0, h.max), h.max))
                } else {
                    None
                };
                (t.position.x, t.position.y, hp)
            })
            .unwrap_or((0.0, 0.0, None));
        let half_w = sw * 0.5;
        let half_h = sh * 0.5;

        let pass_params = PassFrameParams {
            cam_x,
            cam_y,
            half_w,
            half_h,
            rs,
        };

        let bg = packed_rgb_to_linear(crate::render::to_u32(0.15, 0.12, 0.18));

        // Solid clear + solid-color tiles (below textured layer).
        let mut white_under = SubBatch::default();
        let mut tiles = SubBatch::default();
        // Colored rects: UI panels (above world layers). World entity fallbacks are drawn in
        // `entity_chunks` (Y-sorted with textured sprites).
        let mut white_over = SubBatch::default();
        // Skill icon quads drawn after panel backgrounds, before font text.
        let mut skill_icons: BTreeMap<String, SubBatch> = BTreeMap::new();
        let mut font = SubBatch::default();

        let us_i = ui_scale.max(1) as i32;
        let dialogue_portrait = if dialogue_message.is_some() {
            dialogue_npc_id.and_then(|npc| {
                build_dialogue_portrait_batch(
                    self,
                    asset_store,
                    theme,
                    npc,
                    self.screen_w,
                    self.screen_h,
                    us_i,
                )
            })
        } else {
            None
        };
        let dialogue_text_extra_left = if dialogue_portrait.is_some() {
            layout::dialogue_portrait_text_extra_left(theme, us_i)
        } else {
            0
        };

        white_under.push_quad(0.0, 0.0, sw, sh, 0.0, 0.0, 1.0, 1.0, bg);

        draw_tilemap_pass(
            self,
            tilemap,
            tileset,
            pass_params,
            &mut white_under,
            &mut tiles,
        );

        let entity_chunks = draw_entities_pass(self, world, asset_store, pass_params);

        #[cfg(feature = "debug")]
        let entity_debug_borders = crate::gpu::pass_entity_debug::prepare_entity_debug_overlay(
            self,
            world,
            asset_store,
            pass_params,
            &mut font,
            ui_scale,
            font_scale,
        );

        draw_ui_pass(
            self,
            theme,
            player_hp,
            dialogue_message,
            dialogue_text_extra_left,
            overworld_toast,
            backpack_open,
            panel_lines,
            asset_store,
            self.screen_w,
            self.screen_h,
            ui_scale,
            font_scale,
            &mut white_over,
            &mut font,
            &mut skill_icons,
        );

        draw_debug_pass(self, debug_overlay, ui_scale, font_scale, &mut font);

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

            if let (Some(ts), Some((vb, ib, n))) =
                (&self.tileset, upload_batch(&self.device, "tiles", &tiles))
            {
                pass.set_bind_group(1, &ts.bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..n, 0, 0..1);
            }

            for chunk in &entity_chunks {
                match chunk {
                    EntityDrawChunk::Textured {
                        character_id,
                        batch,
                    } => {
                        if batch.is_empty() {
                            continue;
                        }
                        let Some(bg) = self.characters.get(character_id) else {
                            continue;
                        };
                        if let Some((vb, ib, n)) = upload_batch(&self.device, "char", batch) {
                            pass.set_bind_group(1, &bg.bind_group, &[]);
                            pass.set_vertex_buffer(0, vb.slice(..));
                            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                            pass.draw_indexed(0..n, 0, 0..1);
                        }
                    }
                    EntityDrawChunk::Solid { batch } => {
                        if let Some((vb, ib, n)) = upload_batch(&self.device, "entity_solid", batch) {
                            pass.set_bind_group(1, &self.white.bind_group, &[]);
                            pass.set_vertex_buffer(0, vb.slice(..));
                            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                            pass.draw_indexed(0..n, 0, 0..1);
                        }
                    }
                }
            }

            #[cfg(feature = "debug")]
            if let Some((vb, ib, n)) =
                upload_batch(&self.device, "entity_debug", &entity_debug_borders)
            {
                pass.set_bind_group(1, &self.white.bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..n, 0, 0..1);
            }

            if let Some((vb, ib, n)) = upload_batch(&self.device, "white_over", &white_over) {
                pass.set_bind_group(1, &self.white.bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..n, 0, 0..1);
            }

            // Skill icons drawn after panel backgrounds (white_over) but before text.
            for (id, batch) in skill_icons.iter() {
                if batch.is_empty() {
                    continue;
                }
                let Some(bg) = self.characters.get(id) else {
                    continue;
                };
                if let Some((vb, ib, n)) = upload_batch(&self.device, "skill_icon", batch) {
                    pass.set_bind_group(1, &bg.bind_group, &[]);
                    pass.set_vertex_buffer(0, vb.slice(..));
                    pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..n, 0, 0..1);
                }
            }

            if let Some((portrait_id, batch)) = dialogue_portrait.as_ref() {
                if !batch.is_empty() {
                    if let Some(bg) = self.characters.get(portrait_id) {
                        if let Some((vb, ib, n)) =
                            upload_batch(&self.device, "dialogue_portrait", batch)
                        {
                            pass.set_bind_group(1, &bg.bind_group, &[]);
                            pass.set_vertex_buffer(0, vb.slice(..));
                            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                            pass.draw_indexed(0..n, 0, 0..1);
                        }
                    }
                }
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
    label: &str,
) -> TextureBind {
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
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
