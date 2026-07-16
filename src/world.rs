//! Static scene (camera, light, track) and the per-biome environment
//! switching that gives Ice, Water, and Sky their distinct looks.

use bevy::core_pipeline::motion_blur::MotionBlur;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;

use crate::states::Biome;
use crate::tuning::*;

/// Marker for the three lane strip entities.
#[derive(Component)]
pub struct LaneStrip {
    pub center: bool,
}

/// Pre-built lane materials per biome, created once at startup.
#[derive(Resource)]
pub struct TrackMaterials {
    ice: [Handle<StandardMaterial>; 2],
    water: [Handle<StandardMaterial>; 2],
    sky: [Handle<StandardMaterial>; 2],
}

impl TrackMaterials {
    /// `[side, center]` pair for the given biome.
    fn for_biome(&self, biome: Biome) -> &[Handle<StandardMaterial>; 2] {
        match biome {
            Biome::Ice => &self.ice,
            Biome::Water => &self.water,
            Biome::Sky => &self.sky,
        }
    }
}

/// Environment palette per biome: clear color, fog range, ambient brightness.
struct BiomePalette {
    clear: Color,
    fog_start: f32,
    fog_end: f32,
    ambient: f32,
}

fn palette(biome: Biome) -> BiomePalette {
    match biome {
        // Bright antarctic day: white-blue ice under a pale sky.
        Biome::Ice => BiomePalette {
            clear: Color::srgb(0.72, 0.86, 0.96),
            fog_start: TRACK_VIEW_DEPTH * 0.5,
            fog_end: TRACK_VIEW_DEPTH,
            ambient: 300.0,
        },
        // Submerged: deep blue everywhere, murky short-range fog.
        Biome::Water => BiomePalette {
            clear: Color::srgb(0.04, 0.19, 0.33),
            fog_start: TRACK_VIEW_DEPTH * 0.2,
            fog_end: TRACK_VIEW_DEPTH * 0.85,
            ambient: 140.0,
        },
        // High altitude: brilliant cyan, crisp long sightlines.
        Biome::Sky => BiomePalette {
            clear: Color::srgb(0.47, 0.74, 0.98),
            fog_start: TRACK_VIEW_DEPTH * 0.6,
            fog_end: TRACK_VIEW_DEPTH * 1.2,
            ambient: 420.0,
        },
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_camera_and_light, setup_track))
            .add_systems(OnEnter(Biome::Ice), apply_biome_visuals)
            .add_systems(OnEnter(Biome::Water), apply_biome_visuals)
            .add_systems(OnEnter(Biome::Sky), apply_biome_visuals);
    }
}

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
        // Per-object motion blur sells the scroll speed: the world smears
        // past while the penguin stays sharp. (`..default()` also fills the
        // webgl2 padding field this struct grows on wasm builds.)
        MotionBlur {
            shutter_angle: MOTION_BLUR_SHUTTER,
            samples: MOTION_BLUR_SAMPLES,
            ..default()
        },
        // Motion blur and MSAA are incompatible under WebGL2, our shipping
        // target — disable MSAA everywhere for a consistent look.
        Msaa::Off,
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
    let solid = |base: Color| StandardMaterial {
        base_color: base,
        perceptual_roughness: 0.35,
        ..default()
    };
    // Sky lanes are translucent cloud roads rather than solid ground.
    let cloud = |base: Color| StandardMaterial {
        base_color: base,
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 1.0,
        ..default()
    };

    let track = TrackMaterials {
        ice: [
            materials.add(solid(Color::srgb(0.85, 0.93, 0.98))),
            materials.add(solid(Color::srgb(0.78, 0.88, 0.96))),
        ],
        water: [
            materials.add(solid(Color::srgb(0.07, 0.28, 0.42))),
            materials.add(solid(Color::srgb(0.05, 0.24, 0.38))),
        ],
        sky: [
            materials.add(cloud(Color::srgba(1.0, 1.0, 1.0, 0.32))),
            materials.add(cloud(Color::srgba(1.0, 1.0, 1.0, 0.45))),
        ],
    };

    let lane_mesh = meshes.add(Cuboid::new(LANE_WIDTH - 0.1, 0.2, TRACK_VIEW_DEPTH * 2.0));
    for lane in -1i8..=1 {
        let center = lane == 0;
        commands.spawn((
            LaneStrip { center },
            Mesh3d(lane_mesh.clone()),
            MeshMaterial3d(track.for_biome(Biome::Ice)[center as usize].clone()),
            Transform::from_xyz(lane as f32 * LANE_WIDTH, -0.1, -TRACK_VIEW_DEPTH * 0.5),
        ));
    }
    commands.insert_resource(track);
}

/// Re-theme the environment when a biome starts: clear color, fog,
/// ambient level, and lane materials all switch together.
///
/// `TrackMaterials` is `Option` because the initial `OnEnter(Ice)` fires in
/// `PreStartup`, before `setup_track` has run — the spawn defaults already
/// match the Ice palette, so skipping that first call is correct.
fn apply_biome_visuals(
    biome: Res<State<Biome>>,
    track: Option<Res<TrackMaterials>>,
    mut clear: ResMut<ClearColor>,
    mut ambient: ResMut<AmbientLight>,
    mut fog_query: Query<&mut DistanceFog, With<Camera3d>>,
    mut strips: Query<(&LaneStrip, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let Some(track) = track else {
        return;
    };
    let biome = *biome.get();
    let p = palette(biome);

    clear.0 = p.clear;
    ambient.brightness = p.ambient;
    for mut fog in &mut fog_query {
        fog.color = p.clear;
        fog.falloff = FogFalloff::Linear {
            start: p.fog_start,
            end: p.fog_end,
        };
    }
    let pair = track.for_biome(biome);
    for (strip, mut material) in &mut strips {
        material.0 = pair[strip.center as usize].clone();
    }
}
