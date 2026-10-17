use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SystemDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub government: String,
    pub position: (f32, f32),
    pub links: Vec<String>,
    pub planets: Vec<PlanetDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlanetDef {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub sprite: Option<String>,
    #[serde(default)]
    pub chief: Option<ChiefDef>,
    pub commodities: Vec<CommodityPrice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChiefDef {
    pub name: String,
    #[serde(default)]
    pub portrait: Option<String>,
    pub greeting: String,
    pub offer_text: String,
    pub tribute_text: String,
    pub quest_text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommodityPrice {
    pub id: String,
    pub price: u32,
}
