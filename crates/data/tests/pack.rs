use std::fs;
use std::path::Path;

use evr_data::{ContentError, ContentPack};

const DUPLICATE_SHIP: &str = r#"( id: "dupe", name: "Dupe", description: "", price: 1, mass: 1.0, turn_rate: 1.0, acceleration: 1.0, max_speed: 1.0, max_shields: 1.0, max_armor: 1.0, fuel: 1, cargo: 1, weapon_slots: 1 )"#;

#[test]
fn base_pack_loads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let pack = ContentPack::load_dir(&root).expect("base pack must load");

    assert_eq!(pack.manifest.id, "evr-base");

    let shuttle = pack.ship("shuttle").expect("shuttle exists");
    assert_eq!(shuttle.name, "Shuttle");
    assert!(shuttle.weapon_slots >= 1);

    let sol = pack.system("sol").expect("sol exists");
    assert_eq!(sol.government, "confederation");
    assert_eq!(sol.planets.len(), 1);
    assert_eq!(sol.planets[0].commodities.len(), 3);
}

#[test]
fn duplicate_ids_are_rejected() {
    let dir = std::env::temp_dir().join(format!("evr-data-test-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir must be creatable");
    fs::write(
        dir.join("manifest.ron"),
        r#"( id: "t", name: "t", version: (0, 1, 0), ships: ["a.ron", "b.ron"], systems: [] )"#,
    )
    .expect("manifest must be writable");
    fs::write(dir.join("a.ron"), DUPLICATE_SHIP).expect("ship must be writable");
    fs::write(dir.join("b.ron"), DUPLICATE_SHIP).expect("ship must be writable");

    let result = ContentPack::load_dir(&dir);
    assert!(
        matches!(result, Err(ContentError::DuplicateId { .. })),
        "expected DuplicateId, got {result:?}"
    );

    fs::remove_dir_all(&dir).expect("temp dir must be removable");
}
