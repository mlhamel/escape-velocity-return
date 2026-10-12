use bevy::prelude::*;
use bevy_replicon::prelude::*;
use evr_data::ContentPack;
use evr_protocol::{
    BuyCommodity, DockedAt, PlanetTag, Position, RequestHyperjump, RequestLand, RequestLiftOff,
    SellCommodity, SetFiring, SetThrottle, SetTurn, ShipInfo, SystemMembership,
};
use evr_server::gameplay::LAND_RANGE;

use crate::MyNetworkId;

#[derive(Resource, Default)]
struct InputState {
    throttle: bool,
    turn: i32,
    firing: bool,
}

#[derive(Resource)]
pub struct ClientContent(pub ContentPack);

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputState>()
            .add_systems(Startup, load_client_content)
            .add_systems(Update, poll_flight_input)
            .add_systems(Update, poll_action_input);
    }
}

fn load_client_content(mut commands: Commands) {
    let pack = ContentPack::load_dir(std::path::Path::new("content"))
        .expect("client content pack must load");
    commands.insert_resource(ClientContent(pack));
}

fn poll_flight_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<InputState>,
    mut commands: Commands,
) {
    let throttle = keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    if throttle != state.throttle {
        state.throttle = throttle;
        commands.client_trigger(SetThrottle(if throttle { 1.0 } else { 0.0 }));
    }

    let left = i32::from(keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]));
    let right = i32::from(keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]));
    let turn = right - left;
    if turn != state.turn {
        state.turn = turn;
        commands.client_trigger(SetTurn(turn as f32));
    }

    let firing = keys.pressed(KeyCode::Space);
    if firing != state.firing {
        state.firing = firing;
        commands.client_trigger(SetFiring(firing));
    }
}

fn poll_action_input(
    my_id: Res<MyNetworkId>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    content: Res<ClientContent>,
    ships: Query<(&ShipInfo, &Position, &SystemMembership, Option<&DockedAt>)>,
    planets: Query<(&PlanetTag, &Position)>,
) {
    let Some((_, position, membership, docked)) = ships
        .iter()
        .find(|(info, _, _, _)| info.owner == Some(my_id.0))
    else {
        return;
    };

    if docked.is_none() && keys.just_pressed(KeyCode::KeyL) {
        let candidate = planets
            .iter()
            .filter(|(tag, _)| tag.system == membership.0)
            .filter(|(tag, planet_position)| {
                position.0.distance(planet_position.0) <= LAND_RANGE + tag.radius
            })
            .min_by_key(|(_, planet_position)| position.0.distance(planet_position.0).to_bits());
        if let Some((planet, _)) = candidate {
            commands.client_trigger(RequestLand {
                planet: planet.id.clone(),
            });
        }
    }

    if docked.is_some() && keys.just_pressed(KeyCode::KeyU) {
        commands.client_trigger(RequestLiftOff);
    }

    if docked.is_none()
        && keys.just_pressed(KeyCode::KeyJ)
        && let Some(current) = content.0.system(&membership.0)
        && let Some(target) = current.links.first()
    {
        commands.client_trigger(RequestHyperjump {
            target: target.clone(),
        });
    }

    let Some(docked) = docked else {
        return;
    };
    let Some(system) = content.0.system(&docked.system) else {
        return;
    };
    let Some(planet) = system.planets.iter().find(|p| p.id == docked.planet) else {
        return;
    };
    for (index, commodity) in planet.commodities.iter().enumerate() {
        let digit = match index {
            0 => KeyCode::Digit1,
            1 => KeyCode::Digit2,
            2 => KeyCode::Digit3,
            _ => continue,
        };
        if keys.just_pressed(digit) {
            let selling = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
            if selling {
                commands.client_trigger(SellCommodity {
                    commodity: commodity.id.clone(),
                    qty: 1,
                });
            } else {
                commands.client_trigger(BuyCommodity {
                    commodity: commodity.id.clone(),
                    qty: 1,
                });
            }
        }
    }
}
