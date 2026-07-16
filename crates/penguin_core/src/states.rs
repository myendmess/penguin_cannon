//! The biome state machine and every transition rule between biomes.
//!
//! Transition graph (see docs/GDD.md):
//!
//! ```text
//!         timed cycle              cannon pickup
//!   Ice ◄──────────────► Water ─────────────────► Sky
//!    │                     ▲                       │
//!    └── cannon pickup ────┼───────────────────────┘
//!                          └──── altitude ≤ 0 (splashdown)
//! ```

use bevy::prelude::*;

use crate::tuning::*;

/// The three environments the penguin runs, swims, and flies through.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Biome {
    /// Ground level: running and belly-sliding on the ice shelf.
    #[default]
    Ice,
    /// Submerged: swimming between icebergs and crabs.
    Water,
    /// Cannon bonus phase: gliding, orca disabled, altitude draining.
    Sky,
}

/// Whether the run is live or the orca has won.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RunState {
    #[default]
    Running,
    GameOver,
}

/// Remaining altitude during the Sky bonus phase, in abstract meters.
#[derive(Resource, Default)]
pub struct Altitude(pub f32);

/// Timer driving the timed Ice <-> Water alternation (the ice shelf "ends"
/// and the penguin dives in; later it climbs back out).
#[derive(Resource)]
pub struct BiomeCycle(pub Timer);

impl Default for BiomeCycle {
    fn default() -> Self {
        Self(Timer::from_seconds(BIOME_CYCLE_SECS, TimerMode::Repeating))
    }
}

/// Vertical intent during the Sky phase. Written by the input system,
/// read by [`drain_altitude`] — separated so transition logic stays
/// testable without a keyboard.
#[derive(Resource, Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum SkyPitch {
    #[default]
    Level,
    /// Pitching up: sink slower.
    Up,
    /// Diving: sink faster.
    Dive,
}

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Biome>()
            .init_state::<RunState>()
            .init_resource::<Altitude>()
            .init_resource::<BiomeCycle>()
            .init_resource::<SkyPitch>()
            .add_systems(
                Update,
                (
                    cycle_ground_biomes
                        .run_if(in_state(RunState::Running))
                        .run_if(in_ground_biome),
                    drain_altitude
                        .run_if(in_state(RunState::Running))
                        .run_if(in_state(Biome::Sky)),
                ),
            )
            .add_systems(OnEnter(Biome::Sky), enter_sky);
    }
}

/// Run condition: the penguin is on (or under) the surface, where the orca
/// hunts and the ice/water cycle applies.
pub fn in_ground_biome(biome: Res<State<Biome>>) -> bool {
    matches!(biome.get(), Biome::Ice | Biome::Water)
}

/// The ice shelf periodically ends and the penguin dives into the sea —
/// and later climbs back out onto the next shelf.
pub fn cycle_ground_biomes(
    time: Res<Time>,
    mut cycle: ResMut<BiomeCycle>,
    biome: Res<State<Biome>>,
    mut next: ResMut<NextState<Biome>>,
) {
    if cycle.0.tick(time.delta()).just_finished() {
        next.set(match biome.get() {
            Biome::Ice => Biome::Water,
            _ => Biome::Ice,
        });
    }
}

/// Sky bonus phase: the penguin continuously loses altitude until it drops
/// back into the water, where the orca chase resumes.
pub fn drain_altitude(
    time: Res<Time>,
    pitch: Res<SkyPitch>,
    mut altitude: ResMut<Altitude>,
    mut next: ResMut<NextState<Biome>>,
) {
    let factor = match *pitch {
        SkyPitch::Level => 1.0,
        SkyPitch::Up => SKY_PITCH_UP_FACTOR,
        SkyPitch::Dive => SKY_DIVE_FACTOR,
    };
    altitude.0 -= SKY_SINK_RATE * factor * time.delta_secs();
    if altitude.0 <= 0.0 {
        altitude.0 = 0.0;
        next.set(Biome::Water);
    }
}

/// Entering the sky grants full altitude and restarts the biome cycle so the
/// post-splashdown water stretch gets its full duration.
fn enter_sky(mut altitude: ResMut<Altitude>, mut cycle: ResMut<BiomeCycle>) {
    altitude.0 = SKY_ALTITUDE_MAX;
    cycle.0.reset();
}

/// The cannon pickup fires the penguin skyward. Called by the collision
/// system; kept as a plain function so tests can exercise it directly.
pub fn launch_to_sky(next: &mut NextState<Biome>) {
    next.set(Biome::Sky);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Headless app with real state transitions but hand-controlled time.
    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_resource(Time::<()>::default());
        app.add_plugins(GameStatePlugin);
        app
    }

    /// Advance the clock by `secs` and run one frame. Use `tick(app, 0.0)`
    /// to apply a pending state transition without advancing time.
    fn tick(app: &mut App, secs: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(secs));
        app.update();
    }

    fn biome(app: &App) -> Biome {
        *app.world().resource::<State<Biome>>().get()
    }

    #[test]
    fn starts_on_ice() {
        let mut app = test_app();
        app.update();
        assert_eq!(biome(&app), Biome::Ice);
    }

    #[test]
    fn ground_biomes_alternate_on_cycle_timer() {
        let mut app = test_app();
        app.update();

        tick(&mut app, BIOME_CYCLE_SECS + 0.1);
        tick(&mut app, 0.0);
        assert_eq!(biome(&app), Biome::Water, "ice shelf should end in water");

        tick(&mut app, BIOME_CYCLE_SECS + 0.1);
        tick(&mut app, 0.0);
        assert_eq!(biome(&app), Biome::Ice, "penguin should climb back out");
    }

    #[test]
    fn cannon_launch_reaches_sky_with_full_altitude() {
        let mut app = test_app();
        app.update();

        launch_to_sky(&mut app.world_mut().resource_mut::<NextState<Biome>>());
        tick(&mut app, 0.0);
        assert_eq!(biome(&app), Biome::Sky);
        assert_eq!(
            app.world().resource::<Altitude>().0,
            SKY_ALTITUDE_MAX,
            "OnEnter(Sky) must grant full altitude"
        );
    }

    #[test]
    fn sky_splashes_down_into_water_when_altitude_runs_out() {
        let mut app = test_app();
        app.update();
        launch_to_sky(&mut app.world_mut().resource_mut::<NextState<Biome>>());
        tick(&mut app, 0.0);

        // Drain more than the full altitude's worth of time.
        tick(&mut app, SKY_ALTITUDE_MAX / SKY_SINK_RATE + 1.0);
        tick(&mut app, 0.0);
        assert_eq!(biome(&app), Biome::Water, "splashdown must land in water");
        assert_eq!(app.world().resource::<Altitude>().0, 0.0);
    }

    #[test]
    fn biome_cycle_is_paused_in_the_sky() {
        let mut app = test_app();
        app.update();
        launch_to_sky(&mut app.world_mut().resource_mut::<NextState<Biome>>());
        tick(&mut app, 0.0);

        // Longer than the ground cycle, shorter than the full sky phase.
        tick(&mut app, BIOME_CYCLE_SECS + 1.0);
        tick(&mut app, 0.0);
        assert_eq!(
            biome(&app),
            Biome::Sky,
            "the ice/water cycle must not run during the bonus phase"
        );
    }

    #[test]
    fn pitching_up_sinks_slower_than_diving() {
        let drained_after = |pitch: SkyPitch| {
            let mut app = test_app();
            app.update();
            launch_to_sky(&mut app.world_mut().resource_mut::<NextState<Biome>>());
            tick(&mut app, 0.0);
            *app.world_mut().resource_mut::<SkyPitch>() = pitch;
            tick(&mut app, 10.0);
            app.world().resource::<Altitude>().0
        };

        let up = drained_after(SkyPitch::Up);
        let level = drained_after(SkyPitch::Level);
        let dive = drained_after(SkyPitch::Dive);
        assert!(up > level && level > dive, "up={up} level={level} dive={dive}");
    }
}
