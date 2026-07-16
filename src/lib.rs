//! Penguin Cannon — endless 3D lane-runner.
//!
//! A South Pole penguin flees an orca across three lanes of ice, water,
//! and (with a little artillery assistance) sky, trying to reach the
//! North Pole. Built with Bevy, compiled to WebAssembly.

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;

pub mod tuning;

use tuning::{LANE_WIDTH, TRACK_VIEW_DEPTH};

/// Builds and runs the game app. Shared by the native binary and the
/// wasm entry point.
pub fn run() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Penguin Cannon".into(),
                // The id of the <canvas> element in web/index.html.
                canvas: Some("#penguin-cannon-canvas".into()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.72, 0.86, 0.96)))
        .add_systems(Startup, (setup_camera_and_light, setup_track, setup_penguin))
        .run();
}

/// Wasm entry point: runs automatically once the module is initialized
/// from web/index.html.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    run();
}

/// Marker for the player entity (the penguin root).
#[derive(Component)]
pub struct Player;

fn setup_camera_and_light(mut commands: Commands) {
    // Third-person trailing camera: behind and above the penguin,
    // looking down the track (the penguin runs toward -Z).
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.5, 9.0).looking_at(Vec3::new(0.0, 1.0, -12.0), Vec3::Y),
        DistanceFog {
            color: Color::srgb(0.72, 0.86, 0.96),
            falloff: FogFalloff::Linear {
                start: TRACK_VIEW_DEPTH * 0.5,
                end: TRACK_VIEW_DEPTH,
            },
            ..default()
        },
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 11_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.4, 0.0)),
    ));

    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.8, 0.9, 1.0),
        brightness: 300.0,
        ..default()
    });
}

/// Three readable lane strips. The center lane is tinted slightly darker so
/// players can always tell which lane they occupy (Level Designer rule:
/// the critical path must be visually legible).
fn setup_track(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let lane_mesh = meshes.add(Cuboid::new(LANE_WIDTH - 0.1, 0.2, TRACK_VIEW_DEPTH * 2.0));
    let side_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.93, 0.98),
        perceptual_roughness: 0.35,
        ..default()
    });
    let center_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.78, 0.88, 0.96),
        perceptual_roughness: 0.35,
        ..default()
    });

    for lane in -1i8..=1 {
        let material = if lane == 0 {
            center_material.clone()
        } else {
            side_material.clone()
        };
        commands.spawn((
            Mesh3d(lane_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(lane as f32 * LANE_WIDTH, -0.1, -TRACK_VIEW_DEPTH * 0.5),
        ));
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
        .spawn((Player, Transform::from_xyz(0.0, 0.75, 0.0), Visibility::default()))
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
