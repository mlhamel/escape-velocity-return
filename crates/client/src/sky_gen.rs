use bevy::color::Color;
use bevy::math::Vec2;

pub fn sky_clear_color(system_id: &str) -> Color {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in system_id.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    let hue = 0.58 + ((hash % 20) as f32) / 100.0;
    let saturation = 0.25 + (((hash >> 12) % 20) as f32) / 100.0;
    let lightness = 0.02 + (((hash >> 24) % 6) as f32) / 100.0;
    Color::hsl(hue, saturation, lightness)
}

pub struct NebulaAccent {
    pub position: Vec2,
    pub size: f32,
    pub color: Color,
}

pub fn nebula_accents(system_id: &str, count: usize) -> Vec<NebulaAccent> {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in system_id.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash ^= 0x5EED_0000_0000_0001;

    let mut accents = Vec::with_capacity(count);
    for i in 0..count {
        hash = hash.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        hash ^= hash >> 31;

        let x = ((hash % 2000) as f32) - 1000.0;
        let y = (((hash >> 16) % 1200) as f32) - 600.0;
        let size = 200.0 + (((hash >> 32) % 400) as f32);
        let hue = 0.55 + (((hash >> 8) % 30) as f32) / 100.0;
        let saturation = 0.35 + (((hash >> 20) % 25) as f32) / 100.0;
        let lightness = 0.04 + (((hash >> 28) % 10) as f32) / 100.0;

        accents.push(NebulaAccent {
            position: Vec2::new(x, y),
            size,
            color: Color::hsl(hue, saturation, lightness),
        });
        let _ = i;
    }
    accents
}
