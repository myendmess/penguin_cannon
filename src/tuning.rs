//! Central tuning table. Every gameplay number lives here and mirrors
//! docs/GDD.md ("Tuning Table"). Values marked [PLACEHOLDER] in the GDD
//! are hypotheses awaiting playtesting — adjust them here, nowhere else.

/// Center-to-center spacing of the three run lanes, in meters. Locked for
/// readability — obstacles and the penguin are sized against this.
pub const LANE_WIDTH: f32 = 2.5;

/// How far ahead of the player the track extends before fog swallows it.
pub const TRACK_VIEW_DEPTH: f32 = 90.0;

// --- Forward motion -------------------------------------------------------

/// Starting forward speed (m/s). [PLACEHOLDER]
pub const RUN_SPEED_BASE: f32 = 12.0;
/// Speed gained per second of clean running (m/s^2). [PLACEHOLDER]
pub const RUN_SPEED_RAMP: f32 = 0.05;
/// Hard cap on forward speed (m/s). [PLACEHOLDER]
pub const RUN_SPEED_MAX: f32 = 24.0;
/// Fraction of speed kept while the post-hit slowdown is active. [PLACEHOLDER]
pub const HIT_SLOWDOWN_FACTOR: f32 = 0.6;
/// Duration of the post-hit slowdown, seconds. [PLACEHOLDER]
pub const HIT_SLOWDOWN_SECS: f32 = 1.2;

// --- Lane feel per biome (lerp rate toward target lane, 1/s) --------------

/// Slippery: the penguin drifts into lane changes. [PLACEHOLDER]
pub const LANE_LERP_ICE: f32 = 6.0;
/// Smooth, watery response. [PLACEHOLDER]
pub const LANE_LERP_WATER: f32 = 9.0;
/// Freest control while airborne. [PLACEHOLDER]
pub const LANE_LERP_SKY: f32 = 12.0;

// --- Vertical feel ---------------------------------------------------------

/// Jump take-off velocity on ice (m/s). [PLACEHOLDER]
pub const JUMP_IMPULSE: f32 = 7.5;
/// Snappy arcade gravity on ice (m/s^2). [PLACEHOLDER]
pub const GRAVITY_ICE: f32 = 22.0;
/// Floaty gravity underwater (m/s^2). [PLACEHOLDER]
pub const GRAVITY_WATER: f32 = 6.0;
/// Ground height of the penguin's root on ice.
pub const GROUND_Y: f32 = 0.75;
/// Vertical swim band underwater: min and max root height.
pub const SWIM_BAND: (f32, f32) = (0.9, 3.4);
/// Vertical speed of swim up / dive input (m/s). [PLACEHOLDER]
pub const SWIM_SPEED: f32 = 4.0;

// --- Sky (cannon bonus phase) ----------------------------------------------

/// Altitude granted by the cannon launch, in abstract meters. [PLACEHOLDER]
pub const SKY_ALTITUDE_MAX: f32 = 100.0;
/// Passive altitude loss per second. [PLACEHOLDER] Sets bonus phase length
/// (~45 s at base sink).
pub const SKY_SINK_RATE: f32 = 2.2;
/// Sink multiplier while pitching up (W/Up held). [PLACEHOLDER]
pub const SKY_PITCH_UP_FACTOR: f32 = 0.4;
/// Sink multiplier while diving (S/Down held). [PLACEHOLDER]
pub const SKY_DIVE_FACTOR: f32 = 2.5;
/// Altitude lost instantly when hitting a bird or plane. [PLACEHOLDER]
pub const SKY_HIT_ALTITUDE_PENALTY: f32 = 15.0;
/// Player root height at zero altitude / max altitude (visual mapping).
pub const SKY_Y_RANGE: (f32, f32) = (1.5, 4.5);

// --- Orca chase -------------------------------------------------------------

/// Gap between orca and penguin at run start, meters. [PLACEHOLDER]
pub const ORCA_START_GAP: f32 = 18.0;
/// Gap at which the orca catches the penguin. Locked.
pub const ORCA_CATCH_GAP: f32 = 1.5;
/// Gap lost per obstacle hit, meters. [PLACEHOLDER] (~3 hits ends a clean run)
pub const ORCA_HIT_PENALTY: f32 = 6.0;
/// Gap slowly regained per second of clean running. [PLACEHOLDER]
pub const ORCA_GAP_REGEN: f32 = 0.8;
/// Generous gap granted on splashdown from the sky. [PLACEHOLDER]
pub const ORCA_SPLASHDOWN_GAP: f32 = 24.0;
/// Clean running can't push the orca further back than this. [PLACEHOLDER]
pub const ORCA_MAX_GAP: f32 = 28.0;
/// Visual mapping: orca's world-Z when caught / at full start gap.
/// (The camera sits at z=9 looking down -Z; the orca lives between the
/// player at z=0 and the camera so the threat is always on screen.)
pub const ORCA_VISUAL_Z: (f32, f32) = (0.9, 6.5);

// --- Biome cycle -------------------------------------------------------------

/// Seconds between the timed Ice <-> Water alternation. [PLACEHOLDER]
pub const BIOME_CYCLE_SECS: f32 = 30.0;

// --- Scoring ------------------------------------------------------------------

/// Points per fish. Locked.
pub const FISH_POINTS: u32 = 10;
/// Points per shrimp. Locked.
pub const SHRIMP_POINTS: u32 = 5;
/// Meters of forward travel worth one point.
pub const METERS_PER_POINT: f32 = 10.0;

// --- Spawning -----------------------------------------------------------------

/// Obstacle spawn interval range, seconds. [PLACEHOLDER]
pub const OBSTACLE_INTERVAL: (f32, f32) = (0.8, 1.6);
/// Collectible line spawn interval range, seconds. [PLACEHOLDER]
pub const COLLECTIBLE_INTERVAL: (f32, f32) = (2.0, 4.0);
/// Cannon spawn interval range, seconds (ground biomes only). [PLACEHOLDER]
pub const CANNON_INTERVAL: (f32, f32) = (20.0, 35.0);
/// Ambient decor (pebbles/bubbles/clouds) spawn interval range, seconds.
pub const DECOR_INTERVAL: (f32, f32) = (0.15, 0.4);
/// Z position where spawned objects appear (far end of the fog).
pub const SPAWN_Z: f32 = -TRACK_VIEW_DEPTH;
/// Z position behind the camera where objects despawn.
pub const DESPAWN_Z: f32 = 14.0;
