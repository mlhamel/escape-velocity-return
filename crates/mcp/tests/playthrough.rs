use std::f64::consts::PI;
use std::path::Path;

use evr_mcp::Harness;
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

    harness.jump("alpha-centauri");
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
    assert!(
        !state["notices"].as_array().unwrap().is_empty(),
        "notices must have accumulated"
    );
}
