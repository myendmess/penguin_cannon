//! Orca chase rules — the run's tension engine. Obstacles don't kill
//! directly; they close the gap and let the orca catch up (design pillar
//! "Pressure from behind"). The orca's on-screen body lives in the game
//! crate; everything here is headless logic.

use bevy::prelude::*;

use crate::states::{Biome, RunState, in_ground_biome};
use crate::tuning::*;

/// Meters between the orca and the penguin.
#[derive(Resource)]
pub struct OrcaGap(pub f32);

impl Default for OrcaGap {
    fn default() -> Self {
        Self(ORCA_START_GAP)
    }
}

/// Fired whenever the player clips an obstacle; the orca listens.
#[derive(Event)]
pub struct ObstacleHit;

/// Gap bookkeeping and the catch condition, active on the ground biomes.
pub struct ChasePlugin;

impl Plugin for ChasePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OrcaGap>()
            .add_event::<ObstacleHit>()
            // Splashing down grants breathing room before the chase resumes.
            .add_systems(
                OnTransition {
                    exited: Biome::Sky,
                    entered: Biome::Water,
                },
                grant_splashdown_gap,
            )
            .add_systems(
                Update,
                (update_gap, orca_catches)
                    .chain()
                    .run_if(in_state(RunState::Running))
                    .run_if(in_ground_biome),
            );
    }
}

fn grant_splashdown_gap(mut gap: ResMut<OrcaGap>) {
    gap.0 = gap.0.max(ORCA_SPLASHDOWN_GAP);
}

/// Clean running slowly earns distance back; every obstacle hit surrenders
/// a chunk of it.
fn update_gap(time: Res<Time>, mut hits: EventReader<ObstacleHit>, mut gap: ResMut<OrcaGap>) {
    gap.0 = (gap.0 + ORCA_GAP_REGEN * time.delta_secs()).min(ORCA_MAX_GAP);
    for _ in hits.read() {
        gap.0 -= ORCA_HIT_PENALTY;
    }
}

/// The catch: gap closed → game over.
fn orca_catches(gap: Res<OrcaGap>, mut next: ResMut<NextState<RunState>>) {
    if gap.0 <= ORCA_CATCH_GAP {
        next.set(RunState::GameOver);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::states::GameStatePlugin;
    use std::time::Duration;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.insert_resource(Time::<()>::default());
        app.add_plugins((GameStatePlugin, ChasePlugin));
        app
    }

    fn tick(app: &mut App, secs: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(secs));
        app.update();
    }

    fn run_state(app: &App) -> RunState {
        *app.world().resource::<State<RunState>>().get()
    }

    #[test]
    fn obstacle_hits_close_the_gap() {
        let mut app = test_app();
        app.update();
        let before = app.world().resource::<OrcaGap>().0;
        app.world_mut().send_event(ObstacleHit);
        tick(&mut app, 0.0);
        let after = app.world().resource::<OrcaGap>().0;
        assert!(
            (before - after - ORCA_HIT_PENALTY).abs() < 0.01,
            "hit should cost {ORCA_HIT_PENALTY}m: before={before} after={after}"
        );
    }

    #[test]
    fn orca_catches_player_when_gap_closes() {
        let mut app = test_app();
        app.update();
        app.world_mut().resource_mut::<OrcaGap>().0 = ORCA_CATCH_GAP + 0.1;
        app.world_mut().send_event(ObstacleHit);
        tick(&mut app, 0.0); // hit lands, gap drops below the catch distance
        tick(&mut app, 0.0); // catch fires, transition applies
        assert_eq!(run_state(&app), RunState::GameOver);
    }

    #[test]
    fn clean_running_regains_ground() {
        let mut app = test_app();
        app.update();
        app.world_mut().resource_mut::<OrcaGap>().0 = 10.0;
        tick(&mut app, 5.0);
        let gap = app.world().resource::<OrcaGap>().0;
        assert!(gap > 10.0, "gap should regenerate: {gap}");
        assert!(gap <= ORCA_MAX_GAP);
    }

    #[test]
    fn gap_never_exceeds_cap() {
        let mut app = test_app();
        app.update();
        tick(&mut app, 1000.0);
        assert!(app.world().resource::<OrcaGap>().0 <= ORCA_MAX_GAP);
    }

    #[test]
    fn chase_is_suspended_in_the_sky() {
        let mut app = test_app();
        app.update();
        app.world_mut()
            .resource_mut::<NextState<Biome>>()
            .set(Biome::Sky);
        tick(&mut app, 0.0);
        app.world_mut().resource_mut::<OrcaGap>().0 = 0.0;
        tick(&mut app, 1.0);
        tick(&mut app, 0.0);
        assert_eq!(
            run_state(&app),
            RunState::Running,
            "the orca must not catch anyone during the bonus phase"
        );
    }

    #[test]
    fn splashdown_grants_breathing_room() {
        let mut app = test_app();
        app.update();
        app.world_mut()
            .resource_mut::<NextState<Biome>>()
            .set(Biome::Sky);
        tick(&mut app, 0.0);
        app.world_mut().resource_mut::<OrcaGap>().0 = 3.0;
        // Drain the whole sky phase so it splashes down into water.
        tick(&mut app, SKY_ALTITUDE_MAX / SKY_SINK_RATE + 1.0);
        tick(&mut app, 0.0);
        assert_eq!(
            *app.world().resource::<State<Biome>>().get(),
            Biome::Water
        );
        assert!(
            app.world().resource::<OrcaGap>().0 >= ORCA_SPLASHDOWN_GAP,
            "splashdown must reset the orca to a generous distance"
        );
    }
}
