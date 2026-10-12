mod hud;
mod input;
mod textures;
mod visuals;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::renet2::{ConnectionConfig, RenetClient};
use bevy_replicon_renet2::{RenetChannelsExt, RepliconRenetPlugins};

struct EmbeddedServer(App);

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(AssetPlugin {
        file_path: "../../assets".to_owned(),
        ..default()
    }));
    app.add_plugins(RepliconPlugins);
    app.add_plugins(RepliconRenetPlugins);
    evr_protocol::register(&mut app);

    app.add_plugins((
        textures::TexturesPlugin,
        visuals::VisualsPlugin,
        input::InputPlugin,
        hud::HudPlugin,
    ));
    app.add_systems(Startup, (spawn_camera, visuals::spawn_starfield));
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
