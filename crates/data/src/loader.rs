use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::error::{ContentError, validate_ids};
use crate::manifest::Manifest;
use crate::ship::ShipDef;
use crate::system::SystemDef;

#[derive(Debug)]
pub struct ContentPack {
    pub manifest: Manifest,
    pub ships: Vec<ShipDef>,
    pub systems: Vec<SystemDef>,
}

impl ContentPack {
    pub fn load_dir(root: &Path) -> Result<Self, ContentError> {
        let manifest: Manifest = parse_ron(&root.join("manifest.ron"))?;

        let mut ships: Vec<ShipDef> = Vec::with_capacity(manifest.ships.len());
        for path in &manifest.ships {
            ships.push(parse_ron(&root.join(path))?);
        }

        let mut systems: Vec<SystemDef> = Vec::with_capacity(manifest.systems.len());
        for path in &manifest.systems {
            systems.push(parse_ron(&root.join(path))?);
        }

        validate_ids("ship", ships.iter().map(|ship| ship.id.clone()))?;
        validate_ids("system", systems.iter().map(|system| system.id.clone()))?;

        Ok(Self {
            manifest,
            ships,
            systems,
        })
    }

    pub fn ship(&self, id: &str) -> Option<&ShipDef> {
        self.ships.iter().find(|ship| ship.id == id)
    }

    pub fn system(&self, id: &str) -> Option<&SystemDef> {
        self.systems.iter().find(|system| system.id == id)
    }
}

fn parse_ron<T: DeserializeOwned>(path: &PathBuf) -> Result<T, ContentError> {
    let text = fs::read_to_string(path)?;
    ron::from_str(&text).map_err(|err| ContentError::Parse {
        path: path.clone(),
        message: err.to_string(),
    })
}
