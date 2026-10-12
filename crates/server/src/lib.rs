pub mod gameplay;

pub use gameplay::GameplayPlugin;

use std::path::Path;
use std::time::SystemTime;

use bevy::app::App;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::Fixed;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::netcode::{
    ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication,
    ServerSetupConfig, in_memory_server_addr, new_memory_sockets,
};
use bevy_replicon_renet2::renet2::{ConnectionConfig, RenetServer};
use bevy_replicon_renet2::{RenetChannelsExt, RepliconRenetPlugins};
use evr_data::{ContentError, ContentPack};

pub const TICK_RATE: f32 = 60.0;
pub const PROTOCOL_ID: u64 = 1;
pub const EMBEDDED_CLIENT_ID: u16 = 1;

#[derive(Resource)]
pub struct Content(pub ContentPack);

#[derive(Clone, Copy)]
pub enum TickMode {
    Fixed,
    PerUpdate,
}

pub fn build_server_app(
    content_root: impl AsRef<Path>,
    tick: TickMode,
) -> Result<App, ContentError> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);

    match tick {
        TickMode::Fixed => {
            app.add_plugins(RepliconPlugins);
            app.insert_resource(Time::<Fixed>::from_hz(f64::from(TICK_RATE)));
        }
        TickMode::PerUpdate => {
            app.add_plugins(RepliconPlugins.set(ServerPlugin::new(PostUpdate)));
        }
    }
    app.add_plugins(RepliconRenetPlugins);

    evr_protocol::register(&mut app);

    let pack = ContentPack::load_dir(content_root.as_ref())?;
    app.insert_resource(Content(pack));
    app.add_plugins(GameplayPlugin);
    app.finish();
    Ok(app)
}

pub fn attach_memory(app: &mut App) -> NetcodeClientTransport {
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system time must be after the epoch");

    let (server_socket, mut client_sockets) =
        new_memory_sockets(vec![EMBEDDED_CLIENT_ID], false, false);

    let (server_channels, client_channels) = {
        let channels = app.world().resource::<RepliconChannels>();
        (channels.server_configs(), channels.client_configs())
    };

    let server = RenetServer::new(ConnectionConfig::from_channels(
        server_channels,
        client_channels,
    ));
    let server_transport = NetcodeServerTransport::new(
        ServerSetupConfig {
            current_time,
            max_clients: 1,
            protocol_id: PROTOCOL_ID,
            socket_addresses: vec![vec![in_memory_server_addr()]],
            authentication: ServerAuthentication::Unsecure,
        },
        server_socket,
    )
    .expect("memory server transport must be valid");
    app.insert_resource(server);
    app.insert_resource(server_transport);

    let socket = client_sockets.pop().expect("one client socket must exist");
    NetcodeClientTransport::new(
        current_time,
        ClientAuthentication::Unsecure {
            client_id: EMBEDDED_CLIENT_ID as u64,
            protocol_id: PROTOCOL_ID,
            socket_id: 0,
            server_addr: in_memory_server_addr(),
            user_data: None,
        },
        socket,
    )
    .expect("memory client transport must be valid")
}
