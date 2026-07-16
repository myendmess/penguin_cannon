//! HUD (score, biome, orca distance / altitude) and the game-over overlay
//! with restart handling.

use bevy::prelude::*;

use crate::chase::OrcaGap;
use crate::player::{Player, TargetLane, Vertical};
use crate::spawn::{Mover, RunSpeed, Score, SpawnTimers};
use crate::states::{Altitude, Biome, BiomeCycle, RunState};
use crate::tuning::*;

#[derive(Component)]
struct ScoreText;

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
            .add_systems(OnEnter(RunState::GameOver), show_game_over)
            .add_systems(OnExit(RunState::GameOver), hide_game_over)
            .add_systems(
                Update,
                (update_score_text, update_biome_text, update_threat_text)
                    .run_if(in_state(RunState::Running)),
            )
            .add_systems(
                Update,
                restart_on_key.run_if(in_state(RunState::GameOver)),
            );
    }
}

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        ScoreText,
        Text::new("Score: 0"),
        TextFont {
            font_size: 24.0,
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
            font_size: 24.0,
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
            font_size: 20.0,
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
    // so the right-anchored text never collides with the score on narrow
    // canvases.
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

fn show_game_over(mut commands: Commands, score: Res<Score>) {
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
                Text::new("Press R or Enter to restart"),
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
/// its starting value, re-plant the penguin, and run it back from the ice.
fn restart_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    movers: Query<Entity, With<Mover>>,
    mut players: Query<(&mut Transform, &mut TargetLane, &mut Vertical), With<Player>>,
    mut next_biome: ResMut<NextState<Biome>>,
    mut next_run: ResMut<NextState<RunState>>,
) {
    if !(keys.just_pressed(KeyCode::KeyR) || keys.just_pressed(KeyCode::Enter)) {
        return;
    }
    for entity in &movers {
        commands.entity(entity).despawn();
    }
    commands.insert_resource(OrcaGap::default());
    commands.insert_resource(Score::default());
    commands.insert_resource(RunSpeed::default());
    commands.insert_resource(SpawnTimers::default());
    commands.insert_resource(Altitude::default());
    commands.insert_resource(BiomeCycle::default());
    for (mut transform, mut lane, mut vertical) in &mut players {
        transform.translation = Vec3::new(0.0, GROUND_Y, 0.0);
        transform.scale = Vec3::ONE;
        lane.0 = 0;
        *vertical = Vertical {
            grounded: true,
            ..default()
        };
    }
    next_biome.set(Biome::Ice);
    next_run.set(RunState::Running);
}
