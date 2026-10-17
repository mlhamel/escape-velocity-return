use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::error::{ContentError, validate_asset, validate_ids, validate_reference};
use crate::manifest::Manifest;
use crate::outfit::OutfitDef;
use crate::quest::{QuestDef, QuestKind};
use crate::ship::ShipDef;
use crate::system::SystemDef;

#[derive(Debug)]
pub struct ContentPack {
    pub manifest: Manifest,
    pub ships: Vec<ShipDef>,
    pub systems: Vec<SystemDef>,
    pub outfits: Vec<OutfitDef>,
    pub quests: Vec<QuestDef>,
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

        let mut outfits: Vec<OutfitDef> = Vec::with_capacity(manifest.outfits.len());
        for path in &manifest.outfits {
            outfits.push(parse_ron(&root.join(path))?);
        }

        let mut quests: Vec<QuestDef> = Vec::with_capacity(manifest.quests.len());
        for path in &manifest.quests {
            quests.push(parse_ron(&root.join(path))?);
        }

        Self::validate(&ships, &systems, &outfits, &quests)?;
        Self::validate_sprites(root, &ships, &systems)?;

        Ok(Self {
            manifest,
            ships,
            systems,
            outfits,
            quests,
        })
    }

    fn validate_sprites(
        root: &Path,
        ships: &[ShipDef],
        systems: &[SystemDef],
    ) -> Result<(), ContentError> {
        let assets_root = root.parent().unwrap_or(root).join("assets");
        for ship in ships {
            if let Some(sprite) = &ship.sprite {
                validate_asset(&assets_root, sprite)?;
            }
        }
        for system in systems {
            for planet in &system.planets {
                if let Some(sprite) = &planet.sprite {
                    validate_asset(&assets_root, sprite)?;
                }
                if let Some(chief) = &planet.chief
                    && let Some(portrait) = &chief.portrait
                {
                    validate_asset(&assets_root, portrait)?;
                }
            }
        }
        Ok(())
    }

    pub fn ship(&self, id: &str) -> Option<&ShipDef> {
        self.ships.iter().find(|ship| ship.id == id)
    }

    pub fn system(&self, id: &str) -> Option<&SystemDef> {
        self.systems.iter().find(|system| system.id == id)
    }

    pub fn outfit(&self, id: &str) -> Option<&OutfitDef> {
        self.outfits.iter().find(|outfit| outfit.id == id)
    }

    pub fn quest(&self, id: &str) -> Option<&QuestDef> {
        self.quests.iter().find(|quest| quest.id == id)
    }

    fn validate(
        ships: &[ShipDef],
        systems: &[SystemDef],
        outfits: &[OutfitDef],
        quests: &[QuestDef],
    ) -> Result<(), ContentError> {
        validate_ids("ship", ships.iter().map(|ship| ship.id.clone()))?;
        validate_ids("system", systems.iter().map(|system| system.id.clone()))?;
        validate_ids("outfit", outfits.iter().map(|outfit| outfit.id.clone()))?;
        validate_ids("quest", quests.iter().map(|quest| quest.id.clone()))?;

        let outfit_ids: std::collections::HashSet<_> =
            outfits.iter().map(|o| o.id.clone()).collect();
        for ship in ships {
            for outfit in &ship.default_outfits {
                validate_reference("outfit", outfit, &outfit_ids)?;
            }
        }

        let system_ids: std::collections::HashSet<_> =
            systems.iter().map(|s| s.id.clone()).collect();
        for system in systems {
            for link in &system.links {
                validate_reference("system link", link, &system_ids)?;
                let target_has_back_link = systems
                    .iter()
                    .find(|target| &target.id == link)
                    .is_some_and(|target| target.links.contains(&system.id));
                if !target_has_back_link {
                    return Err(ContentError::UnknownReference {
                        kind: "reciprocal link",
                        id: format!("{}->{}", system.id, link),
                    });
                }
            }
        }

        let planet_refs: std::collections::HashSet<String> = systems
            .iter()
            .flat_map(|s| s.planets.iter().map(move |p| format!("{}/{}", s.id, p.id)))
            .collect();

        for quest in quests {
            let giver_ref = format!("{}/{}", quest.giver.0, quest.giver.1);
            validate_reference("quest giver planet", &giver_ref, &planet_refs)?;
            match &quest.kind {
                QuestKind::Delivery { destination, .. } => {
                    let dest_ref = format!("{}/{}", destination.0, destination.1);
                    validate_reference("quest destination planet", &dest_ref, &planet_refs)?;
                }
                QuestKind::Bounty { system, .. } => {
                    validate_reference("quest target system", system, &system_ids)?;
                }
            }
        }

        Ok(())
    }
}

fn parse_ron<T: DeserializeOwned>(path: &PathBuf) -> Result<T, ContentError> {
    let text = fs::read_to_string(path)?;
    ron::from_str(&text).map_err(|err| ContentError::Parse {
        path: path.clone(),
        message: err.to_string(),
    })
}
