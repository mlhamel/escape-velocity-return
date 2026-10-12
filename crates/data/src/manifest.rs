use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: (u32, u32, u32),
    pub ships: Vec<String>,
    pub systems: Vec<String>,
}
