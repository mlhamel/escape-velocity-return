use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ShipDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub price: u32,
    pub mass: f32,
    pub turn_rate: f32,
    pub acceleration: f32,
    pub max_speed: f32,
    pub max_shields: f32,
    pub max_armor: f32,
    pub fuel: u32,
    pub cargo: u32,
    pub weapon_slots: u32,
    #[serde(default)]
    pub default_outfits: Vec<String>,
    #[serde(default)]
    pub sprite: Option<String>,
}
