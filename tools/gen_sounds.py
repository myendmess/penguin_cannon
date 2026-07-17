"""Procedurally generates every sound in the game as 16-bit mono PCM WAVs.

No third-party audio assets, no dependencies beyond the Python stdlib —
run it once and commit the output:

    python tools/gen_sounds.py

Output goes to web/assets/sounds/. Design notes live in docs/AUDIO.md.
"""

import math
import random
import struct
import wave
from pathlib import Path

SAMPLE_RATE = 22050
OUT_DIR = Path(__file__).resolve().parent.parent / "web" / "assets" / "sounds"

rng = random.Random(0x9E3779B9)  # fixed seed: regeneration is reproducible


def write_wav(name: str, samples: list[float], peak: float) -> None:
    """Normalize to `peak` and write 16-bit mono PCM."""
    top = max(abs(s) for s in samples) or 1.0
    scale = peak / top
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    path = OUT_DIR / f"{name}.wav"
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SAMPLE_RATE)
        frames = b"".join(
            struct.pack("<h", int(max(-1.0, min(1.0, s * scale)) * 32767))
            for s in samples
        )
        w.writeframes(frames)
    print(f"  {path.name}  ({len(samples) / SAMPLE_RATE:.2f}s)")


def secs(duration: float) -> range:
    return range(int(duration * SAMPLE_RATE))


def env_decay(i: int, duration: float, sharpness: float = 6.0) -> float:
    """Exponential decay envelope over `duration` seconds."""
    return math.exp(-sharpness * i / (duration * SAMPLE_RATE))


class LowPass:
    """One-pole low-pass, good enough for shaping noise."""

    def __init__(self, cutoff_hz: float):
        c = 2.0 * math.pi * cutoff_hz / SAMPLE_RATE
        self.a = c / (c + 1.0)
        self.y = 0.0

    def step(self, x: float) -> float:
        self.y += self.a * (x - self.y)
        return self.y


def sine_blip(freqs: list[float], note_secs: float, sharpness: float = 7.0) -> list[float]:
    """Consecutive decaying sine notes — the collectible 'blip' family."""
    out = []
    for f in freqs:
        phase = 0.0
        for i in secs(note_secs):
            phase += 2.0 * math.pi * f / SAMPLE_RATE
            out.append(math.sin(phase) * env_decay(i, note_secs, sharpness))
    return out


def collect_fish() -> list[float]:
    return sine_blip([880.0, 1318.5], 0.07)


def collect_shrimp() -> list[float]:
    return sine_blip([659.3, 880.0], 0.06)


def jump() -> list[float]:
    """Rising sine sweep with a whisper of noise: effort, upward motion."""
    duration = 0.14
    out = []
    phase = 0.0
    lp = LowPass(1200.0)
    for i in secs(duration):
        t = i / (duration * SAMPLE_RATE)
        f = 300.0 + 400.0 * t
        phase += 2.0 * math.pi * f / SAMPLE_RATE
        tone = math.sin(phase)
        hiss = lp.step(rng.uniform(-1, 1)) * 0.25
        fade = 1.0 - t
        out.append((tone * 0.8 + hiss) * fade)
    return out


def hit() -> list[float]:
    """Low thud + short noise crack: unmistakably a mistake."""
    duration = 0.22
    out = []
    phase = 0.0
    lp = LowPass(900.0)
    for i in secs(duration):
        phase += 2.0 * math.pi * 110.0 / SAMPLE_RATE
        thud = math.sin(phase) * env_decay(i, duration, 9.0)
        crack = lp.step(rng.uniform(-1, 1)) * env_decay(i, 0.05, 6.0) * 0.7
        out.append(thud + crack)
    return out


def cannon() -> list[float]:
    """Big boom: heavy noise body over a sine that falls through the floor."""
    duration = 0.65
    out = []
    phase = 0.0
    lp = LowPass(500.0)
    for i in secs(duration):
        t = i / (duration * SAMPLE_RATE)
        f = 150.0 - 100.0 * t
        phase += 2.0 * math.pi * max(f, 30.0) / SAMPLE_RATE
        boom = math.sin(phase) * env_decay(i, duration, 4.0)
        body = lp.step(rng.uniform(-1, 1)) * env_decay(i, duration, 5.0)
        out.append(boom * 0.7 + body * 0.8)
    return out


def splash() -> list[float]:
    """Band-shaped noise with a wobble — water swallowing the penguin."""
    duration = 0.4
    out = []
    lp = LowPass(2400.0)
    hp_state = 0.0
    for i in secs(duration):
        t = i / (duration * SAMPLE_RATE)
        n = lp.step(rng.uniform(-1, 1))
        # crude high-pass: subtract a slower low-pass of the same signal
        hp_state += 0.02 * (n - hp_state)
        band = n - hp_state
        wobble = 0.75 + 0.25 * math.sin(2.0 * math.pi * 9.0 * t)
        out.append(band * wobble * env_decay(i, duration, 4.5))
    return out


def game_over() -> list[float]:
    """Three falling notes, minor and final."""
    out = []
    for f in [220.0, 174.6, 146.8]:
        phase = 0.0
        note = 0.3
        for i in secs(note):
            phase += 2.0 * math.pi * f / SAMPLE_RATE
            s = math.sin(phase) + 0.35 * math.sin(2.0 * phase)
            out.append(s * env_decay(i, note, 4.0))
    return out


def ambience(cutoff: float, lfo_cycles: int, bubbles: bool, duration: float = 6.0) -> list[float]:
    """Seamless noise-bed loop. The amplitude LFO completes an integer
    number of cycles over the loop, so the wrap point is inaudible."""
    out = []
    lp = LowPass(cutoff)
    n_samples = int(duration * SAMPLE_RATE)
    bubble_at = sorted(rng.randrange(n_samples) for _ in range(14)) if bubbles else []
    bubble_phase, bubble_env, bubble_freq = 0.0, 0.0, 800.0
    next_bubble = 0
    for i in range(n_samples):
        t = i / n_samples
        lfo = 0.7 + 0.3 * math.sin(2.0 * math.pi * lfo_cycles * t)
        s = lp.step(rng.uniform(-1, 1)) * lfo
        if bubbles:
            if next_bubble < len(bubble_at) and i >= bubble_at[next_bubble]:
                next_bubble += 1
                bubble_env = 1.0
                bubble_freq = rng.uniform(500.0, 1300.0)
                bubble_phase = 0.0
            if bubble_env > 0.001:
                bubble_phase += 2.0 * math.pi * bubble_freq / SAMPLE_RATE
                s += math.sin(bubble_phase) * bubble_env * 0.2
                bubble_env *= 0.9995
        out.append(s)
    return out


def main() -> None:
    print("Generating sounds:")
    write_wav("sfx_collect_fish", collect_fish(), 0.55)
    write_wav("sfx_collect_shrimp", collect_shrimp(), 0.45)
    write_wav("sfx_jump", jump(), 0.5)
    write_wav("sfx_hit", hit(), 0.75)
    write_wav("sfx_cannon", cannon(), 0.85)
    write_wav("sfx_splash", splash(), 0.6)
    write_wav("sfx_game_over", game_over(), 0.6)
    write_wav("amb_ice", ambience(cutoff=700.0, lfo_cycles=2, bubbles=False), 0.3)
    write_wav("amb_water", ambience(cutoff=350.0, lfo_cycles=3, bubbles=True), 0.35)
    write_wav("amb_sky", ambience(cutoff=1500.0, lfo_cycles=3, bubbles=False), 0.25)
    print("Done.")


if __name__ == "__main__":
    main()
