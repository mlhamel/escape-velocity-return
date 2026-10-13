use std::path::Path;
use std::time::Duration;

use bevy::app::PostUpdate;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::netcode::{
    ClientSocket, MemorySocketClient, NetcodeClientTransport, NetcodeServerTransport,
};
use bevy_replicon_renet2::renet2::{ConnectionConfig, RenetClient, RenetServer};
use bevy_replicon_renet2::{RenetChannelsExt, RepliconRenetPlugins};
use evr_protocol::{
    AccountState, BuyCommodity, DockedAt, Hull, Notice, PlanetTag, PlayerDied, Position,
    Projectile, RequestLand, RequestLiftOff, SellCommodity, SetFiring, SetThrottle, SetTurn,
    SetWarp, ShipInfo, ShipKind, SystemMembership,
};
use evr_server::{
    Content, EMBEDDED_CLIENT_ID, TickMode, attach_memory, build_server_app,
    memory_reconnect_transport,
};

#[derive(Resource, Default)]
pub struct NoticeLog(pub Vec<String>);

pub struct Harness {
    pub server: App,
    pub client: App,
    pub ticks: u64,
    client_socket: MemorySocketClient,
}

impl Harness {
    pub fn new(content_root: &Path) -> Self {
        let mut server = build_server_app(content_root, TickMode::PerUpdate, None)
            .expect("server app must build");
        let (transport, client_socket) = attach_memory(&mut server);
        server.finish();

        let mut client = App::new();
        client.add_plugins(MinimalPlugins);
        client.add_plugins(StatesPlugin);
        client.add_plugins(RepliconPlugins.set(ServerPlugin::new(PostUpdate)));
        client.add_plugins(RepliconRenetPlugins);
        evr_protocol::register(&mut client);
        client.init_resource::<NoticeLog>();
        client.add_observer(record_notice);
        client.add_observer(record_death);

        let (server_channels, client_channels) = {
            let channels = client.world().resource::<RepliconChannels>();
            (channels.server_configs(), channels.client_configs())
        };
        let renet_client = RenetClient::new(
            ConnectionConfig::from_channels(server_channels, client_channels),
            false,
        );
        client.insert_resource(renet_client);
        client.insert_resource(transport);
        client.finish();

        let mut harness = Self {
            server,
            client,
            ticks: 0,
            client_socket,
        };
        let mut connect_steps = 0;
        while !harness
            .client
            .world()
            .resource::<RenetClient>()
            .is_connected()
        {
            harness.step();
            connect_steps += 1;
            assert!(connect_steps < 300, "client must connect");
        }
        for _ in 0..10 {
            harness.step();
        }
        harness
    }

    pub fn step(&mut self) {
        self.server.update();
        self.client.update();
        std::thread::sleep(Duration::from_millis(15));
        self.ticks += 1;
    }

    pub fn advance(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.step();
        }
    }

    /// Restores the connection after an idle gap. The netcode transport drops
    /// the connection after 15 seconds without traffic, so callers that pause
    /// between commands must call this before touching the game.
    ///
    /// The renet client only notices the drop one update later, so this checks
    /// the transport's own netcode state, which is updated synchronously.
    pub fn ensure_connected(&mut self) {
        self.step();
        let connected = {
            let transport = self
                .client
                .world()
                .resource::<bevy_replicon_renet2::netcode::NetcodeClientTransport>();
            transport.is_connected()
        };
        if connected {
            return;
        }
        self.force_reconnect();
    }

    /// Replaces the client transport and re-establishes the connection over
    /// the same memory socket. Server-side game state is untouched; the
    /// server rebinds the existing player to the new client entity.
    pub fn force_reconnect(&mut self) {
        self.server.world_mut().resource_scope(
            |world, mut transport: Mut<NetcodeServerTransport>| {
                world.resource_scope(|_, mut renet: Mut<RenetServer>| {
                    transport.disconnect_all(&mut renet);
                });
            },
        );

        {
            let mut buffer = vec![0u8; 4096];
            while self.client_socket.try_recv(&mut buffer).is_ok() {}
        }

        {
            let mut remote = self
                .client
                .world_mut()
                .query_filtered::<Entity, With<Remote>>();
            let world = self.client.world_mut();
            for entity in remote.iter(world).collect::<Vec<_>>() {
                world.despawn(entity);
            }
        }

        let (server_channels, client_channels) = {
            let channels = self.client.world().resource::<RepliconChannels>();
            (channels.server_configs(), channels.client_configs())
        };
        self.client
            .world_mut()
            .remove_resource::<NetcodeClientTransport>();
        self.client.world_mut().remove_resource::<RenetClient>();
        self.client.world_mut().insert_resource(RenetClient::new(
            ConnectionConfig::from_channels(server_channels, client_channels),
            false,
        ));
        self.client
            .world_mut()
            .insert_resource(memory_reconnect_transport(&self.client_socket));

        let mut connect_steps = 0;
        while !self.client.world().resource::<RenetClient>().is_connected() {
            self.step();
            connect_steps += 1;
            assert!(connect_steps < 300, "client must reconnect");
        }
        for _ in 0..10 {
            self.step();
        }
    }

    pub fn set_flight(&mut self, throttle: f32, turn: f32, firing: bool) {
        self.client
            .world_mut()
            .client_trigger(SetThrottle(throttle.clamp(0.0, 1.0)));
        self.client
            .world_mut()
            .client_trigger(SetTurn(turn.clamp(-1.0, 1.0)));
        self.client.world_mut().client_trigger(SetFiring(firing));
    }

    pub fn land(&mut self, planet: &str) {
        self.client.world_mut().client_trigger(RequestLand {
            planet: planet.to_owned(),
        });
    }

    pub fn lift_off(&mut self) {
        self.client.world_mut().client_trigger(RequestLiftOff);
    }

    pub fn buy(&mut self, commodity: &str, qty: u32) {
        self.client.world_mut().client_trigger(BuyCommodity {
            commodity: commodity.to_owned(),
            qty,
        });
    }

    pub fn sell(&mut self, commodity: &str, qty: u32) {
        self.client.world_mut().client_trigger(SellCommodity {
            commodity: commodity.to_owned(),
            qty,
        });
    }

    pub fn set_warp(&mut self, on: bool) {
        self.client.world_mut().client_trigger(SetWarp(on));
    }

    pub fn observe(&mut self) -> serde_json::Value {
        let mut notices = Vec::new();
        if let Some(log) = self.client.world().get_resource::<NoticeLog>() {
            notices = log.0.clone();
        }
        self.client
            .world_mut()
            .resource_mut::<NoticeLog>()
            .0
            .clear();

        let owner = EMBEDDED_CLIENT_ID as u64;
        let my_ship = {
            let mut state = self.server.world_mut().query::<(
                &ShipInfo,
                &evr_server::gameplay::Ship,
                &SystemMembership,
                Option<&DockedAt>,
                Option<&AccountState>,
            )>();
            let world = self.server.world();
            state
                .iter(world)
                .find(|(info, _, _, _, _)| info.owner == Some(owner))
                .map(|(info, ship, membership, docked, account)| {
                    (
                        info.name.clone(),
                        ship.0.clone(),
                        membership.0.clone(),
                        docked.map(|d| (d.system.clone(), d.planet.clone())),
                        account.cloned(),
                    )
                })
        };

        let Some((ship_name, ship, system, docked, account)) = my_ship else {
            return serde_json::json!({
                "tick": self.ticks,
                "alive": false,
                "notices": notices,
            });
        };

        let content = self.server.world().resource::<Content>();
        let market = docked
            .as_ref()
            .and_then(|(system_id, planet_id)| {
                content
                    .0
                    .system(system_id)
                    .and_then(|system| system.planets.iter().find(|p| p.id == *planet_id))
            })
            .map(|planet| {
                planet
                    .commodities
                    .iter()
                    .map(|c| serde_json::json!({ "id": c.id, "price": c.price }))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let planets = {
            let mut state = self.server.world_mut().query::<(&PlanetTag, &Position)>();
            let world = self.server.world();
            state
                .iter(world)
                .filter(|(tag, _)| tag.system == system)
                .map(|(tag, position)| {
                    serde_json::json!({
                        "id": tag.id,
                        "name": tag.name,
                        "position": [position.0.x, position.0.y],
                        "radius": tag.radius,
                    })
                })
                .collect::<Vec<_>>()
        };

        let ships = {
            let mut state = self
                .server
                .world_mut()
                .query::<(&ShipInfo, &Position, &Hull)>();
            let world = self.server.world();
            state
                .iter(world)
                .filter(|(info, _, _)| info.owner != Some(owner))
                .map(|(info, position, hull)| {
                    serde_json::json!({
                        "name": info.name,
                        "kind": match info.kind {
                            ShipKind::Player => "player",
                            ShipKind::Pirate => "pirate",
                            ShipKind::Police => "police",
                        },
                        "position": [position.0.x, position.0.y],
                        "shields": hull.shields,
                        "armor": hull.armor,
                    })
                })
                .collect::<Vec<_>>()
        };

        let projectile_count = {
            let mut state = self.server.world_mut().query::<&Projectile>();
            let world = self.server.world();
            state.iter(world).count()
        };

        let account = account.unwrap_or(AccountState {
            credits: 0,
            cargo: Vec::new(),
            cargo_capacity: 0,
        });

        serde_json::json!({
            "tick": self.ticks,
            "alive": true,
            "system": system,
            "docked": docked.map(|(_, planet)| planet),
            "ship": {
                "name": ship_name,
                "position": [ship.position.x, ship.position.y],
                "velocity": [ship.velocity.x, ship.velocity.y],
                "heading": ship.rotation,
                "shields": ship.shields,
                "armor": ship.armor,
                "fuel": ship.fuel,
                "credits": account.credits,
                "cargo": account.cargo.iter().cloned().collect::<std::collections::BTreeMap<String, u32>>(),
                "cargo_capacity": account.cargo_capacity,
            },
            "planets": planets,
            "ships": ships,
            "market": market,
            "projectiles": projectile_count,
            "notices": notices,
        })
    }
}

fn record_notice(trigger: On<Notice>, mut log: ResMut<NoticeLog>) {
    log.0.push(trigger.event().text.clone());
}

fn record_death(_death: On<PlayerDied>, mut log: ResMut<NoticeLog>) {
    log.0.push("Your ship was destroyed.".to_owned());
}
