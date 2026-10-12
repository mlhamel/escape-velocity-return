use bevy::app::Startup;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, announce)
        .run();
}

fn announce() {
    println!("evr-client: Escape Velocity — The Return");
}
