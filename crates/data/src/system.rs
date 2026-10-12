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
    pub commodities: Vec<CommodityPrice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommodityPrice {
    pub id: String,
    pub price: u32,
}
