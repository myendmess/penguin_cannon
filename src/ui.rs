//! HUD (score, biome, orca distance / altitude) and the game-over overlay
//! with restart handling.

use bevy::prelude::*;

use crate::audio::Muted;
use crate::chase::OrcaGap;
use crate::chaser::{CatchSequence, Orca, catch_finished};
use crate::player::{Player, TargetLane, Vertical};
use crate::spawn::{Mover, RunSpeed, Score, SpawnTimers};
use crate::states::{Altitude, Biome, BiomeCycle, RunState};
use crate::touch::TouchAction;
use crate::transitions::LayerTransition;
use crate::tuning::*;

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct MuteButton;

#[derive(Component)]
struct MuteLabel;

#[derive(Component)]
struct BiomeText;

#[derive(Component)]
struct ThreatText;

#[derive(Component)]
struct GameOverUi;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud)
            .add_systems(OnExit(RunState::GameOver), hide_game_over)
            .add_systems(
                Update,
                (update_score_text, update_biome_text, update_threat_text)
                    .run_if(in_state(RunState::Running)),
            )
            // The overlay waits for the orca's catch animation, and restart
            // (key or tap) only arms once the overlay is up.
            .add_systems(
                Update,
                (
                    show_game_over.run_if(catch_finished),
                    restart_on_input.run_if(catch_finished),
                )
                    .run_if(in_state(RunState::GameOver)),
            )
            .add_systems(Update, mute_button);
    }
}

/// Toggle the global mute from the HUD button (mouse or touch) or the
/// M key (desktop).
fn mute_button(
    mut muted: ResMut<Muted>,
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<&Interaction, (Changed<Interaction>, With<MuteButton>)>,
    mut labels: Query<&mut Text, With<MuteLabel>>,
) {
    let pressed = keys.just_pressed(KeyCode::KeyM)
        || interactions
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed);
    if !pressed {
        return;
    }
    muted.0 = !muted.0;
    for mut label in &mut labels {
        label.0 = if muted.0 { "SOUND OFF" } else { "SOUND ON" }.to_string();
    }
}

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        ScoreText,
        Text::new("Score: 0"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(14.0),
            ..default()
        },
    ));
    commands.spawn((
        BiomeText,
        Text::new("ICE"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(14.0),
            ..default()
        },
    ));
    commands.spawn((
        ThreatText,
        Text::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(14.0),
            ..default()
        },
    ));
    // Sound toggle, tucked under the biome label. Bevy's Button reacts to
    // both mouse and touch.
    commands
        .spawn((
            MuteButton,
            Button,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(44.0),
                right: Val::Px(14.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.55)),
            BorderRadius::all(Val::Px(6.0)),
            BackgroundColor(Color::srgba(0.0, 0.05, 0.12, 0.45)),
        ))
        .with_children(|parent| {
            parent.spawn((
                MuteLabel,
                Text::new("SOUND ON"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn update_score_text(score: Res<Score>, mut texts: Query<&mut Text, With<ScoreText>>) {
    if !score.is_changed() {
        return;
    }
    for mut text in &mut texts {
        text.0 = format!("Score: {}", score.points);
    }
}

fn update_biome_text(
    biome: Res<State<Biome>>,
    mut texts: Query<&mut Text, With<BiomeText>>,
) {
    if !biome.is_changed() {
        return;
    }
    // Keep labels short (and ASCII: the default font subset has no em-dash)
    // and fonts modest so the right-anchored text never collides with the
    // score, even on narrow phone canvases.
    let label = match biome.get() {
        Biome::Ice => "ICE",
        Biome::Water => "WATER",
        Biome::Sky => "SKY",
    };
    for mut text in &mut texts {
        text.0 = label.to_string();
    }
}

/// Bottom-left threat readout: orca distance on the ground (reddening as it
/// closes), remaining altitude in the sky.
fn update_threat_text(
    biome: Res<State<Biome>>,
    gap: Res<OrcaGap>,
    altitude: Res<Altitude>,
    mut texts: Query<(&mut Text, &mut TextColor), With<ThreatText>>,
) {
    let (label, color) = match biome.get() {
        Biome::Sky => (
            format!("Bonus! Altitude: {:.0} m", altitude.0),
            Color::srgb(1.0, 0.95, 0.6),
        ),
        _ => {
            let danger = 1.0 - ((gap.0 - ORCA_CATCH_GAP) / ORCA_START_GAP).clamp(0.0, 1.0);
            (
                format!("Orca: {:.0} m behind!", gap.0),
                Color::srgb(1.0, 1.0 - 0.7 * danger, 1.0 - 0.7 * danger),
            )
        }
    };
    for (mut text, mut text_color) in &mut texts {
        text.0 = label.clone();
        text_color.0 = color;
    }
}

fn show_game_over(
    mut commands: Commands,
    score: Res<Score>,
    existing: Query<Entity, With<GameOverUi>>,
) {
    if !existing.is_empty() {
        return;
    }
    commands
        .spawn((
            GameOverUi,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.04, 0.1, 0.7)),
            GlobalZIndex(10),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("THE ORCA GOT YOU"),
                TextFont {
                    font_size: 52.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.35, 0.3)),
            ));
            parent.spawn((
                Text::new(format!("Final score: {}", score.points)),
                TextFont {
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            parent.spawn((
                Text::new("Press R / Enter or tap to restart"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.85, 0.95)),
            ));
        });
}

fn hide_game_over(mut commands: Commands, overlays: Query<Entity, With<GameOverUi>>) {
    for entity in &overlays {
        commands.entity(entity).despawn();
    }
}

/// Full reset: sweep spawned entities, restore every gameplay resource to
/// its starting value, re-plant the penguin (and un-lunge the orca), and
/// run it back from the ice. Triggered by R/Enter or a tap.
fn restart_on_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut taps: EventReader<TouchAction>,
    mut commands: Commands,
    movers: Query<Entity, With<Mover>>,
    mut players: Query<(&mut Transform, &mut TargetLane, &mut Vertical), With<Player>>,
    mut orcas: Query<&mut Transform, (With<Orca>, Without<Player>)>,
    mut next_biome: ResMut<NextState<Biome>>,
    mut next_run: ResMut<NextState<RunState>>,
) {
    let tapped = taps.read().any(|action| *action == TouchAction::Tap);
    if !(keys.just_pressed(KeyCode::KeyR) || keys.just_pressed(KeyCode::Enter) || tapped) {
        return;
    }
    for entity in &movers {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<CatchSequence>();
    commands.remove_resource::<LayerTransition>();
    commands.insert_resource(OrcaGap::default());
    commands.insert_resource(Score::default());
    commands.insert_resource(RunSpeed::default());
    commands.insert_resource(SpawnTimers::default());
    commands.insert_resource(Altitude::default());
    commands.insert_resource(BiomeCycle::default());
    for (mut transform, mut lane, mut vertical) in &mut players {
        transform.translation = Vec3::new(0.0, GROUND_Y, 0.0);
        transform.scale = Vec3::ONE;
        transform.rotation = Quat::IDENTITY;
        lane.0 = 0;
        *vertical = Vertical {
            grounded: true,
            ..default()
        };
    }
    for mut transform in &mut orcas {
        transform.translation = Vec3::new(0.0, -0.1, ORCA_VISUAL_Z.1);
        transform.rotation = Quat::IDENTITY;
    }
    next_biome.set(Biome::Ice);
    next_run.set(RunState::Running);
}
