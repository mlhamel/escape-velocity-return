mod hud;
mod input;
mod screenshots;
mod textures;
mod tour;
mod visuals;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::window::WindowPlugin;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::renet2::{ConnectionConfig, RenetClient};
use bevy_replicon_renet2::{RenetChannelsExt, RepliconRenetPlugins};

struct EmbeddedServer(App);

fn main() {
    let capture_tour = std::env::args().any(|arg| arg == "--capture-tour");

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: "../../assets".to_owned(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Escape Velocity: The Return".to_owned(),
                    resolution: bevy::window::WindowResolution::new(1280, 720),
                    ..default()
                }),
                ..default()
            }),
    );
    app.add_plugins(RepliconPlugins);
    app.add_plugins(RepliconRenetPlugins);
    evr_protocol::register(&mut app);

    app.add_plugins((
        textures::TexturesPlugin,
        visuals::VisualsPlugin,
        input::InputPlugin,
        hud::HudPlugin,
        screenshots::ScreenshotPlugin,
    ));
    app.add_systems(Startup, (spawn_camera, visuals::spawn_starfield));
    if capture_tour {
        app.add_plugins(tour::CaptureTourPlugin);
    }
    app.add_systems(First, step_embedded_server);

    let mut server_app = evr_server::build_server_app("content", evr_server::TickMode::Fixed)
        .expect("content pack must load");
    let transport = evr_server::attach_memory(&mut server_app);

    let (server_channels, client_channels) = {
        let channels = app.world().resource::<RepliconChannels>();
        (channels.server_configs(), channels.client_configs())
    };
    let client = RenetClient::new(
        ConnectionConfig::from_channels(server_channels, client_channels),
        false,
    );
    app.insert_resource(client);
    app.insert_resource(transport);
    app.insert_non_send(EmbeddedServer(server_app));

    app.run();
}

fn step_embedded_server(mut server: NonSendMut<EmbeddedServer>) {
    server.0.update();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
