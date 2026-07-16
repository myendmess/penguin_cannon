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

pub mod chaser;
pub mod player;
pub mod spawn;
pub mod ui;
pub mod world;

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
        .add_plugins((
            states::GameStatePlugin,
            world::WorldPlugin,
            player::PlayerPlugin,
            spawn::SpawnPlugin,
            chaser::ChaserPlugin,
            ui::UiPlugin,
        ))
        .run();
}

/// Wasm entry point: runs automatically once the module is initialized
/// from web/index.html.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    run();
}

// The player entity (penguin blockout, movement, per-biome physics) lives
// in `player`; the scene and biome environments live in `world`.
