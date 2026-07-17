//! Spawning, world scrolling, collision, and scoring.
//!
//! The penguin never moves forward; the world scrolls toward the camera at
//! [`RunSpeed`]. Each biome has its own obstacle and collectible tables
//! (GDD "Spawning"). Collision is manual AABB — no physics engine needed
//! for a lane runner.

use bevy::prelude::*;

use crate::audio::AudioCue;
use crate::chase::ObstacleHit;
use crate::player::{Player, Vertical};
use crate::states::{self, Altitude, Biome, RunState};
use crate::tuning::*;

/// Deterministic xorshift64 RNG seeded from the clock. Avoids the
/// rand/getrandom wasm glue for the handful of rolls a spawner needs.
#[derive(Resource)]
pub struct GameRng(u64);

impl Default for GameRng {
    fn default() -> Self {
        #[cfg(target_arch = "wasm32")]
        let seed = js_sys::Date::now() as u64;
        #[cfg(not(target_arch = "wasm32"))]
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Self(seed | 1)
    }
}

impl GameRng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u32 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.f32() * (hi - lo)
    }

    pub fn lane(&mut self) -> i8 {
        (self.next() % 3) as i8 - 1
    }

    pub fn chance(&mut self, probability: f32) -> bool {
        self.f32() < probability
    }
}

/// Forward speed of the run, including the post-hit slowdown window.
#[derive(Resource)]
pub struct RunSpeed {
    pub base: f32,
    slowdown: Timer,
}

impl Default for RunSpeed {
    fn default() -> Self {
        let mut slowdown = Timer::from_seconds(HIT_SLOWDOWN_SECS, TimerMode::Once);
        // Start with the slowdown already expired.
        slowdown.tick(std::time::Duration::from_secs_f32(HIT_SLOWDOWN_SECS));
        Self {
            base: RUN_SPEED_BASE,
            slowdown,
        }
    }
}

impl RunSpeed {
    pub fn current(&self) -> f32 {
        if self.slowdown.finished() {
            self.base
        } else {
            self.base * HIT_SLOWDOWN_FACTOR
        }
    }

    pub fn apply_hit_slowdown(&mut self) {
        self.slowdown.reset();
    }

    pub fn is_slowed(&self) -> bool {
        !self.slowdown.finished()
    }
}

/// Score: collectibles plus distance travelled.
#[derive(Resource, Default)]
pub struct Score {
    pub points: u32,
    distance_acc: f32,
}

/// Independent randomized timers per spawn category.
#[derive(Resource)]
pub struct SpawnTimers {
    obstacle: Timer,
    collectible: Timer,
    cannon: Timer,
    decor: Timer,
}

impl Default for SpawnTimers {
    fn default() -> Self {
        Self {
            obstacle: Timer::from_seconds(OBSTACLE_INTERVAL.1, TimerMode::Once),
            collectible: Timer::from_seconds(COLLECTIBLE_INTERVAL.0, TimerMode::Once),
            cannon: Timer::from_seconds(CANNON_INTERVAL.0, TimerMode::Once),
            decor: Timer::from_seconds(DECOR_INTERVAL.0, TimerMode::Once),
        }
    }
}

/// Anything that scrolls toward the camera and despawns behind it.
/// Also the cleanup marker for biome switches and restarts.
#[derive(Component)]
pub struct Mover;

/// Axis-aligned collision half-extents around the entity's translation.
#[derive(Component)]
pub struct Collider {
    pub half: Vec3,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObstacleKind {
    // Ice
    IceChunk,
    Hole,
    Crack,
    // Water
    Iceberg,
    Crab,
    // Sky
    Plane,
    Bird,
}

#[derive(Component)]
pub struct Obstacle {
    pub kind: ObstacleKind,
}

#[derive(Component)]
pub struct Collectible {
    pub points: u32,
}

#[derive(Component)]
pub struct CannonPickup;

/// Wing pivot on a bird obstacle; `side` is -1.0/1.0, `phase` staggers the
/// flap so a flock never beats in unison.
#[derive(Component)]
pub struct BirdWing {
    pub side: f32,
    pub phase: f32,
}

/// Pre-built blockout meshes/materials for everything the spawner emits.
#[derive(Resource)]
pub struct SpawnAssets {
    ice_chunk: (Handle<Mesh>, Handle<StandardMaterial>),
    hole: (Handle<Mesh>, Handle<StandardMaterial>),
    crack: (Handle<Mesh>, Handle<StandardMaterial>),
    iceberg: (Handle<Mesh>, Handle<StandardMaterial>),
    crab: (Handle<Mesh>, Handle<StandardMaterial>),
    plane: (Handle<Mesh>, Handle<StandardMaterial>),
    bird: (Handle<Mesh>, Handle<StandardMaterial>),
    bird_wing: Handle<Mesh>,
    bird_beak: (Handle<Mesh>, Handle<StandardMaterial>),
    fish: (Handle<Mesh>, Handle<StandardMaterial>),
    shrimp: (Handle<Mesh>, Handle<StandardMaterial>),
    cannon_barrel: (Handle<Mesh>, Handle<StandardMaterial>),
    cannon_beacon: (Handle<Mesh>, Handle<StandardMaterial>),
    decor_ice: (Handle<Mesh>, Handle<StandardMaterial>),
    decor_water: (Handle<Mesh>, Handle<StandardMaterial>),
    decor_sky: (Handle<Mesh>, Handle<StandardMaterial>),
}

pub struct SpawnPlugin;

impl Plugin for SpawnPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameRng>()
            .init_resource::<RunSpeed>()
            .init_resource::<Score>()
            .init_resource::<SpawnTimers>()
            .add_systems(Startup, build_spawn_assets)
            .add_systems(OnEnter(Biome::Ice), clear_spawned)
            .add_systems(OnEnter(Biome::Water), clear_spawned)
            .add_systems(OnEnter(Biome::Sky), clear_spawned)
            .add_systems(
                Update,
                (
                    ramp_speed,
                    spawn_tick,
                    advance_movers,
                    collide_player,
                    score_distance,
                    flap_bird_wings,
                )
                    .chain()
                    .run_if(in_state(RunState::Running)),
            );
    }
}

fn build_spawn_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut solid = |color: Color, rough: f32| {
        materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: rough,
            ..default()
        })
    };
    let ice_white = solid(Color::srgb(0.88, 0.96, 1.0), 0.3);
    let dark_hole = solid(Color::srgb(0.02, 0.09, 0.16), 0.9);
    let crack_blue = solid(Color::srgb(0.1, 0.3, 0.45), 0.7);
    let berg_blue = solid(Color::srgb(0.65, 0.85, 0.95), 0.25);
    let crab_red = solid(Color::srgb(0.85, 0.25, 0.15), 0.6);
    let plane_gray = solid(Color::srgb(0.75, 0.78, 0.82), 0.4);
    let bird_dark = solid(Color::srgb(0.25, 0.22, 0.28), 0.8);
    let beak_orange = solid(Color::srgb(0.95, 0.6, 0.15), 0.6);
    let fish_silver = solid(Color::srgb(0.6, 0.8, 0.9), 0.2);
    let shrimp_pink = solid(Color::srgb(1.0, 0.55, 0.6), 0.5);
    let cannon_iron = solid(Color::srgb(0.2, 0.22, 0.26), 0.5);
    let pebble_white = solid(Color::srgb(0.95, 0.98, 1.0), 0.5);

    let beacon_gold = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.8, 0.2),
        emissive: LinearRgba::rgb(2.0, 1.4, 0.2),
        ..default()
    });
    let bubble = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.9, 1.0, 0.4),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let cloud = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.75),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 1.0,
        ..default()
    });

    commands.insert_resource(SpawnAssets {
        ice_chunk: (meshes.add(Cuboid::new(1.6, 2.0, 1.4)), ice_white),
        hole: (meshes.add(Cuboid::new(2.2, 0.06, 2.0)), dark_hole),
        crack: (meshes.add(Cuboid::new(2.3, 0.05, 0.7)), crack_blue),
        iceberg: (meshes.add(Cuboid::new(1.8, 2.2, 1.6)), berg_blue),
        crab: (meshes.add(Cuboid::new(1.5, 0.8, 1.2)), crab_red),
        plane: (meshes.add(Cuboid::new(1.8, 0.8, 2.6)), plane_gray),
        bird: (meshes.add(Sphere::new(0.35)), bird_dark),
        bird_wing: meshes.add(Cuboid::new(0.62, 0.05, 0.30)),
        bird_beak: (
            meshes.add(Cone {
                radius: 0.08,
                height: 0.22,
            }),
            beak_orange,
        ),
        fish: (meshes.add(Sphere::new(0.3)), fish_silver),
        shrimp: (meshes.add(Sphere::new(0.24)), shrimp_pink),
        cannon_barrel: (meshes.add(Cylinder::new(0.45, 1.8)), cannon_iron),
        cannon_beacon: (meshes.add(Sphere::new(0.3)), beacon_gold),
        decor_ice: (meshes.add(Sphere::new(0.15)), pebble_white),
        decor_water: (meshes.add(Sphere::new(0.12)), bubble),
        decor_sky: (meshes.add(Sphere::new(1.1)), cloud),
    });
}

/// Difficulty creep: speed ramps up slowly; the post-hit slowdown ticks here.
fn ramp_speed(time: Res<Time>, mut speed: ResMut<RunSpeed>) {
    speed.base = (speed.base + RUN_SPEED_RAMP * time.delta_secs()).min(RUN_SPEED_MAX);
    speed.slowdown.tick(time.delta());
}

/// Scroll everything toward the camera; despawn what's behind it.
fn advance_movers(
    time: Res<Time>,
    speed: Res<RunSpeed>,
    mut commands: Commands,
    mut movers: Query<(Entity, &mut Transform), With<Mover>>,
) {
    let dz = speed.current() * time.delta_secs();
    for (entity, mut transform) in &mut movers {
        transform.translation.z += dz;
        if transform.translation.z > DESPAWN_Z {
            commands.entity(entity).despawn();
        }
    }
}

/// Biome switches and restarts sweep the track clean.
fn clear_spawned(mut commands: Commands, movers: Query<Entity, With<Mover>>) {
    for entity in &movers {
        commands.entity(entity).despawn();
    }
}

/// One spawner to rule them all: rolls each category's timer and emits
/// biome-appropriate obstacles, collectible lines, cannons, and decor.
fn spawn_tick(
    time: Res<Time>,
    biome: Res<State<Biome>>,
    assets: Res<SpawnAssets>,
    mut rng: ResMut<GameRng>,
    mut timers: ResMut<SpawnTimers>,
    mut commands: Commands,
) {
    let biome = *biome.get();
    let dt = time.delta();

    if timers.obstacle.tick(dt).finished() {
        let secs = rng.range(OBSTACLE_INTERVAL.0, OBSTACLE_INTERVAL.1);
        timers.obstacle = Timer::from_seconds(secs, TimerMode::Once);
        spawn_obstacle(&mut commands, &assets, &mut rng, biome);
    }

    if timers.collectible.tick(dt).finished() {
        let secs = rng.range(COLLECTIBLE_INTERVAL.0, COLLECTIBLE_INTERVAL.1);
        timers.collectible = Timer::from_seconds(secs, TimerMode::Once);
        spawn_collectible_line(&mut commands, &assets, &mut rng, biome);
    }

    // The cannon only spawns at ground level — the sky IS the cannon reward.
    if biome != Biome::Sky && timers.cannon.tick(dt).finished() {
        let secs = rng.range(CANNON_INTERVAL.0, CANNON_INTERVAL.1);
        timers.cannon = Timer::from_seconds(secs, TimerMode::Once);
        spawn_cannon(&mut commands, &assets, &mut rng);
    }

    if timers.decor.tick(dt).finished() {
        let secs = rng.range(DECOR_INTERVAL.0, DECOR_INTERVAL.1);
        timers.decor = Timer::from_seconds(secs, TimerMode::Once);
        spawn_decor(&mut commands, &assets, &mut rng, biome);
    }
}

fn spawn_obstacle(
    commands: &mut Commands,
    assets: &SpawnAssets,
    rng: &mut GameRng,
    biome: Biome,
) {
    let lane_x = rng.lane() as f32 * LANE_WIDTH;
    let roll = rng.f32();

    // (kind, asset, y, collider half-extents)
    let (kind, asset, y, half) = match biome {
        Biome::Ice => {
            if roll < 0.5 {
                // Dodge: too tall to jump.
                (ObstacleKind::IceChunk, &assets.ice_chunk, 1.0, Vec3::new(0.8, 1.0, 0.7))
            } else if roll < 0.75 {
                // Jump: flat, only hits a grounded penguin.
                (ObstacleKind::Hole, &assets.hole, 0.03, Vec3::new(1.1, 0.1, 1.0))
            } else {
                (ObstacleKind::Crack, &assets.crack, 0.03, Vec3::new(1.15, 0.1, 0.35))
            }
        }
        Biome::Water => {
            if roll < 0.55 {
                // Upper band: dive under or dodge.
                (ObstacleKind::Iceberg, &assets.iceberg, SWIM_BAND.1 - 0.2, Vec3::new(0.9, 1.1, 0.8))
            } else {
                // Sea floor: swim up or dodge.
                (ObstacleKind::Crab, &assets.crab, SWIM_BAND.0 - 0.1, Vec3::new(0.75, 0.4, 0.6))
            }
        }
        Biome::Sky => {
            // Full-column hazards: the lane dodge is the reliable evade.
            if roll < 0.45 {
                (ObstacleKind::Plane, &assets.plane, 3.0, Vec3::new(0.9, 1.5, 1.3))
            } else {
                (ObstacleKind::Bird, &assets.bird, 3.0, Vec3::new(0.4, 1.5, 0.4))
            }
        }
    };

    let mut entity = commands.spawn((
        Obstacle { kind },
        Mover,
        Collider { half },
        Mesh3d(asset.0.clone()),
        MeshMaterial3d(asset.1.clone()),
        Transform::from_xyz(lane_x, y, SPAWN_Z),
    ));

    // Planes get visual wings that overhang the lane (collider stays
    // fuselage-only so neighboring lanes remain fair).
    if kind == ObstacleKind::Plane {
        let wings = (assets.plane.0.clone(), assets.plane.1.clone());
        entity.with_children(|parent| {
            parent.spawn((
                Mesh3d(wings.0),
                MeshMaterial3d(wings.1),
                Transform::from_xyz(0.0, 0.1, 0.2).with_scale(Vec3::new(2.4, 0.18, 0.35)),
            ));
        });
    }

    // Birds fly at the player: sphere body plus head, orange beak, and two
    // flapping wings hinged at the shoulder. Collider unchanged.
    if kind == ObstacleKind::Bird {
        let body = assets.bird.clone();
        let wing_mesh = assets.bird_wing.clone();
        let beak = assets.bird_beak.clone();
        let phase = rng.range(0.0, std::f32::consts::TAU);
        entity.with_children(|parent| {
            // Head, toward the player (+Z is the approach direction).
            parent.spawn((
                Mesh3d(body.0.clone()),
                MeshMaterial3d(body.1.clone()),
                Transform::from_xyz(0.0, 0.17, 0.27).with_scale(Vec3::splat(0.55)),
            ));
            // Beak.
            parent.spawn((
                Mesh3d(beak.0),
                MeshMaterial3d(beak.1),
                Transform::from_xyz(0.0, 0.17, 0.48)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            ));
            // Wings: a pivot entity at the shoulder, mesh offset outward,
            // so the flap hinges at the body instead of the wing's center.
            for side in [-1.0f32, 1.0] {
                parent
                    .spawn((
                        BirdWing { side, phase },
                        Transform::from_xyz(side * 0.10, 0.08, 0.0),
                        Visibility::default(),
                    ))
                    .with_children(|wing| {
                        wing.spawn((
                            Mesh3d(wing_mesh.clone()),
                            MeshMaterial3d(body.1.clone()),
                            Transform::from_xyz(side * 0.34, 0.0, 0.0),
                        ));
                    });
            }
        });
    }
}

/// Wings beat fast, staggered per bird by their spawn phase.
fn flap_bird_wings(time: Res<Time>, mut wings: Query<(&BirdWing, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (wing, mut transform) in &mut wings {
        let angle = 0.55 * (t * 13.0 + wing.phase).sin();
        transform.rotation = Quat::from_rotation_z(wing.side * angle);
    }
}

fn spawn_collectible_line(
    commands: &mut Commands,
    assets: &SpawnAssets,
    rng: &mut GameRng,
    biome: Biome,
) {
    let lane_x = rng.lane() as f32 * LANE_WIDTH;
    let y = match biome {
        Biome::Ice => 1.1,
        Biome::Water => rng.range(SWIM_BAND.0 + 0.3, SWIM_BAND.1 - 0.3),
        Biome::Sky => 3.0,
    };
    let shrimp_line = rng.chance(0.3);
    let (asset, points) = if shrimp_line {
        (&assets.shrimp, SHRIMP_POINTS)
    } else {
        (&assets.fish, FISH_POINTS)
    };

    for i in 0..4 {
        commands.spawn((
            Collectible { points },
            Mover,
            Collider {
                half: Vec3::new(0.45, if biome == Biome::Sky { 1.5 } else { 0.5 }, 0.45),
            },
            Mesh3d(asset.0.clone()),
            MeshMaterial3d(asset.1.clone()),
            Transform::from_xyz(lane_x, y, SPAWN_Z - i as f32 * 2.2)
                .with_scale(Vec3::new(0.8, 0.8, 1.4)),
        ));
    }
}

fn spawn_cannon(commands: &mut Commands, assets: &SpawnAssets, rng: &mut GameRng) {
    let lane_x = rng.lane() as f32 * LANE_WIDTH;
    commands
        .spawn((
            CannonPickup,
            Mover,
            Collider {
                half: Vec3::new(1.0, 2.0, 0.8),
            },
            Mesh3d(assets.cannon_barrel.0.clone()),
            MeshMaterial3d(assets.cannon_barrel.1.clone()),
            // Barrel tilted up and toward the player so the silhouette reads.
            Transform::from_xyz(lane_x, 1.0, SPAWN_Z)
                .with_rotation(Quat::from_rotation_x(0.6)),
        ))
        .with_children(|parent| {
            // Golden beacon: rare pickup = maximum visual salience.
            parent.spawn((
                Mesh3d(assets.cannon_beacon.0.clone()),
                MeshMaterial3d(assets.cannon_beacon.1.clone()),
                Transform::from_xyz(0.0, 1.5, 0.0),
            ));
        });
}

fn spawn_decor(commands: &mut Commands, assets: &SpawnAssets, rng: &mut GameRng, biome: Biome) {
    let (asset, x, y, scale) = match biome {
        Biome::Ice => (
            &assets.decor_ice,
            rng.range(-6.0, 6.0),
            0.1,
            Vec3::splat(rng.range(0.6, 1.4)),
        ),
        Biome::Water => (
            &assets.decor_water,
            rng.range(-6.0, 6.0),
            rng.range(0.3, 5.0),
            Vec3::splat(rng.range(0.5, 1.2)),
        ),
        Biome::Sky => (
            &assets.decor_sky,
            rng.range(-9.0, 9.0),
            rng.range(0.2, 5.5),
            Vec3::new(rng.range(1.2, 2.2), 0.7, rng.range(1.0, 1.8)),
        ),
    };
    commands.spawn((
        Mover,
        Mesh3d(asset.0.clone()),
        MeshMaterial3d(asset.1.clone()),
        Transform::from_xyz(x, y, SPAWN_Z).with_scale(scale),
    ));
}

/// Player AABB vs everything with a [`Collider`]. Obstacles slow the run and
/// feed the orca; collectibles score; the cannon fires the penguin skyward.
fn collide_player(
    mut commands: Commands,
    mut speed: ResMut<RunSpeed>,
    mut score: ResMut<Score>,
    mut altitude: ResMut<Altitude>,
    mut hits: EventWriter<ObstacleHit>,
    mut cues: EventWriter<AudioCue>,
    mut next_biome: ResMut<NextState<Biome>>,
    biome: Res<State<Biome>>,
    players: Query<(&Transform, &Vertical), With<Player>>,
    obstacles: Query<(Entity, &Transform, &Collider, &Obstacle)>,
    collectibles: Query<(Entity, &Transform, &Collider), With<Collectible>>,
    collectible_points: Query<&Collectible>,
    cannons: Query<(Entity, &Transform, &Collider), With<CannonPickup>>,
) {
    let Ok((player_transform, vertical)) = players.single() else {
        return;
    };
    let center = player_transform.translation;
    // Belly-sliding shrinks the hitbox with the squash.
    let player_half = Vec3::new(0.4, 0.65 * player_transform.scale.y, 0.4);

    let overlaps = |other: &Transform, half: &Vec3| -> bool {
        let d = (other.translation - center).abs();
        d.x < half.x + player_half.x
            && d.y < half.y + player_half.y
            && d.z < half.z + player_half.z
    };

    // A slowed player is briefly "staggered" — no double penalties while the
    // same obstacle cluster passes through (GDD "Orca Chase" edge case).
    if !speed.is_slowed() {
        for (entity, transform, collider, obstacle) in &obstacles {
            if overlaps(transform, &collider.half) {
                // Holes/cracks only trip a grounded penguin — a jump clears them.
                let grounded_only = matches!(
                    obstacle.kind,
                    ObstacleKind::Hole | ObstacleKind::Crack
                );
                if grounded_only && !vertical.grounded {
                    continue;
                }
                commands.entity(entity).despawn();
                speed.apply_hit_slowdown();
                hits.write(ObstacleHit);
                if *biome.get() == Biome::Sky {
                    altitude.0 = (altitude.0 - SKY_HIT_ALTITUDE_PENALTY).max(0.0);
                }
                break;
            }
        }
    }

    for (entity, transform, collider) in &collectibles {
        if overlaps(transform, &collider.half) {
            if let Ok(collectible) = collectible_points.get(entity) {
                score.points += collectible.points;
                cues.write(if collectible.points == FISH_POINTS {
                    AudioCue::CollectFish
                } else {
                    AudioCue::CollectShrimp
                });
            }
            commands.entity(entity).despawn();
        }
    }

    for (entity, transform, collider) in &cannons {
        if overlaps(transform, &collider.half) {
            commands.entity(entity).despawn();
            cues.write(AudioCue::CannonLaunch);
            states::launch_to_sky(&mut next_biome);
        }
    }
}

/// Distance is worth points too — surviving IS scoring.
fn score_distance(time: Res<Time>, speed: Res<RunSpeed>, mut score: ResMut<Score>) {
    score.distance_acc += speed.current() * time.delta_secs();
    while score.distance_acc >= METERS_PER_POINT {
        score.distance_acc -= METERS_PER_POINT;
        score.points += 1;
    }
}
