# Penguin Cannon — Game Design Document

**Version 0.1** — initial design (changelog at bottom).

## Design Pillars

1. **Always readable** — the player can tell at a glance which lane they're in,
   what's coming, and how close the orca is.
2. **Pressure from behind** — the orca is the tension engine. Obstacles don't
   kill directly; they feed you to the orca.
3. **Biomes change the verb** — ice runs, water swims, sky glides. Same three
   lanes, different physics feel.
4. **The cannon is the fantasy** — being launched into the sky is the reward
   moment the whole run builds toward.

## Core Loop

### Moment-to-moment (0–30 s)
- **Action**: dodge between three lanes, jump/slide/dive past obstacles,
  collect fish and shrimps.
- **Feedback**: score ticks up per collectible; orca visibly closes on any hit.
- **Reward**: near-misses and clean streaks keep the orca at bay.

### Session loop (one run, 1–5 min)
- **Goal**: survive as long as possible; hit a cannon for the sky bonus phase.
- **Tension**: each obstacle hit slows the penguin and lets the orca surge.
- **Resolution**: the orca catches you → game over → score screen → restart.

## Mechanic Specifications

### Mechanic: Lane Dodge
- **Purpose**: primary avoidance verb across all biomes.
- **Input**: A/D, ←/→.
- **Output**: target lane index in {-1, 0, 1}; the penguin lerps toward
  `lane * LANE_WIDTH` on x.
- **Biome feel**: ice = slippery (slow lerp, slight overshoot), water = smooth,
  sky = fast and free.
- **Edge cases**: input at the outer lane is ignored; a second input mid-lerp
  retargets immediately.
- **Failure state**: none (movement never fails; obstacles punish position).

### Mechanic: Vertical Dodge (jump / slide / swim / dive)
- **Purpose**: second avoidance axis; differentiates obstacle types.
- **Input**: W/↑/Space = up, S/↓ = down.
- **Output**: ice — jump arc over low obstacles (holes, cracks); slide under
  nothing yet (reserved: lowers hitbox). Water — swim toward an upper or lower
  swim band. Sky — pitch up briefly slows altitude loss; dive speeds it up.
- **Edge cases**: jump input while airborne is ignored (no double jump);
  slide input while airborne queues nothing.
- **Failure state**: mistimed jump lands on the obstacle → counts as a hit.

### Mechanic: Orca Chase
- **Purpose**: converts obstacle hits into run-ending pressure.
- **State**: `orca_gap` in meters behind the player.
- **Rules**: gap recovers slowly while running clean; each obstacle hit closes
  the gap by a fixed amount and briefly slows the player. Gap ≤ catch distance
  → game over. Disabled in Sky.
- **Failure state**: orca reaches the penguin → Game Over.

### Mechanic: Cannon Power-up
- **Purpose**: the run's climax; converts danger into a bonus phase.
- **Trigger**: collide with a cannon object (rare spawn in Ice and Water).
- **Output**: biome → Sky, orca disabled, altitude set to max, altitude decays
  continuously; obstacles become planes and birds; hits accelerate altitude loss.
- **Exit**: altitude ≤ 0 → splashdown into Water, orca gap reset to a generous
  starting distance, chase resumes.

## Biome Transition Graph

```
        timed cycle              cannon pickup
  Ice ◄──────────────► Water ─────────────────► Sky
   │                     ▲                       │
   └── cannon pickup ────┼───────────────────────┘
                         └──── altitude ≤ 0 (splashdown)
```

- Ice ↔ Water alternate on a distance/time cycle (the ice shelf "ends" and the
  penguin dives in; later it climbs back out).
- Cannon can fire from either ground biome → Sky.
- Sky always drains back into Water.

## Tuning Table

All values are hypotheses until playtested. `[P]` = placeholder.

| Variable            | Base  | Min | Max | Notes                          |
|---------------------|-------|-----|-----|--------------------------------|
| Run speed (m/s)     | 12    | 8   | 24  | [P] ramps up over the run      |
| Speed ramp (m/s²)   | 0.05  | 0   | 0.2 | [P] slow difficulty creep      |
| Lane width (m)      | 2.5   | —   | —   | locked (readability)           |
| Lane lerp ice (1/s) | 6     | 3   | 12  | [P] slippery feel              |
| Lane lerp water     | 9     | 5   | 14  | [P]                            |
| Lane lerp sky       | 12    | 8   | 16  | [P] freest control             |
| Jump impulse (m/s)  | 7.5   | 5   | 10  | [P] clears 1-unit obstacles    |
| Gravity ice (m/s²)  | 22    | 15  | 30  | [P] snappy arcade arc          |
| Gravity water       | 6     | 3   | 10  | [P] floaty                     |
| Sky sink rate (m/s) | 2.2   | 1   | 5   | [P] sets bonus phase length    |
| Orca start gap (m)  | 18    | 10  | 30  | [P]                            |
| Orca catch gap (m)  | 1.5   | —   | —   | locked                         |
| Hit gap penalty (m) | 6     | 3   | 10  | [P] ~3 hits ends a clean run   |
| Gap regen (m/s)     | 0.8   | 0.3 | 2   | [P] slow forgiveness           |
| Hit slowdown        | 40%   | 20% | 60% | [P] for 1.2 s                  |
| Fish points         | 10    | —   | —   | locked                         |
| Shrimp points       | 5     | —   | —   | locked                         |
| Biome cycle (s)     | 30    | 20  | 60  | [P] ice/water alternation      |

## Changelog

- **0.1** — initial document: pillars, core loop, mechanic specs, transition
  graph, tuning table.
