//! Penguin Cannon — endless 3D lane-runner.
//!
//! A South Pole penguin flees an orca across three lanes of ice, water,
//! and (with a little artillery assistance) sky, trying to reach the
//! North Pole. Built with Bevy, compiled to WebAssembly.

use bevy::prelude::*;

// Game logic (states, chase rules, tuning) lives in the rendering-free
// penguin_core crate so its tests run headless; re-export the modules so
// game code can keep `crate::states::...` paths.
pub use penguin_core::{chase, states, tuning};

pub mod world;

use tuning::GROUND_Y;

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
        .add_plugins((states::GameStatePlugin, world::WorldPlugin))
        .add_systems(Startup, setup_penguin)
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
        .spawn((Player, Transform::from_xyz(0.0, GROUND_Y, 0.0), Visibility::default()))
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
