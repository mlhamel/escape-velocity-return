use std::path::Path;
use std::time::Duration;

use bevy::app::PostUpdate;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::renet2::{ConnectionConfig, RenetClient};
use bevy_replicon_renet2::{RenetChannelsExt, RepliconRenetPlugins};
use evr_protocol::{
    AccountState, BuyCommodity, DockedAt, Hull, Notice, PlanetTag, PlayerDied, Position,
    Projectile, RequestHyperjump, RequestLand, RequestLiftOff, SellCommodity, SetFiring,
    SetThrottle, SetTurn, ShipInfo, ShipKind, SystemMembership,
};
use evr_server::{Content, EMBEDDED_CLIENT_ID, TickMode, attach_memory, build_server_app};

#[derive(Resource, Default)]
pub struct NoticeLog(pub Vec<String>);

pub struct Harness {
    pub server: App,
    pub client: App,
    pub ticks: u64,
}

impl Harness {
    pub fn new(content_root: &Path) -> Self {
        let mut server =
            build_server_app(content_root, TickMode::PerUpdate).expect("server app must build");
        let transport = attach_memory(&mut server);
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

    pub fn jump(&mut self, target: &str) {
        self.client.world_mut().client_trigger(RequestHyperjump {
            target: target.to_owned(),
        });
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
