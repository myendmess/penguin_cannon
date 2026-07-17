# Penguin Cannon — Audio Design Document

Per the Game Audio Engineer charter, adapted to Bevy's engine audio on
wasm (no FMOD/Wwise on the web — `src/audio.rs` plays the middleware role:
named events in, playback out, zero direct `AudioPlayer` spawns in
gameplay code).

## Sonic identity

**Crisp, cold, playful.** Short bright transients for rewards, soft broadband
beds for weather, one big low moment for the cannon.

## Sources

Every sound is synthesized by `tools/gen_sounds.py` (Python stdlib only,
fixed seed, reproducible) into 16-bit mono PCM WAV at 22.05 kHz —
`web/assets/sounds/`. Total footprint ≈ 0.9 MB. Charter format table says
PCM for latency-critical short SFX; WAV-PCM everywhere keeps the pipeline
dependency-free (no vorbis encoder needed).

## Event table

| Event (trigger) | File | Vol | Variation | Notes |
|---|---|---|---|---|
| `AudioCue::CollectFish` (pickup) | sfx_collect_fish | 0.50 | pitch ±6% | bright 2-note blip |
| `AudioCue::CollectShrimp` (pickup) | sfx_collect_shrimp | 0.45 | pitch ±6% | lower sibling |
| `AudioCue::Jump` (ice jump) | sfx_jump | 0.40 | pitch ±6% | rising sweep |
| `ObstacleHit` (collision) | sfx_hit | 0.80 | pitch ±6% | thud + crack; loudest SFX — it feeds the orca |
| `AudioCue::CannonLaunch` (pickup) | sfx_cannon | 0.90 | pitch ±6% | the run's climax; biggest sound in the game |
| `OnEnter(Water)` | sfx_splash | 0.60 | pitch ±6% | covers cycle dive AND sky splashdown |
| `OnEnter(GameOver)` | sfx_game_over | 0.70 | pitch ±6% | three falling minor notes |
| ambience `OnEnter(Ice)` | amb_ice loop | 0.35 | — | airy wind |
| ambience `OnEnter(Water)` | amb_water loop | 0.40 | — | low rumble + sparse bubbles |
| ambience `OnEnter(Sky)` | amb_sky loop | 0.30 | — | thin high wind |

All one-shots use `PlaybackMode::Despawn` (self-cleaning entities); loops
use `PlaybackMode::Loop` with integer-cycle LFOs authored into the files so
the wrap point is inaudible.

## Budget & constraints

- **Voice count**: bounded by design, not middleware — 1 ambience loop +
  short one-shots (≤0.9 s). Worst case ~6 simultaneous voices (collectible
  line + hit + ambience). No voice stealing needed at this scale.
- **Memory**: ~0.9 MB PCM, all decompressed (charter: SFX under 2 s live in
  memory).
- **Browser autoplay policy**: the WebAudio context may start suspended
  until the first user gesture. The game demands keyboard input immediately,
  so audio wakes on the first press; no custom resume shim shipped.
- **Not yet done** (future passes): adaptive music with a tension parameter
  driven by `OrcaGap`, spatial panning for lane-offset pickups, low-pass on
  the whole mix underwater.
