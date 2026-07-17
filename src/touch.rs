//! Touch controls for phones: swipe left/right to change lanes, swipe up
//! to jump / swim up / pitch up, swipe down to slide / dive, tap to
//! restart after a game over. Swipes emit [`TouchAction`] events; vertical
//! swipes additionally arm a short "held" window in [`TouchIntent`] so the
//! water/sky systems (which read held keys) respond to bursts of touch.

use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::tuning::*;

/// A recognized one-shot touch gesture.
#[derive(Event, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TouchAction {
    SwipeLeft,
    SwipeRight,
    SwipeUp,
    SwipeDown,
    Tap,
}

/// Emulates "held" vertical input for a short window after a vertical
/// swipe, so held-key movement (swim, sky pitch) works from bursts.
#[derive(Resource, Default)]
pub struct TouchIntent {
    up: Option<Timer>,
    down: Option<Timer>,
}

impl TouchIntent {
    pub fn up_held(&self) -> bool {
        self.up.as_ref().is_some_and(|t| !t.finished())
    }

    pub fn down_held(&self) -> bool {
        self.down.as_ref().is_some_and(|t| !t.finished())
    }

    fn press_up(&mut self) {
        self.up = Some(Timer::from_seconds(TOUCH_HOLD_SECS, TimerMode::Once));
        self.down = None;
    }

    fn press_down(&mut self) {
        self.down = Some(Timer::from_seconds(TOUCH_HOLD_SECS, TimerMode::Once));
        self.up = None;
    }
}

/// Per-touch gesture tracking state.
#[derive(Resource, Default)]
struct ActiveTouches {
    /// (touch id, start position, already classified as a swipe)
    touches: Vec<(u64, Vec2, bool)>,
}

pub struct TouchControlsPlugin;

impl Plugin for TouchControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TouchAction>()
            .init_resource::<TouchIntent>()
            .init_resource::<ActiveTouches>()
            .add_systems(Update, (read_touches, tick_intent));
    }
}

fn tick_intent(time: Res<Time>, mut intent: ResMut<TouchIntent>) {
    if let Some(timer) = &mut intent.up {
        timer.tick(time.delta());
    }
    if let Some(timer) = &mut intent.down {
        timer.tick(time.delta());
    }
}

fn read_touches(
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<ActiveTouches>,
    mut intent: ResMut<TouchIntent>,
    mut actions: EventWriter<TouchAction>,
) {
    for touch in touches.iter_just_pressed() {
        state.touches.push((touch.id(), touch.position(), false));
    }

    for touch in touches.iter() {
        let Some(entry) = state.touches.iter_mut().find(|(id, ..)| *id == touch.id()) else {
            continue;
        };
        if entry.2 {
            continue; // already classified
        }
        let delta = touch.position() - entry.1;
        if delta.length() < SWIPE_THRESHOLD_PX {
            continue;
        }
        entry.2 = true;
        // Screen space: +y is DOWN.
        let action = if delta.x.abs() > delta.y.abs() {
            if delta.x > 0.0 {
                TouchAction::SwipeRight
            } else {
                TouchAction::SwipeLeft
            }
        } else if delta.y < 0.0 {
            intent.press_up();
            TouchAction::SwipeUp
        } else {
            intent.press_down();
            TouchAction::SwipeDown
        };
        actions.write(action);
    }

    // Releases: an unclassified touch that ends quickly is a tap — but not
    // in the HUD strip at the top, where the mute button lives.
    let hud_cutoff = windows
        .single()
        .map(|w| w.height() * 0.18)
        .unwrap_or(80.0);
    let mut ended: Vec<u64> = Vec::new();
    for touch in touches.iter_just_released() {
        ended.push(touch.id());
        if let Some((_, start, classified)) =
            state.touches.iter().find(|(id, ..)| *id == touch.id())
        {
            if !classified && start.y > hud_cutoff {
                actions.write(TouchAction::Tap);
            }
        }
    }
    for touch in touches.iter_just_canceled() {
        ended.push(touch.id());
    }
    state.touches.retain(|(id, ..)| !ended.contains(id));
}
