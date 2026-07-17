//! Player entity and movement: lane dodging, jumping, belly-sliding,
//! swimming, and gliding — with per-biome physics feel (slippery on ice,
//! floaty in water, free-falling in the sky).

use bevy::prelude::*;

use crate::audio::AudioCue;
use crate::states::{Altitude, Biome, RunState, SkyPitch};
use crate::touch::{TouchAction, TouchIntent};
use crate::transitions::transition_inactive;
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
                    // Steering pauses while a layer-crossing animation plays.
                    steer_lanes.run_if(transition_inactive),
                    apply_lane_position,
                    vertical_ice.run_if(in_state(Biome::Ice)),
                    vertical_water.run_if(in_state(Biome::Water)),
                    vertical_sky.run_if(in_state(Biome::Sky)),
                    animate_flippers,
                )
                    .chain()
                    .run_if(in_state(RunState::Running)),
            );
    }
}

/// Marker for the penguin's flippers; `side` is -1.0 (left) or 1.0 (right).
#[derive(Component)]
pub struct Flipper {
    pub side: f32,
}

/// The penguin, art-pass edition: egg-shaped body, head with white face
/// patch and eyes, orange beak, two animated flippers, orange webbed feet,
/// and a stubby tail. Still primitives only — no external assets, so the
/// wasm bundle stays self-contained. The collision hitbox in `spawn.rs`
/// is unchanged.
fn setup_penguin(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let black = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.13, 0.17),
        perceptual_roughness: 0.8,
        ..default()
    });
    let white = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.93),
        perceptual_roughness: 0.9,
        ..default()
    });
    let orange = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.55, 0.1),
        perceptual_roughness: 0.6,
        ..default()
    });

    let flipper_mesh = meshes.add(Capsule3d::new(0.09, 0.36));
    let foot_mesh = meshes.add(Sphere::new(0.16));
    let eye_mesh = meshes.add(Sphere::new(0.05));

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
            // Body: capsule squashed into an egg silhouette.
            parent.spawn((
                Mesh3d(meshes.add(Capsule3d::new(0.42, 0.55))),
                MeshMaterial3d(black.clone()),
                Transform::from_xyz(0.0, -0.06, 0.0).with_scale(Vec3::new(1.0, 1.05, 0.92)),
            ));
            // Belly: white front, from chest down to the feet.
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.36))),
                MeshMaterial3d(white.clone()),
                Transform::from_xyz(0.0, -0.12, -0.19).with_scale(Vec3::new(0.85, 1.2, 0.6)),
            ));
            // Head: sits into the body top so the join reads as a neck.
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.30))),
                MeshMaterial3d(black.clone()),
                Transform::from_xyz(0.0, 0.58, -0.02),
            ));
            // Face patch: white oval on the front of the head.
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.24))),
                MeshMaterial3d(white.clone()),
                Transform::from_xyz(0.0, 0.56, -0.14).with_scale(Vec3::new(0.82, 0.88, 0.55)),
            ));
            // Eyes: black beads on the face patch.
            for side in [-1.0f32, 1.0] {
                parent.spawn((
                    Mesh3d(eye_mesh.clone()),
                    MeshMaterial3d(black.clone()),
                    Transform::from_xyz(side * 0.11, 0.64, -0.26).with_scale(Vec3::splat(0.9)),
                ));
            }
            // Beak
            parent.spawn((
                Mesh3d(meshes.add(Cone {
                    radius: 0.09,
                    height: 0.26,
                })),
                MeshMaterial3d(orange.clone()),
                Transform::from_xyz(0.0, 0.54, -0.32)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ));
            // Flippers: flattened capsules at the shoulders; animated by
            // `animate_flippers` (jog-flap on the ground, spread in the sky).
            for side in [-1.0f32, 1.0] {
                parent.spawn((
                    Flipper { side },
                    Mesh3d(flipper_mesh.clone()),
                    MeshMaterial3d(black.clone()),
                    Transform::from_xyz(side * 0.44, 0.05, 0.02)
                        .with_scale(Vec3::new(0.45, 1.0, 0.75)),
                ));
            }
            // Feet: orange webbed paddles, toed slightly outward.
            for side in [-1.0f32, 1.0] {
                parent.spawn((
                    Mesh3d(foot_mesh.clone()),
                    MeshMaterial3d(orange.clone()),
                    Transform::from_xyz(side * 0.17, -0.66, -0.10)
                        .with_scale(Vec3::new(1.0, 0.35, 1.9))
                        .with_rotation(Quat::from_rotation_y(side * -0.25)),
                ));
            }
            // Tail: stubby cone sweeping back and up.
            parent.spawn((
                Mesh3d(meshes.add(Cone {
                    radius: 0.12,
                    height: 0.28,
                })),
                MeshMaterial3d(black.clone()),
                Transform::from_xyz(0.0, -0.42, 0.38)
                    .with_rotation(Quat::from_rotation_x(2.2)),
            ));
        });
}

/// Flippers flap while running/swimming and lock spread-wide while gliding
/// through the sky. Rotation is authored around the shoulder's Z axis on
/// top of the spawn pose.
fn animate_flippers(
    time: Res<Time>,
    biome: Res<State<Biome>>,
    mut flippers: Query<(&Flipper, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (flipper, mut transform) in &mut flippers {
        let angle = match biome.get() {
            // Gliding: wings out.
            Biome::Sky => 1.25,
            // Jogging/swimming: quick, small flaps.
            _ => 0.35 + 0.18 * (t * 9.0).sin(),
        };
        transform.rotation = Quat::from_rotation_z(flipper.side * -angle);
    }
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

/// Lane input (keys or swipes): retargets immediately, ignored at the
/// outer lanes (see GDD "Lane Dodge" edge cases).
fn steer_lanes(
    keys: Res<ButtonInput<KeyCode>>,
    mut touch_actions: EventReader<TouchAction>,
    mut players: Query<&mut TargetLane, With<Player>>,
) {
    let mut step: i8 = 0;
    if left_just_pressed(&keys) {
        step -= 1;
    }
    if right_just_pressed(&keys) {
        step += 1;
    }
    for action in touch_actions.read() {
        match action {
            TouchAction::SwipeLeft => step -= 1,
            TouchAction::SwipeRight => step += 1,
            _ => {}
        }
    }
    if step == 0 {
        return;
    }
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
    touch: Res<TouchIntent>,
    mut touch_actions: EventReader<TouchAction>,
    mut cues: EventWriter<AudioCue>,
    mut players: Query<(&mut Vertical, &mut Transform), With<Player>>,
) {
    let dt = time.delta_secs();
    let swipe_jump = touch_actions
        .read()
        .any(|action| *action == TouchAction::SwipeUp);
    for (mut vertical, mut transform) in &mut players {
        if vertical.grounded && (up_just_pressed(&keys) || swipe_jump) {
            vertical.velocity = JUMP_IMPULSE;
            vertical.grounded = false;
            cues.write(AudioCue::Jump);
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
        vertical.sliding = vertical.grounded && (down_held(&keys) || touch.down_held());
        transform.scale = if vertical.sliding {
            Vec3::new(1.0, 0.55, 1.0)
        } else {
            Vec3::ONE
        };
    }
}

/// Water: neutral buoyancy; W swims up, S dives, clamped to the swim band.
/// Vertical swipes emulate a short hold (see `TouchIntent`).
fn vertical_water(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    touch: Res<TouchIntent>,
    mut players: Query<(&mut Vertical, &mut Transform), With<Player>>,
) {
    let dt = time.delta_secs();
    let up = up_held(&keys) || touch.up_held();
    let down = down_held(&keys) || touch.down_held();
    let direction = (up as i8 - down as i8) as f32;
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
    touch: Res<TouchIntent>,
    altitude: Res<Altitude>,
    mut pitch: ResMut<SkyPitch>,
    mut players: Query<&mut Transform, With<Player>>,
) {
    *pitch = if up_held(&keys) || touch.up_held() {
        SkyPitch::Up
    } else if down_held(&keys) || touch.down_held() {
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
