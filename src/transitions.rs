//! Layer-crossing logic and animation. The game has three vertical layers
//! — ice surface, underwater, sky — and every crossing between them gets
//! a short, authored moment: the penguin dives in, hops out, backflips off
//! the cannon, or noses down for splashdown. Steering locks while a
//! transition plays so the animation reads (GDD 0.5).

use bevy::prelude::*;

use crate::player::Player;
use crate::spawn::GameRng;
use crate::states::Biome;
use crate::tuning::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransitionKind {
    /// Ice -> Water: nose-first dive.
    Dive,
    /// Water -> Ice: hop out onto the shelf.
    Surface,
    /// Ground -> Sky: cannon backflip.
    Launch,
    /// Sky -> Water: nose-down re-entry.
    Splashdown,
}

/// Present only while a layer-crossing animation is playing.
#[derive(Resource)]
pub struct LayerTransition {
    pub kind: TransitionKind,
    pub timer: Timer,
}

impl LayerTransition {
    fn new(kind: TransitionKind, secs: f32) -> Self {
        Self {
            kind,
            timer: Timer::from_seconds(secs, TimerMode::Once),
        }
    }
}

/// Run condition: no layer transition is currently animating.
pub fn transition_inactive(transition: Option<Res<LayerTransition>>) -> bool {
    transition.is_none()
}

/// Short-lived splash particle from water entry.
#[derive(Component)]
struct SplashDrop {
    velocity: Vec3,
    timer: Timer,
}

pub struct LayerTransitionPlugin;

impl Plugin for LayerTransitionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnTransition {
                exited: Biome::Ice,
                entered: Biome::Water,
            },
            (start_dive, splash_burst),
        )
        .add_systems(
            OnTransition {
                exited: Biome::Water,
                entered: Biome::Ice,
            },
            start_surface,
        )
        .add_systems(
            OnTransition {
                exited: Biome::Ice,
                entered: Biome::Sky,
            },
            start_launch,
        )
        .add_systems(
            OnTransition {
                exited: Biome::Water,
                entered: Biome::Sky,
            },
            start_launch,
        )
        .add_systems(
            OnTransition {
                exited: Biome::Sky,
                entered: Biome::Water,
            },
            (start_splashdown, splash_burst),
        )
        .add_systems(Update, (animate_transition, animate_splash_drops));
    }
}

fn start_dive(mut commands: Commands) {
    commands.insert_resource(LayerTransition::new(TransitionKind::Dive, TRANSITION_DIVE_SECS));
}

fn start_surface(mut commands: Commands) {
    commands.insert_resource(LayerTransition::new(
        TransitionKind::Surface,
        TRANSITION_SURFACE_SECS,
    ));
}

fn start_launch(mut commands: Commands) {
    commands.insert_resource(LayerTransition::new(
        TransitionKind::Launch,
        TRANSITION_LAUNCH_SECS,
    ));
}

fn start_splashdown(mut commands: Commands) {
    commands.insert_resource(LayerTransition::new(
        TransitionKind::Splashdown,
        TRANSITION_SPLASHDOWN_SECS,
    ));
}

/// Drive the player's pitch through the active transition, then clear it.
fn animate_transition(
    time: Res<Time>,
    transition: Option<ResMut<LayerTransition>>,
    mut commands: Commands,
    mut players: Query<&mut Transform, With<Player>>,
) {
    let Some(mut transition) = transition else {
        return;
    };
    transition.timer.tick(time.delta());
    // 0 -> 1 over the transition.
    let progress = transition.timer.fraction();

    let rotation = match transition.kind {
        // Nose-down entry that eases back to level.
        TransitionKind::Dive | TransitionKind::Splashdown => {
            Quat::from_rotation_x(0.9 * (1.0 - progress) * (progress * 8.0).min(1.0))
        }
        // Pop up and out, settling flat.
        TransitionKind::Surface => Quat::from_rotation_x(-0.7 * (1.0 - progress)),
        // Full backflip off the cannon.
        TransitionKind::Launch => {
            Quat::from_rotation_x(std::f32::consts::TAU * progress)
        }
    };
    for mut transform in &mut players {
        transform.rotation = rotation;
    }

    if transition.timer.finished() {
        for mut transform in &mut players {
            transform.rotation = Quat::IDENTITY;
        }
        commands.remove_resource::<LayerTransition>();
    }
}

/// A handful of white droplets kicked up wherever the penguin pierces the
/// water surface (both dive-in and splashdown).
fn splash_burst(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    players: Query<&Transform, With<Player>>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let mesh = meshes.add(Sphere::new(0.09));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.85, 0.95, 1.0, 0.85),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    for _ in 0..SPLASH_DROPLETS {
        let velocity = Vec3::new(
            rng.range(-2.2, 2.2),
            rng.range(2.0, 4.5),
            rng.range(-1.0, 1.5),
        );
        commands.spawn((
            SplashDrop {
                velocity,
                timer: Timer::from_seconds(0.7, TimerMode::Once),
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(player.translation),
        ));
    }
}

fn animate_splash_drops(
    time: Res<Time>,
    mut commands: Commands,
    mut drops: Query<(Entity, &mut SplashDrop, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut drop, mut transform) in &mut drops {
        drop.velocity.y -= 9.0 * dt;
        transform.translation += drop.velocity * dt;
        if drop.timer.tick(time.delta()).finished() {
            commands.entity(entity).despawn();
        }
    }
}
