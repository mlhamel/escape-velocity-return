use std::f32::consts::PI;

use bevy::math::Vec2;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon::shared::backend::connected_client::NetworkId;
use evr_core::{Account, FlightModel, ShipState, WeaponState};
use evr_data::{ContentPack, OutfitKind, WeaponDef};
use evr_protocol::{
    AccountState, BuyCommodity, DockedAt, Explosion, Fuel, Heading, Hull, Notice, PlanetTag,
    PlayerDied, Position, Projectile, RequestHyperjump, RequestLand, RequestLiftOff, SellCommodity,
    SetFiring, SetThrottle, SetTurn, ShipInfo, ShipKind, SystemMembership,
};

use crate::Content;

pub const PLAYER_SYSTEM: &str = "sol";
pub const PIRATE_SYSTEM: &str = "alpha-centauri";
pub const PLAYER_SHIP: &str = "shuttle";
pub const PIRATE_SHIP: &str = "turncoat";
pub const POLICE_SHIP: &str = "patrol";
pub const POLICED_GOVERNMENT: &str = "confederation";
pub const FINE_CREDITS: u32 = 200;
pub const STARTING_CREDITS: u32 = 500;
pub const JUMP_FUEL_COST: f32 = 25.0;
pub const DRAG: f32 = 0.4;
pub const SHIELD_REGEN: f32 = 4.0;
pub const SHIP_RADIUS: f32 = 14.0;
pub const PLANET_RADIUS: f32 = 90.0;
pub const PLANET_OFFSET_X: f32 = 500.0;
pub const LAND_RANGE: f32 = 220.0;
pub const PIRATE_SIGHT_RANGE: f32 = 900.0;
pub const PIRATE_FIRE_RANGE: f32 = 300.0;
pub const PIRATE_RESPAWN_SECS: f32 = 15.0;
pub const PLAYER_RESPAWN_SECS: f32 = 3.0;
pub const FUEL_PRICE_PER_UNIT: f32 = 1.0;

#[derive(Component)]
pub struct Ship(pub ShipState);

#[derive(Component)]
struct ShipSpec {
    flight: FlightModel,
    cargo_capacity: u32,
}

#[derive(Component)]
struct Blaster {
    def: WeaponDef,
    state: WeaponState,
}

#[derive(Component, Default)]
struct Control {
    throttle: f32,
    turn: f32,
    firing: bool,
}

#[derive(Component)]
struct Owner(u64);

#[derive(Component)]
struct Npc;

#[derive(Component)]
struct PlayerShip;

#[derive(Component)]
struct ClientRef(Entity);

#[derive(Component)]
struct Wallet(pub Account);

#[derive(Component)]
struct ProjectileMotion {
    speed: f32,
}

#[derive(Component)]
struct Aggro(u64);

#[derive(Component, PartialEq)]
#[component(immutable)]
pub struct InSystem(pub String);

#[derive(Component, PartialEq)]
#[component(immutable)]
pub struct ClientSystem(pub String);

impl VisibilityFilter for InSystem {
    type ClientComponent = ClientSystem;
    type Scope = Entity;

    fn is_visible(&self, _client: Entity, component: Option<&ClientSystem>) -> bool {
        component.is_some_and(|system| system.0 == self.0)
    }
}

#[derive(Resource, Default)]
struct PlayerRecord {
    account: Option<Account>,
    owner: Option<u64>,
}

#[derive(Resource, Default)]
struct PlayerRespawn(Option<f32>);

#[derive(Resource, Default)]
struct PirateRespawn(Option<f32>);

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_visibility_filter::<InSystem>()
            .init_resource::<PlayerRecord>()
            .init_resource::<PlayerRespawn>()
            .init_resource::<PirateRespawn>()
            .add_observer(on_set_throttle)
            .add_observer(on_set_turn)
            .add_observer(on_set_firing)
            .add_observer(on_request_land)
            .add_observer(on_request_lift_off)
            .add_observer(on_buy_commodity)
            .add_observer(on_sell_commodity)
            .add_observer(on_request_hyperjump)
            .add_systems(OnEnter(ServerState::Running), spawn_world)
            .add_systems(
                Update,
                (
                    authorize_clients,
                    spawn_player_on_connect,
                    apply_ship_control,
                    pirate_ai,
                    fire_weapons,
                    projectile_flight,
                    projectile_hits,
                    shield_regen,
                    ship_deaths,
                    respawn_players,
                    respawn_pirates,
                    sync_ship_state,
                    sync_accounts,
                )
                    .chain(),
            );
    }
}

fn spawn_ship(
    commands: &mut Commands,
    pack: &ContentPack,
    ship_id: &str,
    kind: ShipKind,
    owner: Option<u64>,
    system: &str,
    offset: Vec2,
) -> Entity {
    let def = pack.ship(ship_id).expect("ship must exist in pack");
    let sys = pack.system(system).expect("system must exist in pack");
    let base = Vec2::new(sys.position.0, sys.position.1);
    let mut ship_state = ShipState::new(def.max_shields, def.max_armor, def.fuel as f32);
    ship_state.position = base + offset;

    let mut entity = commands.spawn((
        Replicated,
        Ship(ship_state),
        ShipSpec {
            flight: FlightModel {
                turn_rate: def.turn_rate,
                acceleration: def.acceleration,
                max_speed: def.max_speed,
                drag: DRAG,
            },
            cargo_capacity: def.cargo,
        },
        Control::default(),
        Owner(owner.unwrap_or(0)),
        Position(base + offset),
        Heading(0.0),
        Hull {
            shields: def.max_shields,
            max_shields: def.max_shields,
            armor: def.max_armor,
            max_armor: def.max_armor,
        },
        Fuel {
            current: def.fuel as f32,
            max: def.fuel as f32,
        },
        ShipInfo {
            def_id: def.id.clone(),
            kind,
            owner,
            name: def.name.clone(),
        },
        SystemMembership(system.to_owned()),
        InSystem(system.to_owned()),
    ));

    if kind == ShipKind::Player {
        entity.insert(PlayerShip);
    }
    if kind == ShipKind::Pirate {
        entity.insert(Npc);
    }
    if let Some(outfit) = def
        .default_outfits
        .first()
        .and_then(|outfit_id| pack.outfit(outfit_id))
    {
        match &outfit.kind {
            OutfitKind::Weapon(weapon) => {
                entity.insert(Blaster {
                    def: weapon.clone(),
                    state: WeaponState::new(),
                });
            }
        }
    }

    entity.id()
}

fn spawn_world(mut commands: Commands, content: Res<Content>) {
    let pack = &content.0;
    for system in &pack.systems {
        for (index, planet) in system.planets.iter().enumerate() {
            commands.spawn((
                Replicated,
                PlanetTag {
                    system: system.id.clone(),
                    id: planet.id.clone(),
                    name: planet.name.clone(),
                    radius: PLANET_RADIUS,
                },
                Position(Vec2::new(
                    system.position.0 + PLANET_OFFSET_X + index as f32 * 600.0,
                    system.position.1,
                )),
                InSystem(system.id.clone()),
            ));
        }
    }
    spawn_ship(
        &mut commands,
        pack,
        PIRATE_SHIP,
        ShipKind::Pirate,
        None,
        PIRATE_SYSTEM,
        Vec2::new(0.0, -400.0),
    );
}

fn raw_id(client_id: ClientId, clients: &Query<&NetworkId>) -> u64 {
    match client_id.entity() {
        Some(entity) => clients.get(entity).map(|id| id.get()).unwrap_or(0),
        None => 0,
    }
}

fn notice(commands: &mut Commands, client: ClientId, text: impl Into<String>) {
    commands.server_trigger(ToClients {
        targets: SendTargets::Single(client),
        message: Notice { text: text.into() },
    });
}

fn spawn_player_on_connect(
    mut commands: Commands,
    content: Res<Content>,
    mut record: ResMut<PlayerRecord>,
    connected: Query<(Entity, &NetworkId), Added<ConnectedClient>>,
) {
    for (client_entity, network_id) in &connected {
        let owner = network_id.get();
        let account = record
            .account
            .take()
            .unwrap_or_else(|| Account::new(STARTING_CREDITS));
        let capacity = content
            .0
            .ship(PLAYER_SHIP)
            .expect("player ship must exist in pack")
            .cargo;
        let ship = spawn_ship(
            &mut commands,
            &content.0,
            PLAYER_SHIP,
            ShipKind::Player,
            Some(owner),
            PLAYER_SYSTEM,
            Vec2::ZERO,
        );
        commands.entity(ship).insert((
            Wallet(account.clone()),
            AccountState {
                credits: account.credits,
                cargo: account.cargo.clone(),
                cargo_capacity: capacity,
            },
            ClientRef(client_entity),
        ));
    }
}

fn authorize_clients(
    mut commands: Commands,
    newly_authorized: Query<Entity, Added<AuthorizedClient>>,
) {
    for client_entity in &newly_authorized {
        commands
            .entity(client_entity)
            .insert(ClientSystem(PLAYER_SYSTEM.to_owned()));
    }
}

fn sanitize_input(value: f32, min: f32, max: f32) -> Option<f32> {
    if value.is_finite() {
        Some(value.clamp(min, max))
    } else {
        None
    }
}

fn on_set_throttle(
    trigger: On<FromClient<SetThrottle>>,
    clients: Query<&NetworkId>,
    mut ships: Query<(&Owner, &mut Control)>,
) {
    let owner = raw_id(trigger.client_id, &clients);
    let Some(throttle) = sanitize_input(trigger.event().0, 0.0, 1.0) else {
        return;
    };
    for (ship_owner, mut control) in &mut ships {
        if ship_owner.0 == owner {
            control.throttle = throttle;
        }
    }
}

fn on_set_turn(
    trigger: On<FromClient<SetTurn>>,
    clients: Query<&NetworkId>,
    mut ships: Query<(&Owner, &mut Control)>,
) {
    let owner = raw_id(trigger.client_id, &clients);
    let Some(turn) = sanitize_input(trigger.event().0, -1.0, 1.0) else {
        return;
    };
    for (ship_owner, mut control) in &mut ships {
        if ship_owner.0 == owner {
            control.turn = turn;
        }
    }
}

fn on_set_firing(
    trigger: On<FromClient<SetFiring>>,
    clients: Query<&NetworkId>,
    mut ships: Query<(&Owner, &mut Control)>,
) {
    let owner = raw_id(trigger.client_id, &clients);
    let firing = trigger.event().0;
    for (ship_owner, mut control) in &mut ships {
        if ship_owner.0 == owner {
            control.firing = firing;
        }
    }
}

fn apply_ship_control(
    time: Res<Time>,
    mut ships: Query<(&Control, &ShipSpec, &mut Ship), Without<DockedAt>>,
) {
    let dt = time.delta_secs();
    for (control, spec, mut ship) in &mut ships {
        ship.0
            .apply_control(control.turn, control.throttle, &spec.flight, dt);
    }
}

type NpcShips<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Position,
        &'static Heading,
        &'static SystemMembership,
        &'static mut Control,
        Option<&'static Aggro>,
    ),
    With<Npc>,
>;

fn pirate_ai(
    mut commands: Commands,
    mut pirates: NpcShips,
    players: Query<(Entity, &Position, &SystemMembership, &Owner), With<PlayerShip>>,
) {
    for (entity, position, heading, membership, mut control, aggro) in &mut pirates {
        control.throttle = 0.0;
        control.turn = 0.0;
        control.firing = false;

        let target = match aggro {
            Some(Aggro(wanted_owner)) => {
                players
                    .iter()
                    .find(|(_, _, target_membership, target_owner)| {
                        target_owner.0 == *wanted_owner && target_membership.0 == membership.0
                    })
            }
            None => players
                .iter()
                .find(|(_, _, target_membership, _)| target_membership.0 == membership.0),
        };
        let Some((_, target, _, _)) = target else {
            if aggro.is_some() {
                commands.entity(entity).despawn();
            }
            continue;
        };
        let delta = target.0 - position.0;
        let distance = delta.length();
        if distance > PIRATE_SIGHT_RANGE {
            continue;
        }

        let desired = delta.y.atan2(delta.x);
        let diff = (desired - heading.0 + PI).rem_euclid(2.0 * PI) - PI;
        control.turn = diff.signum();
        control.throttle = if distance > 200.0 { 1.0 } else { 0.3 };
        control.firing = distance < PIRATE_FIRE_RANGE && diff.abs() < 0.35;
    }
}

type FiringShips<'w, 's> = Query<
    'w,
    's,
    (
        &'static Control,
        &'static Owner,
        &'static Ship,
        &'static SystemMembership,
        &'static mut Blaster,
    ),
    Without<DockedAt>,
>;

fn fire_weapons(mut commands: Commands, time: Res<Time>, mut ships: FiringShips) {
    let dt = time.delta_secs();
    for (control, owner, ship, membership, mut blaster) in &mut ships {
        blaster.state.tick(dt);
        let cooldown = blaster.def.cooldown;
        if control.firing && blaster.state.try_fire(cooldown) {
            let direction = ship.0.heading();
            commands.spawn((
                Replicated,
                Projectile {
                    damage: blaster.def.damage,
                    ttl: blaster.def.projectile_ttl,
                    owner: owner.0,
                },
                ProjectileMotion {
                    speed: blaster.def.projectile_speed,
                },
                Position(ship.0.position + direction * SHIP_RADIUS),
                Heading(ship.0.rotation),
                SystemMembership(membership.0.clone()),
                InSystem(membership.0.clone()),
            ));
        }
    }
}

fn projectile_flight(
    mut commands: Commands,
    time: Res<Time>,
    mut projectiles: Query<(
        Entity,
        &Heading,
        &mut Position,
        &mut Projectile,
        &ProjectileMotion,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, heading, mut position, mut projectile, motion) in &mut projectiles {
        let direction = Vec2::new(heading.0.cos(), heading.0.sin());
        position.0 += direction * motion.speed * dt;
        projectile.ttl -= dt;
        if projectile.ttl <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn projectile_hits(
    mut commands: Commands,
    content: Res<Content>,
    projectiles: Query<(Entity, &Position, &Projectile, &SystemMembership)>,
    aggros: Query<&Aggro>,
    ships: Query<(Entity, &ShipInfo, &SystemMembership, Option<&ClientRef>), With<Ship>>,
    mut hittable: Query<(&mut Ship, Option<&mut Wallet>)>,
) {
    let mut hits = Vec::new();
    for (projectile_entity, position, projectile, projectile_membership) in &projectiles {
        for (ship_entity, info, ship_membership, _) in &ships {
            let is_shooter = info.owner == Some(projectile.owner)
                || (projectile.owner == 0 && info.kind != ShipKind::Player);
            if is_shooter || ship_membership.0 != projectile_membership.0 {
                continue;
            }
            if let Ok((ship, _)) = hittable.get(ship_entity)
                && position.0.distance(ship.0.position) <= SHIP_RADIUS
            {
                hits.push((
                    projectile_entity,
                    ship_entity,
                    projectile.damage,
                    projectile.owner,
                ));
                break;
            }
        }
    }

    for (projectile_entity, victim_entity, damage, attacker_owner) in hits {
        commands.entity(projectile_entity).despawn();
        let Ok((mut victim_ship, _)) = hittable.get_mut(victim_entity) else {
            continue;
        };
        victim_ship.0.apply_damage(damage);

        let Ok((_, victim_info, victim_membership, _)) = ships.get(victim_entity) else {
            continue;
        };
        let is_pvp = victim_info.kind == ShipKind::Player
            && attacker_owner != 0
            && victim_info.owner != Some(attacker_owner);
        if !is_pvp {
            continue;
        }
        let policed = content
            .0
            .system(&victim_membership.0)
            .is_some_and(|system| system.government == POLICED_GOVERNMENT);
        if !policed {
            continue;
        }

        let Some((attacker_entity, _, _, attacker_client_ref)) = ships
            .iter()
            .find(|(_, info, _, _)| info.owner == Some(attacker_owner))
        else {
            continue;
        };
        let police_exists = aggros.iter().any(|aggro| aggro.0 == attacker_owner);
        if police_exists {
            continue;
        }
        let police = spawn_ship(
            &mut commands,
            &content.0,
            POLICE_SHIP,
            ShipKind::Police,
            None,
            &victim_membership.0,
            Vec2::new(250.0, 250.0),
        );
        commands.entity(police).insert((Npc, Aggro(attacker_owner)));
        if let Ok((_, Some(mut wallet))) = hittable.get_mut(attacker_entity) {
            let fine = wallet.0.credits.min(FINE_CREDITS);
            wallet.0.credits -= fine;
            if let Some(client_ref) = attacker_client_ref {
                notice(
                    &mut commands,
                    ClientId::Client(client_ref.0),
                    format!("Customs drones: {fine} cr fine for unlawful weapons discharge."),
                );
            }
        }
        if let Some(client_ref) = attacker_client_ref {
            notice(
                &mut commands,
                ClientId::Client(client_ref.0),
                "A Confederation patrol locks onto you.",
            );
        }
    }
}

fn shield_regen(time: Res<Time>, mut ships: Query<&mut Ship>) {
    let dt = time.delta_secs();
    for mut ship in &mut ships {
        ship.0.regen_shields(SHIELD_REGEN, dt);
    }
}

type DyingShips<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Ship,
        &'static ShipInfo,
        &'static SystemMembership,
        &'static Owner,
        Option<&'static ClientRef>,
        Option<&'static Wallet>,
    ),
>;

fn ship_deaths(
    mut commands: Commands,
    mut record: ResMut<PlayerRecord>,
    mut player_respawn: ResMut<PlayerRespawn>,
    mut pirate_respawn: ResMut<PirateRespawn>,
    ships: DyingShips,
    clients: Query<(Entity, &ClientSystem), With<ConnectedClient>>,
) {
    for (entity, ship, info, membership, owner, client_ref, wallet) in &ships {
        if !ship.0.is_destroyed() {
            continue;
        }
        let explosion = Explosion {
            position: (ship.0.position.x, ship.0.position.y),
            system: membership.0.clone(),
        };
        for (client_entity, client_system) in &clients {
            if client_system.0 == membership.0 {
                commands.server_trigger(ToClients {
                    targets: SendTargets::Single(ClientId::Client(client_entity)),
                    message: explosion.clone(),
                });
            }
        }
        match info.kind {
            ShipKind::Player => {
                if let Some(wallet) = wallet {
                    record.account = Some(Account::new(wallet.0.credits));
                }
                record.owner = Some(owner.0);
                player_respawn.0 = Some(PLAYER_RESPAWN_SECS);
                if let Some(client_ref) = client_ref {
                    commands.server_trigger(ToClients {
                        targets: SendTargets::Single(ClientId::Client(client_ref.0)),
                        message: PlayerDied,
                    });
                }
                commands.entity(entity).despawn();
            }
            ShipKind::Pirate => {
                pirate_respawn.0 = Some(PIRATE_RESPAWN_SECS);
                commands.entity(entity).despawn();
            }
            ShipKind::Police => {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn respawn_players(
    mut commands: Commands,
    time: Res<Time>,
    content: Res<Content>,
    mut record: ResMut<PlayerRecord>,
    mut respawn: ResMut<PlayerRespawn>,
    clients: Query<(Entity, &NetworkId), With<ConnectedClient>>,
) {
    let Some(timer) = respawn.0.as_mut() else {
        return;
    };
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    respawn.0 = None;

    let Some(owner) = record.owner else {
        return;
    };
    let Some((client_entity, _)) = clients.iter().find(|(_, id)| id.get() == owner) else {
        respawn.0 = Some(0.5);
        return;
    };
    record.owner = None;
    let account = record
        .account
        .take()
        .unwrap_or_else(|| Account::new(STARTING_CREDITS));
    let capacity = content
        .0
        .ship(PLAYER_SHIP)
        .expect("player ship must exist in pack")
        .cargo;

    let ship = spawn_ship(
        &mut commands,
        &content.0,
        PLAYER_SHIP,
        ShipKind::Player,
        Some(owner),
        PLAYER_SYSTEM,
        Vec2::ZERO,
    );
    commands
        .entity(client_entity)
        .insert(ClientSystem(PLAYER_SYSTEM.to_owned()));
    commands.entity(ship).insert((
        Wallet(account.clone()),
        AccountState {
            credits: account.credits,
            cargo: account.cargo.clone(),
            cargo_capacity: capacity,
        },
        ClientRef(client_entity),
    ));
}

fn respawn_pirates(
    mut commands: Commands,
    time: Res<Time>,
    content: Res<Content>,
    mut respawn: ResMut<PirateRespawn>,
) {
    let Some(timer) = respawn.0.as_mut() else {
        return;
    };
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    respawn.0 = None;
    spawn_ship(
        &mut commands,
        &content.0,
        PIRATE_SHIP,
        ShipKind::Pirate,
        None,
        PIRATE_SYSTEM,
        Vec2::new(0.0, -400.0),
    );
}

fn sync_ship_state(mut ships: Query<(&Ship, &mut Position, &mut Heading, &mut Hull, &mut Fuel)>) {
    for (ship, mut position, mut heading, mut hull, mut fuel) in &mut ships {
        position.0 = ship.0.position;
        heading.0 = ship.0.rotation;
        hull.shields = ship.0.shields;
        hull.armor = ship.0.armor;
        fuel.current = ship.0.fuel;
    }
}

fn sync_accounts(mut players: Query<(&Wallet, &mut AccountState)>) {
    for (wallet, mut state) in &mut players {
        if state.credits != wallet.0.credits || state.cargo != wallet.0.cargo {
            state.credits = wallet.0.credits;
            state.cargo = wallet.0.cargo.clone();
        }
    }
}

type LandingShips<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Owner,
        &'static Position,
        &'static SystemMembership,
        Option<&'static DockedAt>,
        &'static mut Ship,
        &'static mut Wallet,
    ),
>;

fn on_request_land(
    trigger: On<FromClient<RequestLand>>,
    mut commands: Commands,
    clients: Query<&NetworkId>,
    mut ships: LandingShips,
    planets: Query<(&PlanetTag, &Position)>,
) {
    let owner = raw_id(trigger.client_id, &clients);
    let planet_id = &trigger.event().planet;

    let Some((ship_entity, _, ship_position, membership, docked, mut ship, mut wallet)) = ships
        .iter_mut()
        .find(|(_, ship_owner, _, _, _, _, _)| ship_owner.0 == owner)
    else {
        return;
    };
    if docked.is_some() {
        notice(&mut commands, trigger.client_id, "You are already docked.");
        return;
    }
    let Some((planet, planet_position)) = planets
        .iter()
        .find(|(tag, _)| tag.id == *planet_id && tag.system == membership.0)
    else {
        notice(
            &mut commands,
            trigger.client_id,
            "No planet answers by that name.",
        );
        return;
    };
    if ship_position.0.distance(planet_position.0) > LAND_RANGE + planet.radius {
        notice(
            &mut commands,
            trigger.client_id,
            "You are not close enough to land.",
        );
        return;
    }

    ship.0.velocity = Vec2::ZERO;
    commands.entity(ship_entity).insert(DockedAt {
        system: membership.0.clone(),
        planet: planet.id.clone(),
    });

    let missing = ship.0.max_fuel - ship.0.fuel;
    let cost = (missing * FUEL_PRICE_PER_UNIT).ceil() as u32;
    if cost == 0 {
        notice(
            &mut commands,
            trigger.client_id,
            format!("Docked at {}. Tanks already full.", planet.name),
        );
    } else if wallet.0.credits >= cost {
        wallet.0.credits -= cost;
        ship.0.fuel = ship.0.max_fuel;
        notice(
            &mut commands,
            trigger.client_id,
            format!("Docked at {}. Refuel: {cost} cr.", planet.name),
        );
    } else {
        notice(
            &mut commands,
            trigger.client_id,
            format!(
                "Docked at {}. The fuel clerk wants cash you don't have.",
                planet.name
            ),
        );
    }
}

fn on_request_lift_off(
    trigger: On<FromClient<RequestLiftOff>>,
    mut commands: Commands,
    clients: Query<&NetworkId>,
    ships: Query<(Entity, &Owner, &DockedAt)>,
) {
    let owner = raw_id(trigger.client_id, &clients);
    let Some((ship_entity, _, _)) = ships
        .iter()
        .find(|(_, ship_owner, _)| ship_owner.0 == owner)
    else {
        return;
    };
    commands.entity(ship_entity).remove::<DockedAt>();
}

fn on_buy_commodity(
    trigger: On<FromClient<BuyCommodity>>,
    mut commands: Commands,
    clients: Query<&NetworkId>,
    content: Res<Content>,
    mut ships: Query<(&Owner, &DockedAt, &ShipSpec, &mut Wallet, &mut Ship)>,
) {
    let owner = raw_id(trigger.client_id, &clients);
    let event = trigger.event();
    let Some((_, docked, spec, mut wallet, _)) = ships
        .iter_mut()
        .find(|(ship_owner, _, _, _, _)| ship_owner.0 == owner)
    else {
        return;
    };
    let Some(price) = planet_price(&content.0, &docked.system, &docked.planet, &event.commodity)
    else {
        notice(
            &mut commands,
            trigger.client_id,
            format!("The market master has never heard of {}.", event.commodity),
        );
        return;
    };
    if event.qty == 0 {
        notice(
            &mut commands,
            trigger.client_id,
            "The broker stares at your empty order.",
        );
        return;
    }
    let Some(total) = price.checked_mul(event.qty) else {
        notice(
            &mut commands,
            trigger.client_id,
            "Nobody has that many credits.",
        );
        return;
    };
    if wallet.0.credits < total {
        notice(&mut commands, trigger.client_id, "You can't afford that.");
        return;
    }
    if !wallet
        .0
        .add_cargo(&event.commodity, event.qty, spec.cargo_capacity)
    {
        notice(&mut commands, trigger.client_id, "Your cargo hold is full.");
        return;
    }
    wallet.0.credits -= total;
    notice(
        &mut commands,
        trigger.client_id,
        format!("Bought {} {} for {total} cr.", event.qty, event.commodity),
    );
}

fn on_sell_commodity(
    trigger: On<FromClient<SellCommodity>>,
    mut commands: Commands,
    clients: Query<&NetworkId>,
    content: Res<Content>,
    mut ships: Query<(&Owner, &DockedAt, &mut Wallet)>,
) {
    let owner = raw_id(trigger.client_id, &clients);
    let event = trigger.event();
    let Some((_, docked, mut wallet)) = ships
        .iter_mut()
        .find(|(ship_owner, _, _)| ship_owner.0 == owner)
    else {
        return;
    };
    let Some(price) = planet_price(&content.0, &docked.system, &docked.planet, &event.commodity)
    else {
        notice(
            &mut commands,
            trigger.client_id,
            format!("The market master has never heard of {}.", event.commodity),
        );
        return;
    };
    if event.qty == 0 {
        notice(
            &mut commands,
            trigger.client_id,
            "The broker stares at your empty order.",
        );
        return;
    }
    if !wallet.0.remove_cargo(&event.commodity, event.qty) {
        notice(
            &mut commands,
            trigger.client_id,
            format!("You don't have {} {}.", event.qty, event.commodity),
        );
        return;
    }
    let Some(total) = price.checked_mul(event.qty) else {
        notice(
            &mut commands,
            trigger.client_id,
            "Nobody has that many credits.",
        );
        return;
    };
    wallet.0.credits += total;
    notice(
        &mut commands,
        trigger.client_id,
        format!("Sold {} {} for {total} cr.", event.qty, event.commodity),
    );
}

fn on_request_hyperjump(
    trigger: On<FromClient<RequestHyperjump>>,
    mut commands: Commands,
    clients: Query<&NetworkId>,
    client_entities: Query<Entity, With<ConnectedClient>>,
    content: Res<Content>,
    mut ships: Query<(
        Entity,
        &Owner,
        Option<&DockedAt>,
        &mut Ship,
        &mut SystemMembership,
    )>,
) {
    let owner = raw_id(trigger.client_id, &clients);
    let target = trigger.event().target.clone();
    let Some((ship_entity, _, docked, mut ship, mut membership)) = ships
        .iter_mut()
        .find(|(_, ship_owner, _, _, _)| ship_owner.0 == owner)
    else {
        return;
    };
    if docked.is_some() {
        notice(&mut commands, trigger.client_id, "Undock before jumping.");
        return;
    }
    let Some(current) = content.0.system(&membership.0) else {
        return;
    };
    if !current.links.contains(&target) {
        notice(
            &mut commands,
            trigger.client_id,
            "No hyperlane leads there.",
        );
        return;
    }
    if ship.0.fuel < JUMP_FUEL_COST {
        notice(
            &mut commands,
            trigger.client_id,
            "Your fuel gauge laughs at the idea.",
        );
        return;
    }
    let Some(target_system) = content.0.system(&target) else {
        return;
    };

    ship.0.fuel -= JUMP_FUEL_COST;
    ship.0.velocity = Vec2::ZERO;
    ship.0.position = Vec2::new(target_system.position.0, target_system.position.1 + 300.0);
    membership.0 = target.clone();
    commands
        .entity(ship_entity)
        .insert(InSystem(target.clone()));
    if let Some(client_entity) = trigger
        .client_id
        .entity()
        .filter(|entity| client_entities.contains(*entity))
    {
        commands
            .entity(client_entity)
            .insert(ClientSystem(target.clone()));
    }
    notice(
        &mut commands,
        trigger.client_id,
        format!("Hyperjump to {} complete.", target_system.name),
    );
}

fn planet_price(pack: &ContentPack, system: &str, planet: &str, commodity: &str) -> Option<u32> {
    let system = pack.system(system)?;
    let planet = system.planets.iter().find(|p| p.id == planet)?;
    let entry = planet
        .commodities
        .iter()
        .find(|c| c.id == commodity)
        .map(|c| c.price)?;
    Some(entry)
}
#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use bevy::app::PostUpdate;
    use bevy::math::Vec2;
    use bevy::state::app::StatesPlugin;
    use bevy_replicon_renet2::netcode::{
        ClientAuthentication, NativeSocket, NetcodeClientTransport,
    };
    use bevy_replicon_renet2::renet2::{ConnectionConfig, RenetClient};
    use bevy_replicon_renet2::{RenetChannelsExt, RepliconRenetPlugins};
    use evr_protocol::AccountState;

    use super::*;
    use crate::{
        EMBEDDED_CLIENT_ID, PROTOCOL_ID, TickMode, attach_memory, attach_udp, build_server_app,
    };

    struct Harness {
        server: App,
        client: App,
        steps: usize,
    }

    impl Harness {
        fn new() -> Self {
            let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content");
            let mut server =
                build_server_app(content_root, TickMode::PerUpdate).expect("server app builds");
            let transport = attach_memory(&mut server);
            server.finish();

            let mut client = App::new();
            client.add_plugins(MinimalPlugins);
            client.add_plugins(StatesPlugin);
            client.add_plugins(RepliconPlugins.set(ServerPlugin::new(PostUpdate)));
            client.add_plugins(RepliconRenetPlugins);
            evr_protocol::register(&mut client);

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
                steps: 0,
            };
            while !harness
                .client
                .world()
                .resource::<RenetClient>()
                .is_connected()
            {
                harness.step();
                assert!(harness.steps < 300, "client must connect");
            }
            harness
        }

        fn step(&mut self) {
            self.server.update();
            self.client.update();
            std::thread::sleep(Duration::from_millis(15));
            self.steps += 1;
        }

        fn step_until(&mut self, max_steps: usize, mut cond: impl FnMut(&mut Self) -> bool) {
            let start = self.steps;
            while !cond(self) {
                assert!(
                    self.steps - start < max_steps,
                    "condition not met within {max_steps} steps"
                );
                self.step();
            }
        }

        fn my_ship(&mut self) -> Option<Entity> {
            let mut state = self
                .client
                .world_mut()
                .query_filtered::<(Entity, &ShipInfo), With<Remote>>();
            let world = self.client.world();
            state
                .iter(world)
                .find(|(_, info)| info.owner == Some(EMBEDDED_CLIENT_ID as u64))
                .map(|(entity, _)| entity)
        }

        fn pirate_ship(&mut self) -> Option<Entity> {
            let mut state = self
                .client
                .world_mut()
                .query_filtered::<(Entity, &ShipInfo), With<Remote>>();
            let world = self.client.world();
            state
                .iter(world)
                .find(|(_, info)| info.kind == ShipKind::Pirate)
                .map(|(entity, _)| entity)
        }

        fn client_projectile_count(&mut self) -> usize {
            let mut state = self
                .client
                .world_mut()
                .query_filtered::<(), (With<Remote>, With<Projectile>)>();
            let world = self.client.world();
            state.iter(world).count()
        }

        fn client_planet_ids(&mut self) -> Vec<String> {
            let mut state = self
                .client
                .world_mut()
                .query_filtered::<&PlanetTag, With<Remote>>();
            let world = self.client.world();
            state.iter(world).map(|tag| tag.id.clone()).collect()
        }

        fn client_component<T: Component + Clone>(&mut self, entity: Entity) -> Option<T> {
            self.client.world().get::<T>(entity).cloned()
        }

        fn server_ship_position(&mut self, owner: u64) -> Option<Vec2> {
            let mut state = self.server.world_mut().query::<(&Owner, &Ship)>();
            let world = self.server.world();
            state
                .iter(world)
                .find(|(o, _)| o.0 == owner)
                .map(|(_, ship)| ship.0.position)
        }

        fn set_server_ship(&mut self, owner: u64, mutate: impl Fn(&mut ShipState)) {
            let mut state = self.server.world_mut().query::<(&Owner, &mut Ship)>();
            let world = self.server.world_mut();
            for (o, mut ship) in state.iter_mut(world) {
                if o.0 == owner {
                    mutate(&mut ship.0);
                }
            }
        }

        fn send<E: Event>(&mut self, event: E) {
            self.client.world_mut().client_trigger(event);
        }
    }

    #[test]
    fn embedded_vertical_slice() {
        let mut h = Harness::new();

        h.step_until(200, |h| {
            let mut planets = h
                .client
                .world_mut()
                .query_filtered::<(), (With<Remote>, With<PlanetTag>)>();
            let world = h.client.world();
            planets.iter(world).count() == 1 && h.my_ship().is_some()
        });
        for _ in 0..20 {
            h.step();
        }
        assert!(
            h.pirate_ship().is_none(),
            "pirate must be hidden while the player is in sol"
        );

        let my_ship = h.my_ship().expect("my ship must replicate");
        let account = h
            .client_component::<AccountState>(my_ship)
            .expect("account must replicate");
        assert_eq!(account.credits, STARTING_CREDITS);
        assert_eq!(account.cargo_capacity, 20);
        let hull = h
            .client_component::<Hull>(my_ship)
            .expect("hull replicates");
        assert_eq!(hull.shields, hull.max_shields);
        let fuel = h
            .client_component::<Fuel>(my_ship)
            .expect("fuel replicates");
        assert_eq!(fuel.current, fuel.max);

        let start = h.server_ship_position(1).expect("player exists");
        h.send(SetThrottle(1.0));
        h.step_until(200, |h| {
            h.server_ship_position(1)
                .is_some_and(|p| p.x > start.x + 40.0)
        });
        h.send(SetThrottle(0.0));
        h.send(SetTurn(1.0));
        h.step_until(200, |h| {
            h.client_component::<evr_protocol::Heading>(my_ship)
                .is_some_and(|heading| heading.0 > 0.5)
        });
        h.send(SetTurn(0.0));

        h.set_server_ship(1, |ship| {
            ship.position = Vec2::new(480.0, 0.0);
            ship.velocity = Vec2::ZERO;
        });
        h.send(RequestLand {
            planet: "earth".into(),
        });
        h.step_until(100, |h| h.client_component::<DockedAt>(my_ship).is_some());
        let docked = h.client_component::<DockedAt>(my_ship).unwrap();
        assert_eq!(docked.planet, "earth");
        assert_eq!(docked.system, PLAYER_SYSTEM);

        h.send(BuyCommodity {
            commodity: "food".into(),
            qty: 5,
        });
        h.step_until(100, |h| {
            h.client_component::<AccountState>(my_ship)
                .is_some_and(|account| account.credits == STARTING_CREDITS - 40)
        });
        h.send(SellCommodity {
            commodity: "food".into(),
            qty: 5,
        });
        h.step_until(100, |h| {
            h.client_component::<AccountState>(my_ship)
                .is_some_and(|account| account.credits == STARTING_CREDITS)
        });

        h.send(RequestLiftOff);
        h.step_until(100, |h| h.client_component::<DockedAt>(my_ship).is_none());

        h.send(RequestHyperjump {
            target: "alpha-centauri".into(),
        });
        h.step_until(100, |h| {
            h.client_component::<SystemMembership>(my_ship)
                .is_some_and(|m| m.0 == "alpha-centauri")
        });
        let fuel = h.client_component::<Fuel>(my_ship).unwrap();
        assert!((fuel.current - (fuel.max - JUMP_FUEL_COST)).abs() < 0.01);
        let position = h.client_component::<Position>(my_ship).unwrap();
        assert_eq!(position.0, Vec2::new(200.0, 300.0));

        h.step_until(100, |h| {
            h.client_planet_ids() == vec!["newhaven".to_owned()]
        });

        h.set_server_ship(1, |ship| {
            ship.rotation = 0.0;
            ship.velocity = Vec2::ZERO;
        });
        h.set_server_ship(0, |ship| {
            ship.shields = 6.0;
            ship.armor = 4.0;
            ship.position = Vec2::new(260.0, 300.0);
            ship.velocity = Vec2::ZERO;
        });
        h.send(SetFiring(true));
        let mut saw_projectiles = false;
        h.step_until(300, |h| {
            if h.client_projectile_count() > 0 {
                saw_projectiles = true;
            }
            h.pirate_ship().is_none()
        });
        h.send(SetFiring(false));
        assert!(saw_projectiles, "client must see projectiles in flight");
        assert!(
            !h.client_component::<AccountState>(my_ship)
                .unwrap()
                .cargo
                .iter()
                .any(|(id, _)| id == "food")
        );

        h.set_server_ship(1, |ship| {
            ship.shields = 0.0;
            ship.armor = 0.0;
        });
        h.step_until(100, |h| h.my_ship().is_none());
        h.step_until(500, |h| h.my_ship().is_some());
        let respawned = h.my_ship().unwrap();
        let account = h.client_component::<AccountState>(respawned).unwrap();
        assert_eq!(account.credits, STARTING_CREDITS);
        let hull = h.client_component::<Hull>(respawned).unwrap();
        assert_eq!(hull.shields, hull.max_shields);
        let membership = h.client_component::<SystemMembership>(respawned).unwrap();
        assert_eq!(membership.0, PLAYER_SYSTEM);
    }

    fn udp_client_app(client_id: u64, server_addr: std::net::SocketAddr) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.add_plugins(RepliconPlugins.set(ServerPlugin::new(PostUpdate)));
        app.add_plugins(RepliconRenetPlugins);
        evr_protocol::register(&mut app);

        let (server_channels, client_channels) = {
            let channels = app.world().resource::<RepliconChannels>();
            (channels.server_configs(), channels.client_configs())
        };
        app.insert_resource(RenetClient::new(
            ConnectionConfig::from_channels(server_channels, client_channels),
            false,
        ));

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("system time must be after the epoch");
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").expect("test socket must bind");
        let transport = NetcodeClientTransport::new(
            current_time,
            ClientAuthentication::Unsecure {
                client_id,
                protocol_id: PROTOCOL_ID,
                socket_id: 0,
                server_addr,
                user_data: None,
            },
            NativeSocket::new(socket).expect("test socket must be valid"),
        )
        .expect("test transport must be valid");
        app.insert_resource(transport);
        app.finish();
        app
    }

    fn remote_ship_ids(client: &mut App) -> Vec<u64> {
        let mut state = client
            .world_mut()
            .query_filtered::<&ShipInfo, With<Remote>>();
        let world = client.world();
        let mut owners: Vec<u64> = state.iter(world).filter_map(|info| info.owner).collect();
        owners.sort_unstable();
        owners
    }

    fn remote_planet_ids(client: &mut App) -> Vec<String> {
        let mut state = client
            .world_mut()
            .query_filtered::<&PlanetTag, With<Remote>>();
        let world = client.world();
        state.iter(world).map(|tag| tag.id.clone()).collect()
    }

    fn server_police_target(server: &mut App) -> Option<u64> {
        let mut state = server.world_mut().query::<&Aggro>();
        let world = server.world();
        state.iter(world).next().map(|aggro| aggro.0)
    }

    fn server_credits(server: &mut App, owner: u64) -> Option<u32> {
        let mut state = server.world_mut().query::<(&Owner, &Wallet)>();
        let world = server.world();
        state
            .iter(world)
            .find(|(wallet_owner, _)| wallet_owner.0 == owner)
            .map(|(_, wallet)| wallet.0.credits)
    }

    #[test]
    fn udp_two_players_rooms_and_govt_zones() {
        let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let mut server =
            build_server_app(content_root, TickMode::PerUpdate).expect("server app must build");
        let bound = attach_udp(
            &mut server,
            std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
            8,
        )
        .expect("udp socket must bind");
        server.finish();

        let mut alice = udp_client_app(101, bound);
        let mut bob = udp_client_app(202, bound);

        let step_all = |server: &mut App, alice: &mut App, bob: &mut App| {
            alice.update();
            bob.update();
            server.update();
            std::thread::sleep(Duration::from_millis(15));
        };

        let mut steps = 0;
        while !alice.world().resource::<RenetClient>().is_connected()
            || !bob.world().resource::<RenetClient>().is_connected()
        {
            step_all(&mut server, &mut alice, &mut bob);
            steps += 1;
            assert!(steps < 400, "both clients must connect");
        }

        while remote_ship_ids(&mut alice) != vec![101, 202]
            || remote_ship_ids(&mut bob) != vec![101, 202]
        {
            step_all(&mut server, &mut alice, &mut bob);
            steps += 1;
            assert!(steps < 800, "both players must replicate");
        }

        assert_eq!(remote_planet_ids(&mut alice), vec!["earth".to_owned()]);
        assert_eq!(remote_planet_ids(&mut bob), vec!["earth".to_owned()]);

        {
            let mut pirates = alice
                .world_mut()
                .query_filtered::<&ShipInfo, With<Remote>>();
            let world = alice.world();
            assert!(
                pirates
                    .iter(world)
                    .all(|info| info.kind != ShipKind::Pirate),
                "the alpha-centauri pirate must be invisible from sol"
            );
        }

        let set_ship = |server: &mut App, owner: u64, position: Vec2, rotation: f32| {
            let mut state = server.world_mut().query::<(&Owner, &mut Ship)>();
            let world = server.world_mut();
            for (ship_owner, mut ship) in state.iter_mut(world) {
                if ship_owner.0 == owner {
                    ship.0.position = position;
                    ship.0.velocity = Vec2::ZERO;
                    ship.0.rotation = rotation;
                }
            }
        };
        set_ship(&mut server, 101, Vec2::new(0.0, 0.0), 0.0);
        set_ship(&mut server, 202, Vec2::new(200.0, 0.0), 0.0);
        for _ in 0..5 {
            step_all(&mut server, &mut alice, &mut bob);
        }

        alice.world_mut().client_trigger(SetFiring(true));
        let mut fired = 0;
        while server_police_target(&mut server).is_none() {
            step_all(&mut server, &mut alice, &mut bob);
            fired += 1;
            assert!(fired < 400, "pvp offense must trigger the patrol");
        }
        alice.world_mut().client_trigger(SetFiring(false));

        assert_eq!(server_police_target(&mut server), Some(101));
        assert_eq!(
            server_credits(&mut server, 101),
            Some(STARTING_CREDITS - FINE_CREDITS),
            "the attacker is fined once per offense"
        );
        assert_eq!(
            server_credits(&mut server, 202),
            Some(STARTING_CREDITS),
            "the victim keeps their credits"
        );
        let mut police_seen = false;
        for _ in 0..100 {
            step_all(&mut server, &mut alice, &mut bob);
            let mut ships = alice
                .world_mut()
                .query_filtered::<&ShipInfo, With<Remote>>();
            let world = alice.world();
            police_seen =
                police_seen || ships.iter(world).any(|info| info.kind == ShipKind::Police);
        }
        assert!(
            police_seen,
            "the police ship must replicate to both players"
        );

        alice.world_mut().client_trigger(RequestHyperjump {
            target: "alpha-centauri".to_owned(),
        });
        let mut jumped = 0;
        while remote_planet_ids(&mut alice) != vec!["newhaven".to_owned()]
            || server_police_target(&mut server).is_some()
        {
            step_all(&mut server, &mut alice, &mut bob);
            jumped += 1;
            assert!(jumped < 600, "alice must reach alpha centauri");
        }
        assert_eq!(
            remote_ship_ids(&mut bob),
            vec![202],
            "bob must no longer see alice after the hyperjump"
        );
        assert_eq!(remote_planet_ids(&mut bob), vec!["earth".to_owned()]);
        let mut pirates = alice
            .world_mut()
            .query_filtered::<&ShipInfo, With<Remote>>();
        let world = alice.world();
        assert!(
            pirates
                .iter(world)
                .any(|info| info.kind == ShipKind::Pirate),
            "alice must see the alpha centauri pirate now"
        );
    }
}
