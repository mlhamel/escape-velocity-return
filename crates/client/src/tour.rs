use std::f32::consts::PI;

use bevy::math::Vec2;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use evr_protocol::{
    BuyCommodity, DockedAt, Heading, PlanetTag, Position, RequestHyperjump, RequestLand, SetFiring,
    SetThrottle, SetTurn, ShipInfo, ShipKind, SystemMembership,
};

use crate::MyNetworkId;
use crate::screenshots::request_capture;

const CAPTURE_DIR: &str = "docs/screenshots";
const STAGE_TICK_LIMIT: u32 = 900;

pub struct CaptureTourPlugin;

impl Plugin for CaptureTourPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(CAPTURE_DIR).expect("screenshot dir must be creatable");
        app.init_resource::<Tour>().add_systems(Update, run_tour);
    }
}

#[derive(Resource)]
struct Tour {
    stage: TourStage,
    actioned: bool,
    ticks: u32,
}

impl Default for Tour {
    fn default() -> Self {
        Self {
            stage: TourStage::ApproachEarth,
            actioned: false,
            ticks: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum TourStage {
    ApproachEarth,
    JumpToAlpha,
    Duel,
    ApproachNewhaven,
    Dock,
    Flush,
    Done,
}

fn steer(current: Vec2, heading: f32, target: Vec2) -> (f32, f32) {
    let delta = target - current;
    let desired = delta.y.atan2(delta.x);
    let diff = (desired - heading + PI).rem_euclid(2.0 * PI) - PI;
    let turn = diff.signum();
    let throttle = if diff.abs() < 0.6 { 1.0 } else { 0.2 };
    (turn, throttle)
}

type TourShips<'w, 's> = Query<
    'w,
    's,
    (
        &'static ShipInfo,
        &'static Position,
        &'static Heading,
        &'static SystemMembership,
        Option<&'static DockedAt>,
    ),
>;

fn run_tour(
    my_id: Res<MyNetworkId>,
    mut commands: Commands,
    mut tour: ResMut<Tour>,
    mut exit: MessageWriter<AppExit>,
    ships: TourShips,
    planets: Query<(&PlanetTag, &Position)>,
) {
    let Some((_, position, heading, membership, docked)) = ships
        .iter()
        .find(|(info, _, _, _, _)| info.owner == Some(my_id.0))
    else {
        return;
    };

    tour.ticks += 1;
    if tour.ticks > STAGE_TICK_LIMIT {
        exit.write(AppExit::Error(
            std::num::NonZero::<u8>::new(1).expect("one is non-zero"),
        ));
        return;
    }

    match tour.stage {
        TourStage::ApproachEarth => {
            let Some((_, earth)) = planets
                .iter()
                .find(|(tag, _)| tag.id == "earth" && tag.system == membership.0)
            else {
                return;
            };
            if position.0.distance(earth.0) < 380.0 {
                commands.client_trigger(SetThrottle(0.0));
                request_capture(&mut commands, format!("{CAPTURE_DIR}/01-flight-earth.png"));
                tour.stage = TourStage::JumpToAlpha;
                tour.actioned = false;
                tour.ticks = 0;
            } else {
                let (turn, throttle) = steer(position.0, heading.0, earth.0);
                commands.client_trigger(SetTurn(turn));
                commands.client_trigger(SetThrottle(throttle));
            }
        }
        TourStage::JumpToAlpha => {
            if !tour.actioned {
                tour.actioned = true;
                commands.client_trigger(RequestHyperjump {
                    target: "alpha-centauri".to_owned(),
                });
            } else if membership.0 == "alpha-centauri" {
                tour.stage = TourStage::Duel;
                tour.actioned = false;
                tour.ticks = 0;
            }
        }
        TourStage::Duel => {
            let Some(pirate_position) = ships
                .iter()
                .find(|(info, _, _, _, _)| info.kind == ShipKind::Pirate)
                .map(|(_, position, _, _, _)| position.0)
            else {
                return;
            };
            let distance = position.0.distance(pirate_position);
            if distance < 520.0 {
                let (turn, throttle) = steer(position.0, heading.0, pirate_position);
                commands.client_trigger(SetTurn(turn));
                commands.client_trigger(SetThrottle(if distance > 260.0 { throttle } else { 0.0 }));
                if !tour.actioned {
                    tour.actioned = true;
                    commands.client_trigger(SetFiring(true));
                } else if tour.ticks > 45 {
                    commands.client_trigger(SetFiring(false));
                    request_capture(&mut commands, format!("{CAPTURE_DIR}/02-combat.png"));
                    tour.stage = TourStage::ApproachNewhaven;
                    tour.actioned = false;
                    tour.ticks = 0;
                }
            } else {
                commands.client_trigger(SetThrottle(0.0));
            }
        }
        TourStage::ApproachNewhaven => {
            let Some((tag, newhaven)) = planets
                .iter()
                .find(|(tag, _)| tag.id == "newhaven" && tag.system == membership.0)
            else {
                return;
            };
            if position.0.distance(newhaven.0) <= 180.0 + tag.radius {
                commands.client_trigger(SetThrottle(0.0));
                commands.client_trigger(RequestLand {
                    planet: "newhaven".to_owned(),
                });
                tour.stage = TourStage::Dock;
                tour.actioned = false;
                tour.ticks = 0;
            } else {
                let (turn, throttle) = steer(position.0, heading.0, newhaven.0);
                commands.client_trigger(SetTurn(turn));
                commands.client_trigger(SetThrottle(throttle));
            }
        }
        TourStage::Dock => {
            if docked.is_none() {
                return;
            }
            if !tour.actioned {
                tour.actioned = true;
                commands.client_trigger(BuyCommodity {
                    commodity: "food".to_owned(),
                    qty: 1,
                });
            } else if tour.ticks > 45 {
                request_capture(&mut commands, format!("{CAPTURE_DIR}/03-docked-trade.png"));
                tour.stage = TourStage::Flush;
                tour.ticks = 0;
            }
        }
        TourStage::Flush => {
            if tour.ticks > 60 {
                exit.write(AppExit::Success);
                tour.stage = TourStage::Done;
            }
        }
        TourStage::Done => {}
    }
}
