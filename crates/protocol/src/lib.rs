use bevy::math::Vec2;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position(pub Vec2);

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Heading(pub f32);

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hull {
    pub shields: f32,
    pub max_shields: f32,
    pub armor: f32,
    pub max_armor: f32,
}

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fuel {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShipInfo {
    pub def_id: String,
    pub kind: ShipKind,
    pub owner: Option<u64>,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShipKind {
    Player,
    Pirate,
}

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Projectile {
    pub damage: f32,
    pub ttl: f32,
    pub owner: u64,
}

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetTag {
    pub system: String,
    pub id: String,
    pub name: String,
    pub radius: f32,
}

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMembership(pub String);

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockedAt {
    pub system: String,
    pub planet: String,
}

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountState {
    pub credits: u32,
    pub cargo: Vec<(String, u32)>,
    pub cargo_capacity: u32,
}

#[derive(Event, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetThrottle(pub f32);

#[derive(Event, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetTurn(pub f32);

#[derive(Event, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetFiring(pub bool);

#[derive(Event, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestLand {
    pub planet: String,
}

#[derive(Event, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestLiftOff;

#[derive(Event, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuyCommodity {
    pub commodity: String,
    pub qty: u32,
}

#[derive(Event, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SellCommodity {
    pub commodity: String,
    pub qty: u32,
}

#[derive(Event, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestHyperjump {
    pub target: String,
}

#[derive(Event, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notice {
    pub text: String,
}

#[derive(Event, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerDied;

#[derive(Event, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Explosion {
    pub position: (f32, f32),
    pub system: String,
}

pub fn register(app: &mut App) {
    app.replicate::<Position>()
        .replicate::<Heading>()
        .replicate::<Hull>()
        .replicate::<Fuel>()
        .replicate::<ShipInfo>()
        .replicate::<Projectile>()
        .replicate::<PlanetTag>()
        .replicate::<SystemMembership>()
        .replicate::<DockedAt>()
        .replicate::<AccountState>()
        .add_client_event::<SetThrottle>(Channel::Ordered)
        .add_client_event::<SetTurn>(Channel::Ordered)
        .add_client_event::<SetFiring>(Channel::Ordered)
        .add_client_event::<RequestLand>(Channel::Ordered)
        .add_client_event::<RequestLiftOff>(Channel::Ordered)
        .add_client_event::<BuyCommodity>(Channel::Ordered)
        .add_client_event::<SellCommodity>(Channel::Ordered)
        .add_client_event::<RequestHyperjump>(Channel::Ordered)
        .add_server_event::<Notice>(Channel::Ordered)
        .add_server_event::<PlayerDied>(Channel::Ordered)
        .add_server_event::<Explosion>(Channel::Unordered);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;

    fn roundtrip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: T) {
        let text = ron::to_string(&value).expect("must serialize");
        let parsed: T = ron::from_str(&text).expect("must deserialize");
        assert_eq!(parsed, value);
    }

    #[test]
    fn components_roundtrip() {
        roundtrip(Position(Vec2::new(12.5, -3.25)));
        roundtrip(Heading(1.25));
        roundtrip(Hull {
            shields: 40.0,
            max_shields: 40.0,
            armor: 25.0,
            max_armor: 25.0,
        });
        roundtrip(ShipInfo {
            def_id: "shuttle".into(),
            kind: ShipKind::Pirate,
            owner: None,
            name: "Turncoat".into(),
        });
        roundtrip(AccountState {
            credits: 500,
            cargo: vec![("food".into(), 3)],
            cargo_capacity: 20,
        });
    }

    #[test]
    fn events_roundtrip() {
        roundtrip(SetThrottle(0.8));
        roundtrip(SetTurn(-1.0));
        roundtrip(SetFiring(true));
        roundtrip(RequestLand {
            planet: "earth".into(),
        });
        roundtrip(BuyCommodity {
            commodity: "food".into(),
            qty: 5,
        });
        roundtrip(RequestHyperjump {
            target: "alpha-centauri".into(),
        });
        roundtrip(Notice {
            text: "No fuel, no glory.".into(),
        });
        roundtrip(PlayerDied);
        roundtrip(Explosion {
            position: (10.0, -4.0),
            system: "sol".into(),
        });
    }
}
