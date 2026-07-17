//! All game audio, in one place. Gameplay systems never touch playback —
//! they emit [`AudioCue`] events (or the existing [`ObstacleHit`]/state
//! transitions), and only this module spawns audio entities. That is the
//! Game Audio Engineer charter's middleware rule, adapted to Bevy's engine
//! audio: named events in, sound out.
//!
//! Every one-shot gets a small random pitch offset so nothing sounds
//! identical twice. Sources are procedurally generated PCM WAVs — see
//! tools/gen_sounds.py and docs/AUDIO.md.

use bevy::audio::{PlaybackMode, PlaybackSettings, Volume};
use bevy::prelude::*;

use crate::chase::ObstacleHit;
use crate::spawn::GameRng;
use crate::states::{Biome, RunState};

/// Named sound events. Gameplay writes these; the audio module listens.
#[derive(Event, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AudioCue {
    CollectFish,
    CollectShrimp,
    Jump,
    CannonLaunch,
}

#[derive(Resource)]
struct GameSounds {
    collect_fish: Handle<AudioSource>,
    collect_shrimp: Handle<AudioSource>,
    jump: Handle<AudioSource>,
    hit: Handle<AudioSource>,
    cannon: Handle<AudioSource>,
    splash: Handle<AudioSource>,
    game_over: Handle<AudioSource>,
    amb_ice: Handle<AudioSource>,
    amb_water: Handle<AudioSource>,
    amb_sky: Handle<AudioSource>,
}

/// Marker for the currently playing ambience loop entity.
#[derive(Component)]
struct AmbienceLoop;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AudioCue>()
            .add_systems(Startup, (load_sounds, start_ambience).chain())
            .add_systems(
                Update,
                (play_cues, play_hits).run_if(in_state(RunState::Running)),
            )
            // The initial OnEnter(Ice) fires in PreStartup, before sounds
            // are loaded — `start_ambience` covers the first loop, these
            // cover every later biome switch.
            .add_systems(OnEnter(Biome::Ice), swap_ambience)
            .add_systems(OnEnter(Biome::Water), (swap_ambience, play_splash))
            .add_systems(OnEnter(Biome::Sky), swap_ambience)
            .add_systems(OnEnter(RunState::GameOver), play_game_over);
    }
}

fn load_sounds(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(GameSounds {
        collect_fish: assets.load("sounds/sfx_collect_fish.wav"),
        collect_shrimp: assets.load("sounds/sfx_collect_shrimp.wav"),
        jump: assets.load("sounds/sfx_jump.wav"),
        hit: assets.load("sounds/sfx_hit.wav"),
        cannon: assets.load("sounds/sfx_cannon.wav"),
        splash: assets.load("sounds/sfx_splash.wav"),
        game_over: assets.load("sounds/sfx_game_over.wav"),
        amb_ice: assets.load("sounds/amb_ice.wav"),
        amb_water: assets.load("sounds/amb_water.wav"),
        amb_sky: assets.load("sounds/amb_sky.wav"),
    });
}

/// One-shot with the charter's "randomized container" treatment: slight
/// pitch variation per play.
fn spawn_sfx(
    commands: &mut Commands,
    rng: &mut GameRng,
    handle: &Handle<AudioSource>,
    volume: f32,
) {
    commands.spawn((
        AudioPlayer(handle.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::Linear(volume),
            speed: rng.range(0.94, 1.06),
            ..default()
        },
    ));
}

fn play_cues(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    sounds: Option<Res<GameSounds>>,
    mut cues: EventReader<AudioCue>,
) {
    let Some(sounds) = sounds else { return };
    for cue in cues.read() {
        let (handle, volume) = match cue {
            AudioCue::CollectFish => (&sounds.collect_fish, 0.5),
            AudioCue::CollectShrimp => (&sounds.collect_shrimp, 0.45),
            AudioCue::Jump => (&sounds.jump, 0.4),
            AudioCue::CannonLaunch => (&sounds.cannon, 0.9),
        };
        spawn_sfx(&mut commands, &mut rng, handle, volume);
    }
}

fn play_hits(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    sounds: Option<Res<GameSounds>>,
    mut hits: EventReader<ObstacleHit>,
) {
    let Some(sounds) = sounds else { return };
    for _ in hits.read() {
        spawn_sfx(&mut commands, &mut rng, &sounds.hit, 0.8);
    }
}

fn play_splash(mut commands: Commands, mut rng: ResMut<GameRng>, sounds: Option<Res<GameSounds>>) {
    let Some(sounds) = sounds else { return };
    spawn_sfx(&mut commands, &mut rng, &sounds.splash, 0.6);
}

fn play_game_over(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    sounds: Option<Res<GameSounds>>,
) {
    let Some(sounds) = sounds else { return };
    spawn_sfx(&mut commands, &mut rng, &sounds.game_over, 0.7);
}

fn start_ambience(
    commands: Commands,
    sounds: Option<Res<GameSounds>>,
    biome: Res<State<Biome>>,
    loops: Query<Entity, With<AmbienceLoop>>,
) {
    swap_ambience(commands, sounds, biome, loops);
}

/// Cross-biome ambience: stop the old bed, start the new one. Loops are
/// authored with integer-cycle LFOs so the wrap point is inaudible.
fn swap_ambience(
    mut commands: Commands,
    sounds: Option<Res<GameSounds>>,
    biome: Res<State<Biome>>,
    loops: Query<Entity, With<AmbienceLoop>>,
) {
    let Some(sounds) = sounds else { return };
    for entity in &loops {
        commands.entity(entity).despawn();
    }
    let (handle, volume) = match biome.get() {
        Biome::Ice => (&sounds.amb_ice, 0.35),
        Biome::Water => (&sounds.amb_water, 0.4),
        Biome::Sky => (&sounds.amb_sky, 0.3),
    };
    commands.spawn((
        AmbienceLoop,
        AudioPlayer(handle.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::Linear(volume),
            ..default()
        },
    ));
}
