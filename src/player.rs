//! Player entity and movement: lane dodging, jumping, belly-sliding,
//! swimming, and gliding — with per-biome physics feel (slippery on ice,
//! floaty in water, free-falling in the sky).

use bevy::prelude::*;

use crate::states::{Altitude, Biome, RunState, SkyPitch};
use crate::tuning::*;

/// Marker for the player entity (the penguin root).
#[derive(Component)]
pub struct Player;

/// Which lane the player is steering toward (-1, 0, or 1).
#[derive(Component, Default)]
pub struct TargetLane(pub i8);

/// Vertical motion state for jumping and sliding.
#[derive(Component, Default)]
pub struct Vertical {
    pub velocity: f32,
    pub grounded: bool,
    pub sliding: bool,
}

/// Per-biome movement feel, swapped on biome entry.
#[derive(Resource)]
pub struct BiomePhysics {
    pub lane_lerp: f32,
    pub gravity: f32,
}

impl Default for BiomePhysics {
    fn default() -> Self {
        Self {
            lane_lerp: LANE_LERP_ICE,
            gravity: GRAVITY_ICE,
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BiomePhysics>()
            .add_systems(Startup, setup_penguin)
            .add_systems(OnEnter(Biome::Ice), enter_ice_physics)
            .add_systems(OnEnter(Biome::Water), enter_water_physics)
            .add_systems(OnEnter(Biome::Sky), enter_sky_physics)
            .add_systems(
                Update,
                (
                    steer_lanes,
                    apply_lane_position,
                    vertical_ice.run_if(in_state(Biome::Ice)),
                    vertical_water.run_if(in_state(Biome::Water)),
                    vertical_sky.run_if(in_state(Biome::Sky)),
                )
                    .chain()
                    .run_if(in_state(RunState::Running)),
            );
    }
}

/// Blockout penguin: black capsule body, white belly, orange beak.
/// Grey-box only — art pass comes after mechanics are proven.
fn setup_penguin(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let body_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.13, 0.17),
        perceptual_roughness: 0.8,
        ..default()
    });
    let belly_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.93),
        perceptual_roughness: 0.9,
        ..default()
    });
    let beak_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.55, 0.1),
        perceptual_roughness: 0.6,
        ..default()
    });

    commands
        .spawn((
            Player,
            TargetLane::default(),
            Vertical {
                grounded: true,
                ..default()
            },
            Transform::from_xyz(0.0, GROUND_Y, 0.0),
            Visibility::default(),
        ))
        .with_children(|parent| {
            // Body
            parent.spawn((
                Mesh3d(meshes.add(Capsule3d::new(0.42, 0.7))),
                MeshMaterial3d(body_material),
                Transform::default(),
            ));
            // Belly: flattened sphere pushed toward the direction of travel (-Z)
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.36))),
                MeshMaterial3d(belly_material),
                Transform::from_xyz(0.0, -0.05, -0.18).with_scale(Vec3::new(0.9, 1.1, 0.7)),
            ));
            // Beak: cone rotated to point down the track
            parent.spawn((
                Mesh3d(meshes.add(Cone {
                    radius: 0.12,
                    height: 0.32,
                })),
                MeshMaterial3d(beak_material),
                Transform::from_xyz(0.0, 0.42, -0.42)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ));
        });
}

fn enter_ice_physics(mut physics: ResMut<BiomePhysics>) {
    physics.lane_lerp = LANE_LERP_ICE;
    physics.gravity = GRAVITY_ICE;
}

fn enter_water_physics(
    mut physics: ResMut<BiomePhysics>,
    mut players: Query<&mut Vertical, With<Player>>,
) {
    physics.lane_lerp = LANE_LERP_WATER;
    physics.gravity = GRAVITY_WATER;
    for mut vertical in &mut players {
        vertical.velocity = 0.0;
        vertical.grounded = false;
        vertical.sliding = false;
    }
}

fn enter_sky_physics(
    mut physics: ResMut<BiomePhysics>,
    mut players: Query<&mut Vertical, With<Player>>,
) {
    physics.lane_lerp = LANE_LERP_SKY;
    for mut vertical in &mut players {
        vertical.velocity = 0.0;
        vertical.grounded = false;
        vertical.sliding = false;
    }
}

fn left_just_pressed(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA)
}

fn right_just_pressed(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD)
}

fn up_just_pressed(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::ArrowUp)
        || keys.just_pressed(KeyCode::KeyW)
        || keys.just_pressed(KeyCode::Space)
}

fn up_held(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::Space)
}

fn down_held(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS)
}

/// Lane input: retargets immediately, ignored at the outer lanes
/// (see GDD "Lane Dodge" edge cases).
fn steer_lanes(keys: Res<ButtonInput<KeyCode>>, mut players: Query<&mut TargetLane, With<Player>>) {
    let left = left_just_pressed(&keys);
    let right = right_just_pressed(&keys);
    if left == right {
        return;
    }
    let step: i8 = if right { 1 } else { -1 };
    for mut lane in &mut players {
        lane.0 = (lane.0 + step).clamp(-1, 1);
    }
}

/// Slide the penguin toward its target lane at the biome's lerp rate —
/// this is where "slippery" vs "responsive" comes from.
fn apply_lane_position(
    time: Res<Time>,
    physics: Res<BiomePhysics>,
    mut players: Query<(&TargetLane, &mut Transform), With<Player>>,
) {
    for (lane, mut transform) in &mut players {
        let target = lane.0 as f32 * LANE_WIDTH;
        let fraction = (physics.lane_lerp * time.delta_secs()).min(1.0);
        transform.translation.x += (target - transform.translation.x) * fraction;
    }
}

/// Ice: arcade jump arc plus belly-slide squash while Down is held.
fn vertical_ice(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    physics: Res<BiomePhysics>,
    mut players: Query<(&mut Vertical, &mut Transform), With<Player>>,
) {
    let dt = time.delta_secs();
    for (mut vertical, mut transform) in &mut players {
        if vertical.grounded && up_just_pressed(&keys) {
            vertical.velocity = JUMP_IMPULSE;
            vertical.grounded = false;
        }
        if !vertical.grounded {
            vertical.velocity -= physics.gravity * dt;
            transform.translation.y += vertical.velocity * dt;
            if transform.translation.y <= GROUND_Y {
                transform.translation.y = GROUND_Y;
                vertical.velocity = 0.0;
                vertical.grounded = true;
            }
        }
        // Belly-slide: squash the penguin and (via collision) shrink its hitbox.
        vertical.sliding = vertical.grounded && down_held(&keys);
        transform.scale = if vertical.sliding {
            Vec3::new(1.0, 0.55, 1.0)
        } else {
            Vec3::ONE
        };
    }
}

/// Water: neutral buoyancy; W swims up, S dives, clamped to the swim band.
fn vertical_water(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut players: Query<(&mut Vertical, &mut Transform), With<Player>>,
) {
    let dt = time.delta_secs();
    let direction = (up_held(&keys) as i8 - down_held(&keys) as i8) as f32;
    for (mut vertical, mut transform) in &mut players {
        transform.translation.y =
            (transform.translation.y + direction * SWIM_SPEED * dt).clamp(SWIM_BAND.0, SWIM_BAND.1);
        vertical.sliding = false;
        transform.scale = Vec3::ONE;
    }
}

/// Sky: W pitches up (sink slower), S dives (sink faster). The penguin's
/// height on screen tracks remaining altitude, so the descent reads visually.
fn vertical_sky(
    keys: Res<ButtonInput<KeyCode>>,
    altitude: Res<Altitude>,
    mut pitch: ResMut<SkyPitch>,
    mut players: Query<&mut Transform, With<Player>>,
) {
    *pitch = if up_held(&keys) {
        SkyPitch::Up
    } else if down_held(&keys) {
        SkyPitch::Dive
    } else {
        SkyPitch::Level
    };
    let fraction = (altitude.0 / SKY_ALTITUDE_MAX).clamp(0.0, 1.0);
    for mut transform in &mut players {
        transform.translation.y = SKY_Y_RANGE.0 + fraction * (SKY_Y_RANGE.1 - SKY_Y_RANGE.0);
        transform.scale = Vec3::ONE;
    }
}
