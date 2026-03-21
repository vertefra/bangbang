//! sRGB ↔ linear: the CPU renderer wrote **display** 8-bit RGB into a buffer; the GPU uses an
//! **sRGB swapchain**, which expects **linear** fragment output. Without converting, solid colors and
//! unorm texture sampling look slightly dark or shifted versus the old path.

/// sRGB-encoded byte (0–255) → **linear** 0–1 for blending / framebuffer output.
#[inline]
pub fn srgb8_to_linear(c: u8) -> f32 {
    let x = c as f32 * (1.0 / 255.0);
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

/// Packed `0x00RRGGBB` using the same convention as `software::to_u32` → linear RGBA for the GPU.
#[inline]
pub fn packed_rgb_to_linear(p: u32) -> [f32; 4] {
    let r = ((p >> 16) & 0xff) as u8;
    let g = ((p >> 8) & 0xff) as u8;
    let b = (p & 0xff) as u8;
    let a = if p == 0 { 0.0 } else { 1.0 };
    [
        srgb8_to_linear(r),
        srgb8_to_linear(g),
        srgb8_to_linear(b),
        a,
    ]
}

/// ECS `Sprite.color` uses 0–1 floats the same way the CPU path fed `to_u32` (perceptual → byte).
#[inline]
pub fn sprite_color_to_linear(c: [f32; 4]) -> [f32; 4] {
    let r = (c[0].clamp(0.0, 1.0) * 255.0) as u8;
    let g = (c[1].clamp(0.0, 1.0) * 255.0) as u8;
    let b = (c[2].clamp(0.0, 1.0) * 255.0) as u8;
    [
        srgb8_to_linear(r),
        srgb8_to_linear(g),
        srgb8_to_linear(b),
        c[3].clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::srgb8_to_linear;

    #[test]
    fn black_and_white_endpoints() {
        assert!((srgb8_to_linear(0) - 0.0).abs() < 1e-6);
        assert!((srgb8_to_linear(255) - 1.0).abs() < 1e-5);
    }
}
