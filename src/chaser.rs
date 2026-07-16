//! The orca's on-screen presence: blockout body, biome visibility, and the
//! screen-space mapping of the chase gap. The chase *rules* (gap
//! bookkeeping, catch condition) live in `penguin_core::chase`.

use bevy::prelude::*;

use crate::chase::{ChasePlugin, OrcaGap};
use crate::player::Player;
use crate::states::Biome;
use crate::tuning::*;

/// Marker for the orca entity root.
#[derive(Component)]
pub struct Orca;

pub struct ChaserPlugin;

impl Plugin for ChaserPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ChasePlugin)
            .add_systems(Startup, setup_orca)
            // The bonus phase: no orca in the sky.
            .add_systems(OnEnter(Biome::Sky), hide_orca)
            .add_systems(OnEnter(Biome::Ice), show_orca)
            .add_systems(OnEnter(Biome::Water), show_orca)
            .add_systems(Update, position_orca);
    }
}

/// Blockout orca: black capsule lying along the track, white belly,
/// tall dorsal fin so the silhouette reads instantly.
fn setup_orca(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let black = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.06, 0.09),
        perceptual_roughness: 0.4,
        ..default()
    });
    let white = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.94, 0.95),
        perceptual_roughness: 0.5,
        ..default()
    });

    commands
        .spawn((
            Orca,
            Transform::from_xyz(0.0, -0.1, ORCA_VISUAL_Z.1),
            Visibility::default(),
        ))
        .with_children(|parent| {
            // Body: capsule rotated to lie along the track (Z axis).
            parent.spawn((
                Mesh3d(meshes.add(Capsule3d::new(0.7, 2.2))),
                MeshMaterial3d(black.clone()),
                Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            ));
            // Belly patch.
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.55))),
                MeshMaterial3d(white.clone()),
                Transform::from_xyz(0.0, -0.35, -0.6).with_scale(Vec3::new(1.0, 0.5, 1.4)),
            ));
            // Eye patches: the orca's signature marking.
            for side in [-1.0, 1.0] {
                parent.spawn((
                    Mesh3d(meshes.add(Sphere::new(0.16))),
                    MeshMaterial3d(white.clone()),
                    Transform::from_xyz(side * 0.5, 0.25, -1.5)
                        .with_scale(Vec3::new(0.5, 0.7, 1.0)),
                ));
            }
            // Dorsal fin.
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.14, 1.1, 0.7))),
                MeshMaterial3d(black),
                Transform::from_xyz(0.0, 0.9, 0.4).with_rotation(Quat::from_rotation_x(-0.25)),
            ));
        });
}

fn hide_orca(mut orcas: Query<&mut Visibility, With<Orca>>) {
    for mut visibility in &mut orcas {
        *visibility = Visibility::Hidden;
    }
}

fn show_orca(mut orcas: Query<&mut Visibility, With<Orca>>) {
    for mut visibility in &mut orcas {
        *visibility = Visibility::Inherited;
    }
}

/// Map the abstract gap onto screen space: the orca slides between the
/// camera and the penguin, hunting across lanes toward the player.
fn position_orca(
    time: Res<Time>,
    gap: Res<OrcaGap>,
    players: Query<&Transform, (With<Player>, Without<Orca>)>,
    mut orcas: Query<&mut Transform, With<Orca>>,
) {
    let player_x = players.single().map(|t| t.translation.x).unwrap_or(0.0);
    let fraction = (gap.0 / ORCA_MAX_GAP).clamp(0.0, 1.0);
    let target_z = ORCA_VISUAL_Z.0 + fraction * (ORCA_VISUAL_Z.1 - ORCA_VISUAL_Z.0);
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();

    for mut transform in &mut orcas {
        let t = &mut transform.translation;
        t.z += (target_z - t.z) * (3.0 * dt).min(1.0);
        t.x += (player_x - t.x) * (2.5 * dt).min(1.0);
        // Porpoising bob: keeps the chase alive even at constant gap.
        t.y = -0.1 + 0.25 * (elapsed * 3.0).sin();
    }
}
