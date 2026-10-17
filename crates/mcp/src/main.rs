use evr_mcp::harness::Harness;

use std::path::PathBuf;

use rmcp::ServiceExt;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::schemars::JsonSchema;
use rmcp::tool;
use rmcp::tool_router;
use rmcp::transport::stdio;
use serde::Deserialize;
use tokio::sync::{mpsc, oneshot};

const SETTLE_TICKS: u32 = 10;
const MAX_TICKS: u32 = 600;

#[derive(Debug, Deserialize, JsonSchema)]
struct AdvanceArgs {
    /// Number of simulation ticks to run. Each tick is ~15ms of game time. Max 600.
    ticks: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SetFlightArgs {
    /// Forward thrust, 0.0 to 1.0
    throttle: f32,
    /// Turn input, -1.0 (left) to 1.0 (right)
    turn: f32,
    /// Hold weapons firing
    firing: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct LandArgs {
    /// Planet id to land at (see observe.planets); must be within landing range
    planet: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TradeArgs {
    /// Commodity id (see observe.market while docked)
    commodity: String,
    /// Quantity to trade
    qty: u32,
    /// "buy" or "sell"
    direction: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct WarpArgs {
    /// Enable warp drive
    on: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RespondArgs {
    /// Encounter option id (e.g. "accept_quest", "turn_in", "accept_offer", "pay_tribute", "decline") or 1-based option number ("1", "2")
    choice: String,
}

enum Command {
    Advance(u32),
    Observe,
    SetFlight {
        throttle: f32,
        turn: f32,
        firing: bool,
    },
    Land(String),
    LiftOff,
    Trade {
        commodity: String,
        qty: u32,
        buy: bool,
    },
    SetWarp {
        on: bool,
    },
    Respond(String),
}

#[derive(Clone)]
struct GameMcp {
    tx: mpsc::Sender<(Command, oneshot::Sender<String>)>,
}

impl GameMcp {
    fn new(content_root: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel(32);
        std::thread::spawn(move || game_thread(content_root, rx));
        Self { tx }
    }

    async fn call(&self, command: Command) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send((command, reply_tx)).await.is_err() {
            return "the game thread is gone".to_owned();
        }
        reply_rx
            .await
            .unwrap_or_else(|_| "the game thread is gone".to_owned())
    }
}

#[tool_router(server_handler)]
impl GameMcp {
    #[tool(description = "Advance the game simulation. Ticks are ~15ms each; max 600 per call.")]
    async fn advance(&self, Parameters(AdvanceArgs { ticks }): Parameters<AdvanceArgs>) -> String {
        self.call(Command::Advance(ticks.min(MAX_TICKS))).await
    }

    #[tool(
        description = "Observe the full authoritative game state as JSON: ship, planets, ships, market, encounter, quests, notices."
    )]
    async fn observe(&self) -> String {
        self.call(Command::Observe).await
    }

    #[tool(
        description = "Set flight inputs: throttle (0..1), turn (-1..1), and firing hold. Returns the updated state."
    )]
    async fn set_flight(
        &self,
        Parameters(SetFlightArgs {
            throttle,
            turn,
            firing,
        }): Parameters<SetFlightArgs>,
    ) -> String {
        self.call(Command::SetFlight {
            throttle,
            turn,
            firing,
        })
        .await
    }

    #[tool(
        description = "Request landing at a planet. Fly within landing range first. Returns the updated state."
    )]
    async fn land(&self, Parameters(LandArgs { planet }): Parameters<LandArgs>) -> String {
        self.call(Command::Land(planet)).await
    }

    #[tool(description = "Lift off from the planet you are docked at. Returns the updated state.")]
    async fn lift_off(&self) -> String {
        self.call(Command::LiftOff).await
    }

    #[tool(
        description = "Buy or sell a commodity while docked. Returns the updated state; check notices for the result."
    )]
    async fn trade(
        &self,
        Parameters(TradeArgs {
            commodity,
            qty,
            direction,
        }): Parameters<TradeArgs>,
    ) -> String {
        match direction.as_str() {
            "buy" => {
                self.call(Command::Trade {
                    commodity,
                    qty,
                    buy: true,
                })
                .await
            }
            "sell" => {
                self.call(Command::Trade {
                    commodity,
                    qty,
                    buy: false,
                })
                .await
            }
            other => format!("direction must be \"buy\" or \"sell\", got {other:?}"),
        }
    }

    #[tool(description = "Toggle warp drive. Warp multiplies speed by 3 and drains fuel faster.")]
    async fn warp(&self, Parameters(args): Parameters<WarpArgs>) -> String {
        self.call(Command::SetWarp { on: args.on }).await
    }

    #[tool(
        description = "Respond to a planet chief encounter while docked (see observe.encounter.options). Returns the updated state."
    )]
    async fn respond(&self, Parameters(RespondArgs { choice }): Parameters<RespondArgs>) -> String {
        self.call(Command::Respond(choice)).await
    }
}

fn game_thread(content_root: PathBuf, mut rx: mpsc::Receiver<(Command, oneshot::Sender<String>)>) {
    let mut harness = Harness::new(&content_root);
    while let Some((command, reply)) = rx.blocking_recv() {
        harness.ensure_connected();
        let _ = reply.send(match command {
            Command::Advance(ticks) => {
                harness.advance(ticks);
                format!("advanced {ticks} ticks (tick {})", harness.ticks)
            }
            Command::Observe => json(&harness.observe()),
            Command::SetFlight {
                throttle,
                turn,
                firing,
            } => {
                harness.set_flight(throttle, turn, firing);
                harness.advance(SETTLE_TICKS);
                json(&harness.observe())
            }
            Command::Land(planet) => {
                harness.land(&planet);
                harness.advance(SETTLE_TICKS);
                json(&harness.observe())
            }
            Command::LiftOff => {
                harness.lift_off();
                harness.advance(SETTLE_TICKS);
                json(&harness.observe())
            }
            Command::Trade {
                commodity,
                qty,
                buy,
            } => {
                if buy {
                    harness.buy(&commodity, qty);
                } else {
                    harness.sell(&commodity, qty);
                }
                harness.advance(SETTLE_TICKS);
                json(&harness.observe())
            }
            Command::SetWarp { on } => {
                harness.set_warp(on);
                harness.advance(SETTLE_TICKS);
                json(&harness.observe())
            }
            Command::Respond(choice) => {
                harness.respond(&choice);
                harness.advance(SETTLE_TICKS);
                json(&harness.observe())
            }
        });
    }
}

fn json(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|err| format!("serialize failed: {err}"))
}

fn content_root() -> PathBuf {
    if std::path::Path::new("content").exists() {
        PathBuf::from("content")
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let service = GameMcp::new(content_root());
        let server = service.serve(stdio()).await?;
        server.waiting().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    Ok(())
}
