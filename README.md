# Penguin Cannon

An endless 3D lane-runner built with [Bevy](https://bevyengine.org/) and compiled
to WebAssembly. A South Pole penguin flees a hungry orca across three lanes of
ice and water — until a cannon power-up launches it toward the North Pole.

## Biomes

| Biome | Movement | Obstacles | Threat |
|-------|----------|-----------|--------|
| Ice   | Run, jump, belly-slide | Ice chunks, holes, cracks | Orca closing in from behind |
| Water | Swim up / dive | Submerged icebergs, giant crabs | Orca closing in from behind |
| Sky   | Glide, losing altitude | Airplanes, birds | None — bonus phase until splashdown |

Collect fish and shrimps for points. Hitting an obstacle slows you down and
lets the orca catch up. When it reaches you, it's game over.

## Controls

- **A / D** or **← / →** — switch lanes
- **W / ↑ / Space** — jump (ice), swim up (water), pitch up (sky)
- **S / ↓** — belly-slide (ice), dive (water/sky)
- **R / Enter** — restart after game over

## Building for the web

This repo is pinned (via `rustup override`) to the `stable-x86_64-pc-windows-gnu`
toolchain, which ships a self-contained linker — no Visual Studio Build Tools
required. `blake3` is forced to its pure-Rust implementation for the same
reason (no C compiler needed anywhere in the build).

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal
rustup target add wasm32-unknown-unknown --toolchain stable-x86_64-pc-windows-gnu
rustup override set stable-x86_64-pc-windows-gnu   # scoped to this directory
npm install -g wasm-pack
./build-wasm.ps1
python -m http.server 8000 -d web
# open http://localhost:8000
```

## Tests

State-transition logic is covered by unit tests:

```powershell
cargo test
```
