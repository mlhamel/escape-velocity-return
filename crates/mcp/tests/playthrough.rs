use std::f64::consts::PI;
use std::path::Path;

use evr_mcp::Harness;
use evr_protocol::ShipInfo;
use evr_server::EMBEDDED_CLIENT_ID;
use serde_json::Value;

fn steer_to(target: [f64; 2], position: [f64; 2], heading: f64) -> (f32, f32) {
    let dx = target[0] - position[0];
    let dy = target[1] - position[1];
    let desired = dy.atan2(dx);
    let diff = (desired - heading + PI).rem_euclid(2.0 * PI) - PI;
    let turn = diff.signum();
    let throttle = if diff.abs() < 0.6 { 1.0 } else { 0.2 };
    (turn as f32, throttle as f32)
}

fn vec2(value: &Value, key: &str) -> [f64; 2] {
    let raw = value[key].as_array().expect("vec2 must be an array");
    [raw[0].as_f64().expect("x"), raw[1].as_f64().expect("y")]
}

fn fly_to(harness: &mut Harness, target: [f64; 2], stop_distance: f64, max_ticks: u32) -> bool {
    for _ in 0..max_ticks {
        let state = harness.observe();
        let position = vec2(&state["ship"], "position");
        if (position[0] - target[0]).hypot(position[1] - target[1]) <= stop_distance {
            return true;
        }
        let heading = state["ship"]["heading"].as_f64().expect("heading");
        let (turn, throttle) = steer_to(target, position, heading);
        harness.set_flight(throttle, turn, false);
        harness.advance(10);
    }
    false
}

#[test]
fn agent_playthrough() {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let mut harness = Harness::new(&content);
    harness.advance(20);

    let state = harness.observe();
    assert_eq!(state["system"], "sol");
    assert_eq!(state["ship"]["credits"], 500);
    assert_eq!(state["ship"]["cargo_capacity"], 20);
    assert!(!state["planets"].as_array().unwrap().is_empty());

    let earth = state["planets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "earth")
        .expect("earth visible")
        .clone();
    let earth_pos = vec2(&earth, "position");

    assert!(
        fly_to(&mut harness, earth_pos, 280.0, 600),
        "must reach earth"
    );

    harness.land("earth");
    harness.advance(20);
    let state = harness.observe();
    assert_eq!(state["docked"], "earth");

    harness.buy("food", 2);
    harness.advance(10);
    let state = harness.observe();
    assert_eq!(state["ship"]["credits"], 484);
    assert_eq!(state["ship"]["cargo"]["food"], 2);

    harness.sell("food", 1);
    harness.advance(10);
    let state = harness.observe();
    assert_eq!(state["ship"]["credits"], 492);

    harness.lift_off();
    harness.advance(10);
    assert_eq!(harness.observe()["docked"], Value::Null);

    harness.set_warp(true);
    assert!(
        fly_to(&mut harness, [1250.0, 0.0], 60.0, 600),
        "must reach alpha centauri space"
    );
    harness.set_flight(0.0, 0.0, false);
    harness.set_warp(false);
    harness.advance(10);
    let state = harness.observe();
    assert_eq!(state["system"], "alpha-centauri");
    assert!(state["ship"]["fuel"].as_f64().unwrap() < 120.0);
    assert!(
        state["ships"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["kind"] == "pirate"),
        "pirate must be present in the system"
    );

    let newhaven = state["planets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "newhaven")
        .expect("newhaven visible from alpha centauri space")
        .clone();
    let newhaven_pos = vec2(&newhaven, "position");
    harness.set_warp(true);
    assert!(
        fly_to(&mut harness, newhaven_pos, 250.0, 600),
        "must reach newhaven"
    );
    harness.set_flight(0.0, 0.0, false);
    harness.set_warp(false);
    harness.land("newhaven");
    harness.advance(20);
    let state = harness.observe();
    assert_eq!(state["docked"], "newhaven");
    assert!(
        !state["notices"].as_array().unwrap().is_empty(),
        "notices must have accumulated"
    );
}

#[test]
fn forced_reconnect_keeps_the_game_responsive() {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let mut harness = Harness::new(&content);

    assert!(
        fly_to(&mut harness, [300.0, 0.0], 60.0, 200),
        "flight must work before the forced reconnect"
    );

    harness.force_reconnect();

    assert!(
        fly_to(&mut harness, [600.0, 0.0], 60.0, 200),
        "flight must still work after a forced reconnect"
    );
}

#[test]
fn harness_survives_idle_gap() {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let mut harness = Harness::new(&content);

    let state = harness.observe();
    let start = vec2(&state["ship"], "position");
    assert_eq!(state["alive"], serde_json::Value::Bool(true));

    std::thread::sleep(std::time::Duration::from_secs(20));
    harness.ensure_connected();

    let state = harness.observe();
    let after_idle = vec2(&state["ship"], "position");
    assert_eq!(start, after_idle, "server state must survive the idle gap");

    assert!(
        fly_to(&mut harness, [start[0] + 300.0, start[1]], 60.0, 200),
        "ship must still respond after an idle gap"
    );

    let state = harness.observe();
    assert_eq!(state["alive"], serde_json::Value::Bool(true));
    let players = {
        let mut query = harness.server.world_mut().query::<&ShipInfo>();
        query
            .iter(harness.server.world())
            .filter(|info| info.owner == Some(EMBEDDED_CLIENT_ID as u64))
            .count()
    };
    assert_eq!(players, 1, "reconnect must not spawn a duplicate player");
}
