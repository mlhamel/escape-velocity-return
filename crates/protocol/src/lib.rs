use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientIntent {
    SetThrottle(f32),
    SetTurn(f32),
    ToggleWeapons,
    RequestHyperjump { system: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerEvent {
    ShipSpawned { id: u64, position: (f32, f32) },
    ShipDestroyed { id: u64 },
    HyperjumpComplete { system: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intents_roundtrip_through_ron() {
        let intents = [
            ClientIntent::SetThrottle(0.8),
            ClientIntent::SetTurn(-1.0),
            ClientIntent::ToggleWeapons,
            ClientIntent::RequestHyperjump {
                system: "sol".into(),
            },
        ];
        for intent in intents {
            let text = ron::to_string(&intent).expect("intent must serialize");
            let parsed: ClientIntent = ron::from_str(&text).expect("intent must deserialize");
            assert_eq!(parsed, intent);
        }
    }

    #[test]
    fn events_roundtrip_through_ron() {
        let events = [
            ServerEvent::ShipSpawned {
                id: 42,
                position: (10.0, -5.0),
            },
            ServerEvent::ShipDestroyed { id: 42 },
            ServerEvent::HyperjumpComplete {
                system: "sol".into(),
            },
        ];
        for event in events {
            let text = ron::to_string(&event).expect("event must serialize");
            let parsed: ServerEvent = ron::from_str(&text).expect("event must deserialize");
            assert_eq!(parsed, event);
        }
    }
}
