use std::path::Path;

use bevy::prelude::*;
use evr_data::{ContentError, ContentPack};

pub const TICK_RATE: f32 = 20.0;

#[derive(Resource)]
pub struct Content(pub ContentPack);

pub fn build_app(content_root: impl AsRef<Path>) -> Result<App, ContentError> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    let pack = ContentPack::load_dir(content_root.as_ref())?;
    app.insert_resource(Content(pack));

    app.add_systems(Startup, announce);
    Ok(app)
}

fn announce(content: Res<Content>) {
    let (major, minor, patch) = content.0.manifest.version;
    println!(
        "evr-server: pack {} v{major}.{minor}.{patch} — {} ships, {} systems",
        content.0.manifest.name,
        content.0.ships.len(),
        content.0.systems.len()
    );
}
