//! Vector UI font: `fontdue` rasterizes TTF into a dynamic GPU atlas (linear sampling).

use std::collections::HashMap;

use fontdue::layout::{CoordinateSystem, GlyphRasterConfig, Layout, LayoutSettings, TextStyle};
use fontdue::Font;

pub const UI_FONT_ATLAS_DIM: u32 = 1024;
const ATLAS_W: u32 = UI_FONT_ATLAS_DIM;
const ATLAS_H: u32 = UI_FONT_ATLAS_DIM;
const FONT_TTF: &[u8] = include_bytes!("../../assets/fonts/NotoSans-Regular.ttf");
const FONT_BOLD_TTF: &[u8] = include_bytes!("../../assets/fonts/NotoSans-Bold.ttf");

#[derive(Clone, Copy)]
struct CachedGlyph {
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
}

/// One textured quad in screen space (pixel coords, top-left origin).
pub struct TextQuad {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

pub struct UiFontAtlas {
    font: Font,
    font_bold: Font,
    layout: Layout,
    data: Vec<u8>,
    pen_x: u32,
    pen_y: u32,
    row_h: u32,
    cache: HashMap<(usize, GlyphRasterConfig), CachedGlyph>,
}

impl UiFontAtlas {
    pub fn new() -> Result<Self, &'static str> {
        let font = Font::from_bytes(FONT_TTF, fontdue::FontSettings::default())?;
        let font_bold = Font::from_bytes(FONT_BOLD_TTF, fontdue::FontSettings::default())?;
        let n = (ATLAS_W * ATLAS_H * 4) as usize;
        Ok(Self {
            font,
            font_bold,
            layout: Layout::new(CoordinateSystem::PositiveYDown),
            data: vec![0u8; n],
            pen_x: 0,
            pen_y: 0,
            row_h: 0,
            cache: HashMap::new(),
        })
    }

    fn reset_atlas(&mut self) {
        self.data.fill(0);
        self.pen_x = 0;
        self.pen_y = 0;
        self.row_h = 0;
        self.cache.clear();
        log::warn!("ui font glyph atlas cleared (out of space)");
    }

    /// Allocate `w` x `h` pixels plus a 1px gutter. Returns top-left in atlas, or `None` if full.
    fn alloc(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if w == 0 || h == 0 {
            return Some((0, 0));
        }
        let gw = w + 1;
        let gh = h + 1;
        if gw > ATLAS_W || gh > ATLAS_H {
            return None;
        }
        if self.pen_x + gw > ATLAS_W {
            self.pen_x = 0;
            self.pen_y += self.row_h;
            self.row_h = 0;
        }
        if self.pen_y + gh > ATLAS_H {
            return None;
        }
        let ox = self.pen_x;
        let oy = self.pen_y;
        self.pen_x += gw;
        self.row_h = self.row_h.max(gh);
        Some((ox, oy))
    }

    fn alloc_or_reset(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        match self.alloc(w, h) {
            Some(p) => Some(p),
            None => {
                self.reset_atlas();
                let p = self.alloc(w, h);
                if p.is_none() {
                    log::warn!(
                        "ui font glyph {}x{} does not fit in {}x{} atlas",
                        w,
                        h,
                        ATLAS_W,
                        ATLAS_H
                    );
                }
                p
            }
        }
    }

    fn blit_coverage(&mut self, ox: u32, oy: u32, gw: u32, gh: u32, bmp: &[u8]) {
        for row in 0..gh {
            for col in 0..gw {
                let v = bmp[(row * gw + col) as usize];
                let px = ox + col;
                let py = oy + row;
                let i = ((py * ATLAS_W + px) * 4) as usize;
                self.data[i..i + 4].copy_from_slice(&[v, v, v, v]);
            }
        }
    }

    fn upload_rect(
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        ox: u32,
        oy: u32,
        gw: u32,
        gh: u32,
        src: &[u8],
    ) {
        if gw == 0 || gh == 0 {
            return;
        }
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: ox, y: oy, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            src,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * gw),
                rows_per_image: Some(gh),
            },
            wgpu::Extent3d {
                width: gw,
                height: gh,
                depth_or_array_layers: 1,
            },
        );
    }

    fn ensure_glyph(
        &mut self,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        key: GlyphRasterConfig,
        font_index: usize,
    ) -> Option<CachedGlyph> {
        let cache_key = (font_index, key);
        if let Some(g) = self.cache.get(&cache_key) {
            return Some(*g);
        }

        let (m, bmp) = if font_index == 1 {
            self.font_bold.rasterize_indexed(key.glyph_index, key.px)
        } else {
            self.font.rasterize_indexed(key.glyph_index, key.px)
        };
        let gw = m.width as u32;
        let gh = m.height as u32;
        if gw == 0 || gh == 0 {
            return None;
        }

        let (ox, oy) = self.alloc_or_reset(gw, gh)?;
        self.blit_coverage(ox, oy, gw, gh, &bmp);

        let mut upload = Vec::with_capacity((gw * gh * 4) as usize);
        for row in 0..gh {
            let src_i = (((oy + row) * ATLAS_W + ox) * 4) as usize;
            let row_len = (gw * 4) as usize;
            upload.extend_from_slice(&self.data[src_i..src_i + row_len]);
        }
        Self::upload_rect(queue, texture, ox, oy, gw, gh, &upload);

        let aw = ATLAS_W as f32;
        let ah = ATLAS_H as f32;
        let g = CachedGlyph {
            u0: ox as f32 / aw,
            v0: oy as f32 / ah,
            u1: (ox + gw) as f32 / aw,
            v1: (oy + gh) as f32 / ah,
        };
        self.cache.insert(cache_key, g);
        Some(g)
    }

    /// `text_scale` is `ui_scale * font_scale` (same knob as the old bitmap font). Wraps when `max_width` is set.
    pub fn layout_text_quads(
        &mut self,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        text: &str,
        x: f32,
        y: f32,
        text_scale: f32,
        max_width: Option<f32>,
        out: &mut Vec<TextQuad>,
    ) {
        out.clear();
        if text.is_empty() {
            return;
        }

        let px = (text_scale * 8.0).clamp(10.0, 56.0);
        self.layout.reset(&LayoutSettings {
            x,
            y,
            max_width,
            ..LayoutSettings::default()
        });
        self.layout
            .append(&[&self.font], &TextStyle::new(text, px, 0));

        let glyphs: Vec<_> = self.layout.glyphs().to_vec();
        for g in glyphs {
            if g.width == 0 || g.height == 0 {
                continue;
            }
            let Some(uv) = self.ensure_glyph(queue, texture, g.key, 0) else {
                continue;
            };
            let x0 = g.x;
            let y0 = g.y;
            let x1 = x0 + g.width as f32;
            let y1 = y0 + g.height as f32;
            out.push(TextQuad {
                x0,
                y0,
                x1,
                y1,
                u0: uv.u0,
                v0: uv.v0,
                u1: uv.u1,
                v1: uv.v1,
            });
        }
    }

    /// Same as [`layout_text_quads`](Self::layout_text_quads) but uses **Noto Sans Bold** (`font_index` 1).
    pub fn layout_debug_text_quads(
        &mut self,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        text: &str,
        x: f32,
        y: f32,
        text_scale: f32,
        max_width: Option<f32>,
        out: &mut Vec<TextQuad>,
    ) {
        out.clear();
        if text.is_empty() {
            return;
        }

        let px = (text_scale * 8.0).clamp(10.0, 56.0);
        self.layout.reset(&LayoutSettings {
            x,
            y,
            max_width,
            ..LayoutSettings::default()
        });
        self.layout
            .append(&[&self.font, &self.font_bold], &TextStyle::new(text, px, 1));

        let glyphs: Vec<_> = self.layout.glyphs().to_vec();
        for g in glyphs {
            if g.width == 0 || g.height == 0 {
                continue;
            }
            let Some(uv) = self.ensure_glyph(queue, texture, g.key, g.font_index) else {
                continue;
            };
            let x0 = g.x;
            let y0 = g.y;
            let x1 = x0 + g.width as f32;
            let y1 = y0 + g.height as f32;
            out.push(TextQuad {
                x0,
                y0,
                x1,
                y1,
                u0: uv.u0,
                v0: uv.v0,
                u1: uv.u1,
                v1: uv.v1,
            });
        }
    }
}
