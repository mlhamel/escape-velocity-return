use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct OutfitDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub price: u32,
    pub mass: f32,
    pub kind: OutfitKind,
}

#[derive(Debug, Clone, Deserialize)]
pub enum OutfitKind {
    Weapon(WeaponDef),
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponDef {
    pub damage: f32,
    pub cooldown: f32,
    pub projectile_speed: f32,
    pub projectile_ttl: f32,
}
