#!/usr/bin/env python3
"""Composes and renders the Jibaru Editor promo soundtrack from code.

Every sound is synthesized here (oscillators, filtered noise, a generated reverb
impulse); no samples or third-party audio. The output is deterministic (fixed
seed) and is dedicated to the public domain under CC0 1.0 (see MUSIC-LICENSE.md).

Track: 120 BPM, F minor (i-VI-III-VII: Fm Db Ab Eb), 30 s, 48 kHz stereo,
mastered to about -14 LUFS integrated with true peak <= -1.5 dBTP.

Timeline (video frames at 30 fps; 1 beat = 15 frames, 1 bar = 60 frames):
  0-54     build: Eb pad opening, snare roll, risers, gap, then
  54       DROP (bar 0 downbeat). Every engine-reel cut (54 + 30k) is on a beat.
  234-294  counter: riser bar, snare roll, gap
  294      drop 2 (editor), arp layer joins
  654-705  platforms: groove thins, filter closes
  705      break (AI/MCP): grid re-phases so 705 is a beat and 810 a downbeat
  750-810  build: riser + roll
  810      final hit (outro), one last bar, final stab at 870, tail done by 900

Usage: python scripts/compose-music.py [out.wav]   (needs numpy + scipy; ffmpeg for mp3)
Default: writes out/jibaru-promo-music.wav, then (if ffmpeg is on PATH) the 320 kbps
mp3s out/jibaru-promo-music.mp3 and public/music/jibaru-promo-music.mp3 (used by Promo.tsx).
"""
import os
import shutil
import subprocess
import sys

import numpy as np
from scipy import signal
from scipy.io import wavfile
from scipy.ndimage import minimum_filter1d, uniform_filter1d

SR = 48000
FPS = 30
SPF = SR // FPS  # samples per video frame
TOTAL = 900 * SPF
BEAT = SR // 2  # 120 BPM
STEP = BEAT // 4  # 16th note
RNG = np.random.default_rng(0x71BA2)
TARGET_LUFS = -14.0
CEIL_DBTP = -1.5


def F(frame):
    return int(round(frame * SPF))


def OB(b):
    """Sample of beat b on the main grid (beat 0 = frame 54, the drop)."""
    return F(54) + int(round(b * BEAT))


def NB(b):
    """Sample of beat b on the break/outro grid (beat 0 = frame 705, 810 = beat 7)."""
    return F(705) + int(round(b * BEAT))


def hz(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def tt(n):
    return np.arange(n) / SR


# ----------------------------------------------------------------- utilities
class Bus:
    def __init__(self):
        self.x = np.zeros((2, TOTAL))

    def add(self, sig, start, gain=1.0, pan=0.0):
        sig = np.asarray(sig, dtype=float)
        if sig.ndim == 1:
            a = (pan + 1) * np.pi / 4
            sig = np.stack([sig * np.cos(a), sig * np.sin(a)]) * np.sqrt(2)
        start = int(start)
        s0, s1 = max(start, 0), min(start + sig.shape[1], TOTAL)
        if s1 <= s0:
            return
        self.x[:, s0:s1] += gain * sig[:, s0 - start:s1 - start]


def sos(kind, f, order=2):
    return signal.butter(order, f, btype=kind, fs=SR, output="sos")


def filt(x, kind, f, order=2):
    return signal.sosfilt(sos(kind, f, order), x, axis=-1)


def rbj_lp(fc, q):
    w = 2 * np.pi * min(fc, SR * 0.45) / SR
    al = np.sin(w) / (2 * q)
    c = np.cos(w)
    b = np.array([(1 - c) / 2, 1 - c, (1 - c) / 2])
    a = np.array([1 + al, -2 * c, 1 - al])
    return np.concatenate([b / a[0], a / a[0]])[None, :]


def tv_lowpass(x, fc, q=0.8, block=64):
    """Time-varying resonant lowpass; fc is an array (Hz) per sample."""
    x = np.atleast_2d(x)
    y = np.zeros_like(x)
    zi = np.zeros((1, x.shape[0], 2))
    for i in range(0, x.shape[1], block):
        s = rbj_lp(fc[min(i + block // 2, len(fc) - 1)], q)
        y[:, i:i + block], zi = signal.sosfilt(s, x[:, i:i + block], axis=-1, zi=zi)
    return y


def env_adsr(n, a=0.005, d=0.1, s=0.7, r=0.05):
    t = tt(n)
    e = np.where(t < a, t / a, s + (1 - s) * np.exp(-(t - a) / max(d, 1e-4)))
    rn = int(r * SR)
    if rn and rn < n:
        e[-rn:] *= np.linspace(1, 0, rn) ** 2
    return e


def fade_ends(x, a=0.002, r=0.01):
    n = x.shape[-1]
    an, rn = min(int(a * SR), n), min(int(r * SR), n)
    x[..., :an] *= np.linspace(0, 1, an)
    x[..., n - rn:] *= np.linspace(1, 0, rn)
    return x


def saw(freq, n, ph0=0.0):
    """PolyBLEP band-limited saw. freq is a scalar or per-sample array."""
    dt = np.broadcast_to(np.asarray(freq, float) / SR, (n,))
    ph = (ph0 + np.cumsum(dt)) % 1.0
    y = 2 * ph - 1
    m = ph < dt
    t = ph[m] / dt[m]
    y[m] -= t + t - t * t - 1
    m = ph > 1 - dt
    t = (ph[m] - 1) / dt[m]
    y[m] -= t * t + t + t + 1
    return y


def noise(n):
    return RNG.standard_normal(n)


# ---------------------------------------------------------------- instruments
def kick(vel=1.0, length=0.42):
    n = int(length * SR)
    t = tt(n)
    f = 46 + 120 * np.exp(-t / 0.028) + 500 * np.exp(-t / 0.0025)
    body = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.24)
    click = filt(noise(n), "highpass", 2500) * np.exp(-t / 0.0025) * 0.35
    y = np.tanh(1.8 * (body + click)) / np.tanh(1.8)
    return fade_ends(y * vel, 0.0004, 0.03)


def clap(vel=1.0):
    n = int(0.35 * SR)
    t = tt(n)
    e = sum(np.where(t >= o, np.exp(-(t - o) / 0.0045), 0) for o in (0, 0.009, 0.018))
    e = e * (t < 0.026) + np.where(t >= 0.026, 0.9 * np.exp(-(t - 0.026) / 0.07), 0)
    y = filt(noise(n), "bandpass", [850, 3200]) * e
    y += filt(noise(n), "highpass", 5000) * np.exp(-t / 0.03) * 0.25
    return fade_ends(y / np.max(np.abs(y)) * vel)


def snare(vel=1.0, tone=190.0, length=0.22):
    n = int(length * SR)
    t = tt(n)
    body = np.sin(2 * np.pi * np.cumsum(tone * (1 + 0.4 * np.exp(-t / 0.01))) / SR) * np.exp(-t / 0.05)
    nz = filt(noise(n), "bandpass", [1200, 7500]) * np.exp(-t / 0.07)
    y = 0.6 * body + 0.9 * nz / np.max(np.abs(nz))
    return fade_ends(y * vel)


HAT_F = np.array([263.0, 400.0, 421.0, 474.0, 587.0, 845.0]) * 1.9


def hat(vel=1.0, decay=0.018, length=None):
    n = int((length or decay * 8) * SR)
    t = tt(n)
    sq = sum(np.sign(np.sin(2 * np.pi * f * t + RNG.uniform(0, 6.28))) for f in HAT_F)
    y = filt(0.5 * sq / 6 + 0.7 * noise(n) * 0.5, "highpass", 7000, 4) * np.exp(-t / decay)
    return fade_ends(y / np.max(np.abs(y)) * vel)


def crash(vel=1.0, length=2.4):
    n = int(length * SR)
    t = tt(n)
    out = []
    for _ in range(2):
        sq = sum(np.sign(np.sin(2 * np.pi * f * 1.37 * t + RNG.uniform(0, 6.28))) for f in HAT_F)
        y = filt(0.3 * sq / 6 + noise(n) * 0.6, "highpass", 3800, 2)
        y = filt(y, "lowpass", 14000)
        out.append(y * (np.exp(-t / 0.55) * 0.8 + 0.2 * np.exp(-t / 0.06)))
    y = np.stack(out)
    return fade_ends(y / np.max(np.abs(y)) * vel, 0.0005, 0.3)


def rev_cymbal(length, vel=1.0):
    """Reverse crash swell ending exactly at its last sample."""
    c = crash(1.0, length + 0.2)[:, :int(length * SR)]
    y = c[:, ::-1] * np.linspace(0, 1, c.shape[1]) ** 1.5
    return y / np.max(np.abs(y)) * vel


def riser(length, vel=1.0, f0=300, f1=11000):
    n = int(length * SR)
    t = tt(n) / length
    nz = np.stack([noise(n), noise(n)])
    fc = f0 * (f1 / f0) ** (t ** 1.6)
    y = tv_lowpass(nz, fc, q=2.5)
    y = filt(y, "highpass", 200)
    # pitched layer: detuned saws gliding up an octave and a fifth
    pitch = hz(53) * 2 ** (t ** 2 * 1.58)
    tone = sum(saw(pitch * 2 ** (d / 1200), n, RNG.uniform()) for d in (-15, 0, 15)) / 3
    tone = filt(tone, "lowpass", 4000) * 0.35
    y = y / np.max(np.abs(y)) + tone[None, :]
    return y * (t ** 2.2)[None, :] * vel


def downlifter(length, vel=1.0):
    n = int(length * SR)
    t = tt(n) / length
    nz = np.stack([noise(n), noise(n)])
    y = tv_lowpass(nz, 9000 * (250 / 9000) ** t, q=2.0)
    y = y / np.max(np.abs(y)) * (1 - t) ** 2
    return fade_ends(y * vel, 0.001, 0.05)


def impact(vel=1.0, length=1.6, f0=70):
    n = int(length * SR)
    t = tt(n)
    boom = np.sin(2 * np.pi * np.cumsum(f0 * 0.45 + f0 * 0.55 * np.exp(-t / 0.25)) / SR) * np.exp(-t / 0.45)
    thud = filt(noise(n), "lowpass", 900) * np.exp(-t / 0.05)
    y = np.tanh(1.5 * (boom + 0.5 * thud / np.max(np.abs(thud))))
    return fade_ends(y * vel, 0.0005, 0.2)


DETUNE = np.array([-24, -14, -6, 0, 6, 14, 24])
SPREAD = np.array([-0.9, 0.6, -0.4, 0.0, 0.4, -0.6, 0.9])


def supersaw(m, n, width=1.0):
    """7-voice detuned saw for MIDI note m, stereo-spread."""
    out = np.zeros((2, n))
    for d, p in zip(DETUNE, SPREAD):
        v = saw(hz(m) * 2 ** (d / 1200), n, RNG.uniform())
        a = (p * width + 1) * np.pi / 4
        out[0] += v * np.cos(a)
        out[1] += v * np.sin(a)
    return out / 4.0


def chord_pad(notes, n, cutoff, octave=0, width=1.0, env=None):
    y = sum(supersaw(m + 12 * octave, n, width) for m in notes)
    e = env if env is not None else env_adsr(n, 0.01, 0.4, 0.85, 0.08)
    y = y * e[None, :]
    fc = np.asarray(cutoff, float)
    fc = np.full(n, float(fc)) if fc.ndim == 0 else np.interp(np.linspace(0, 1, n), np.linspace(0, 1, len(fc)), fc)
    return tv_lowpass(y, fc, q=0.9) / len(notes)


def pluck(m, length=0.35, bright=4500, vel=1.0):
    n = int(length * SR)
    t = tt(n)
    x = 0.6 * saw(hz(m), n, 0.0) + 0.4 * saw(hz(m) * 2 ** (7 / 1200), n, 0.3)
    x += 0.35 * np.sign(np.sin(2 * np.pi * hz(m - 12) * t))
    fc = 350 + bright * np.exp(-t / 0.07)
    y = tv_lowpass(x, fc, q=1.4, block=32)[0]
    y *= np.exp(-t / 0.22) * np.minimum(1, t / 0.002)
    return fade_ends(y * vel, 0.0005, 0.02)


def bleep(m, length=0.18, vel=1.0):
    n = int(length * SR)
    t = tt(n)
    y = np.sin(2 * np.pi * hz(m) * t + 0.8 * np.sin(2 * np.pi * hz(m) * 2 * t) * np.exp(-t / 0.03))
    return fade_ends(y * np.exp(-t / 0.06) * vel, 0.001, 0.02)


def bass_note(m, length, sub_gain=1.0, mid_gain=1.0):
    n = int(length * SR)
    t = tt(n)
    e = env_adsr(n, 0.003, 0.12, 0.75, 0.03)
    sub = np.sin(2 * np.pi * hz(m) * t) * e
    mid = saw(hz(m + 12), n, 0.0) + 0.5 * saw(hz(m + 12) * 1.004, n, 0.5)
    mid = tv_lowpass(mid, 300 + 1400 * np.exp(-t / 0.06), q=1.2, block=32)[0] * e
    return sub * sub_gain, np.tanh(1.5 * mid) * mid_gain


# ------------------------------------------------------------------- harmony
CHORDS = {
    "Fm": [53, 56, 60, 65],
    "Db": [53, 56, 61, 65],
    "Ab": [51, 56, 60, 63],
    "Eb": [51, 55, 58, 63],
}
ROOT = {"Fm": 29, "Db": 37, "Ab": 32, "Eb": 39}
PROG = ["Fm", "Db", "Ab", "Eb"]
# hook motif: (16th step, MIDI note, length in 16ths) per chord
HOOK = {
    "Fm": [(0, 72, 2), (3, 68, 1), (6, 72, 2), (8, 75, 2), (10, 77, 2), (12, 75, 1), (14, 72, 2)],
    "Db": [(0, 77, 2), (3, 73, 1), (6, 72, 2), (8, 68, 2), (10, 72, 3), (14, 68, 2)],
    "Ab": [(0, 72, 2), (3, 68, 1), (6, 72, 2), (8, 75, 2), (10, 79, 2), (12, 75, 1), (14, 72, 2)],
    "Eb": [(0, 70, 2), (3, 67, 1), (6, 70, 2), (8, 75, 3), (12, 72, 2), (14, 70, 2)],
}
SWING = int(0.12 * STEP)


# ------------------------------------------------------------- arrangement
def compose():
    drums, bass_sub, bass_mid = Bus(), Bus(), Bus()
    chords, pad, lead, arp, fx = Bus(), Bus(), Bus(), Bus(), Bus()
    rev_send, dly_send = Bus(), Bus()
    kicks = []

    K = [kick(1.0), kick(0.95), kick(1.0, 0.8)]
    CL = clap(1.0)

    def do_kick(s, v=1.0, long=False):
        drums.add(K[2] if long else K[0], s, 0.5 * v)
        kicks.append(s)

    def do_clap(s, v=1.0):
        drums.add(CL, s, 0.55 * v, -0.05)
        rev_send.add(CL, s, 0.22 * v)

    def hats(s0, steps, density=1.0, grid=STEP):
        for k in range(steps):
            s = s0 + k * grid + (SWING if k % 2 else 0)
            if k % 4 == 2:
                drums.add(hat(1.0, 0.11), s, 0.22 * density, 0.25)
                rev_send.add(hat(1.0, 0.11), s, 0.03)
            elif k % 4 != 0 or density > 1.0:
                v = (0.55 if k % 2 else 0.8) * RNG.uniform(0.85, 1.0)
                drums.add(hat(v, 0.016), s, 0.2 * density, -0.3 if k % 2 else 0.2)

    def roll(s0, s1, steps_per_beat, v0=0.25, v1=0.8, tone=190, beat=BEAT):
        g = beat // steps_per_beat
        k, s = 0, s0
        while s < s1:
            frac = (s - s0) / max(1, s1 - s0)
            sn = snare(1.0, tone * (1 + 0.6 * frac), 0.16)
            drums.add(sn, s, (v0 + (v1 - v0) * frac) * 0.6, 0.1 * (-1) ** k)
            rev_send.add(sn, s, 0.15)
            s += g
            k += 1

    def bass_bar(s0, name, beats=4, pattern="off"):
        r = ROOT[name]
        for b in range(beats):
            for st, ln in (((2, 1.6),) if pattern == "off" else ((2, 1.0), (3, 0.9))):
                sub, mid = bass_note(r, ln * STEP / SR * 1.0 + 0.02)
                bass_sub.add(sub, s0 + b * BEAT + st * STEP, 0.42)
                bass_mid.add(mid, s0 + b * BEAT + st * STEP, 0.32)

    def chord_block(s0, s1, name, cutoff=5200, gain=1.0, octave=0):
        n = s1 - s0 + int(0.08 * SR)
        y = chord_pad(CHORDS[name], n, cutoff, octave)
        chords.add(y, s0, 1.1 * gain)
        rev_send.add(y, s0, 0.12 * gain)
        low = chord_pad([CHORDS[name][0] - 12, CHORDS[name][2] - 12], n, 900, 0, 0.5)
        pad.add(low, s0, 0.5 * gain)

    def hook(s0, name, octave=0, gain=1.0, limit=16):
        for st, m, ln in HOOK[name]:
            if st >= limit:
                continue
            p = pluck(m + 12 * octave, 0.12 + ln * STEP / SR, 5200, 1.0)
            lead.add(p, s0 + st * STEP, 0.55 * gain, 0.0)
            dly_send.add(p, s0 + st * STEP, 0.16 * gain)
            rev_send.add(p, s0 + st * STEP, 0.1 * gain)

    def arpeggio(s0, name, steps=16, gain=1.0, start=0):
        notes = [m + 12 for m in CHORDS[name]]
        seq = [notes[i] for i in (0, 1, 2, 3, 2, 1, 2, 3)]
        for k in range(start, steps):
            p = pluck(seq[k % len(seq)] + 12, 0.16, 2800, 1.0)
            arp.add(p, s0 + k * STEP, 0.15 * gain, 0.45 * (-1) ** k)
            dly_send.add(p, s0 + k * STEP, 0.05 * gain)

    def crash_at(s, v=1.0):
        c = crash(1.0)
        fx.add(c, s, 0.26 * v)
        rev_send.add(c, s, 0.08 * v)

    # ---- 0-54: build (Eb = VII, resolving to Fm at the drop)
    s_drop = OB(0)
    n_in = s_drop
    y = chord_pad(CHORDS["Eb"], n_in, 250 * (5000 / 250) ** (np.arange(n_in) / n_in) ** 1.5)
    chords.add(y * np.linspace(0.3, 1, n_in) ** 2, 0, 1.4)
    rev_send.add(y, 0, 0.2)
    fx.add(riser(n_in / SR, 1.0), 0, 0.45)
    fx.add(rev_cymbal(1.2), s_drop - int(1.2 * SR), 0.3)
    roll(OB(-3), OB(-1), 2, 0.2, 0.5)
    roll(OB(-1), OB(-0.25), 4, 0.5, 0.9)  # last 16th left silent before the drop
    sub, _ = bass_note(ROOT["Eb"], 0.25)
    bass_sub.add(sub * np.linspace(0, 1, len(sub)), OB(-0.5), 0.3)

    # ---- main grid bars 0..10
    for bar in range(11):
        s0 = OB(4 * bar)
        name = PROG[bar % 4]
        if bar in (0, 4, 8):
            crash_at(s0, 1.0 if bar != 8 else 0.6)
        if bar in (0, 4):
            fx.add(impact(0.9), s0, 0.32)
            rev_send.add(impact(0.9), s0, 0.12)

        if bar == 3:  # counter: riser bar into the editor drop at 294
            for b in range(2):
                do_kick(OB(4 * bar + b))
            do_clap(OB(4 * bar + 1))
            hats(s0, 8)
            bass_bar(s0, name, 2)
            chord_block(s0, OB(4 * bar + 3.75), name, 1200 + 4800 * np.linspace(0, 1, OB(4) - OB(0) + 4000) ** 2)
            hook(s0, name, limit=8)
            roll(OB(4 * bar + 2), OB(4 * bar + 3), 4, 0.3, 0.6)
            roll(OB(4 * bar + 3), OB(4 * bar + 3.75), 8, 0.6, 1.0)
            fx.add(riser(2.0, 1.0), OB(4 * bar + 4) - int(2.0 * SR), 0.45)
            fx.add(rev_cymbal(1.0), OB(4 * bar + 4) - SR, 0.22)
            continue

        if bar == 10:  # platforms: groove thins, filter closes into the break
            for b in range(3):
                do_kick(OB(40 + b))
            do_clap(OB(41))
            hats(s0, 12)
            bass_bar(s0, name, 3)
            n = F(705) - s0
            chord_block(s0, F(705), name, 5200 * (500 / 5200) ** (np.linspace(0, 1, n + 4000) ** 0.8))
            arpeggio(s0, name, 12, 0.8)
            fx.add(rev_cymbal(0.9), F(705) - int(0.9 * SR), 0.24)
            continue

        for b in range(4):
            do_kick(OB(4 * bar + b))
        do_clap(OB(4 * bar + 1))
        do_clap(OB(4 * bar + 3))
        hats(s0, 16, 1.0 if bar < 4 else 1.1)
        bass_bar(s0, name, 4, "off" if bar < 4 or bar % 2 == 0 else "roll")
        chord_block(s0, OB(4 * bar + 4), name)
        if bar >= 6:
            arpeggio(s0, name, 16, 1.0)
        if bar in (5, 9):
            hook(s0, name, limit=12)
        else:
            hook(s0, name, octave=1 if bar in (8, 9) else 0, gain=0.8 if bar in (8, 9) else 1.0)
        if bar in (2, 7, 9):  # fills at phrase ends
            roll(OB(4 * bar + 3), OB(4 * bar + 4), 4, 0.25, 0.7, tone=230)

    # short swells landing exactly on the off-grid scene changes
    for f in (444, 489, 543, 597, 657):
        fx.add(rev_cymbal(0.6), F(f) - int(0.6 * SR), 0.12)

    # ---- 705-810: break (AI/MCP), new grid
    fx.add(impact(0.6, 1.4, 55), F(705), 0.25)
    fx.add(downlifter(1.4), F(705), 0.16)
    rev_send.add(impact(0.6, 1.4, 55), F(705), 0.2)
    n = NB(3) - F(705)
    y = chord_pad(CHORDS["Db"], n + 4000, 700)
    pad.add(y, F(705), 1.25)
    rev_send.add(y, F(705), 0.25)
    for k in range(6):  # "AI" bleeps on 8ths, through the delay
        m = [77, 73, 80, 72, 77, 85][k]
        b = bleep(m, 0.18)
        arp.add(b, NB(k * 0.5), 0.16, 0.5 * (-1) ** k)
        dly_send.add(b, NB(k * 0.5), 0.12)
    n = NB(7) - NB(3)
    y = chord_pad(CHORDS["Eb"], n, 700 * (7000 / 700) ** np.linspace(0, 1, n) ** 2)
    pad.add(y, NB(3), 1.1)
    rev_send.add(y, NB(3), 0.2)
    for k in range(8):
        b = bleep([75, 70, 79, 67, 75, 82, 79, 87][k], 0.15)
        arp.add(b, NB(3 + k * 0.5), 0.08 + 0.01 * k, 0.5 * (-1) ** k)
        dly_send.add(b, NB(3 + k * 0.5), 0.08)
    for b in range(4):
        do_kick(NB(3 + b), 0.6 + 0.1 * b)
    roll(NB(3), NB(5), 2, 0.2, 0.45)
    roll(NB(5), NB(6), 4, 0.45, 0.7)
    roll(NB(6), NB(6.75), 8, 0.7, 1.0)
    fx.add(riser(2.0, 1.0), NB(7) - 2 * SR, 0.45)
    fx.add(rev_cymbal(1.3), NB(7) - int(1.3 * SR), 0.24)

    # ---- 810: final hit + last bar (Fm), final stab at 870, tail ends by 900
    s = NB(7)
    crash_at(s, 1.1)
    fx.add(impact(1.0, 1.8, 65), s, 0.4)
    rev_send.add(impact(1.0), s, 0.15)
    for b in range(4):
        do_kick(NB(7 + b))
    do_clap(NB(8))
    do_clap(NB(10))
    hats(s, 16)
    bass_bar(s, "Fm", 4)
    chord_block(s, NB(11), "Fm")
    hook(s, "Fm", octave=0)
    e = NB(11)
    do_kick(e, 1.0, long=True)
    crash_at(e, 0.7)
    fx.add(impact(0.8, 1.0, 58), e, 0.3)
    n = int(0.5 * SR)
    stab = chord_pad(CHORDS["Fm"], n, 4500, env=env_adsr(n, 0.003, 0.15, 0.0, 0.05))
    chords.add(stab, e, 1.2)
    rev_send.add(stab, e, 0.35)
    for m in (77, 84):
        p = pluck(m, 0.6, 5000)
        lead.add(p, e, 0.45)
        dly_send.add(p, e, 0.15)
        rev_send.add(p, e, 0.15)
    sub, _ = bass_note(ROOT["Fm"], 0.6)
    bass_sub.add(sub, e, 0.45)

    return dict(drums=drums, bass_sub=bass_sub, bass_mid=bass_mid, chords=chords, pad=pad,
                lead=lead, arp=arp, fx=fx, rev=rev_send, dly=dly_send), kicks


# --------------------------------------------------------------- processing
def sidechain(kicks, depth, release=0.19):
    g = np.ones(TOTAL)
    rn = int(release * SR)
    att = int(0.003 * SR)
    shape = 1 - depth * (1 - np.linspace(0, 1, rn)) ** 2
    shape[:att] = np.linspace(1, shape[att], att)
    for s in kicks:
        s0, s1 = max(s, 0), min(s + rn, TOTAL)
        if s1 > s0:
            g[s0:s1] = np.minimum(g[s0:s1], shape[s0 - s:s1 - s])
    return g


def reverb_ir(length=2.0, predelay=0.018):
    n = int(length * SR)
    t = tt(n)
    ir = np.zeros((2, n + int(predelay * SR)))
    for ch in range(2):
        hi = filt(noise(n), "highpass", 3000) * np.exp(-t / 0.18)
        lo = filt(noise(n), "lowpass", 3000) * np.exp(-t / 0.42)
        ir[ch, int(predelay * SR):] = hi * 0.5 + lo
    ir = filt(ir, "highpass", 250)
    return ir / np.sqrt(np.sum(ir ** 2) / 2) * 0.5


def pingpong(x, delay=STEP * 3, fb=0.42, taps=7):
    x = filt(filt(x, "highpass", 400), "lowpass", 6000)
    mono = x.mean(axis=0)
    y = np.zeros((2, TOTAL))
    for k in range(1, taps + 1):
        d = k * delay
        if d >= TOTAL:
            break
        y[(k + 1) % 2, d:] += mono[:TOTAL - d] * fb ** (k - 1)
    return y


K1 = (np.array([1.53512485958697, -2.69169618940638, 1.19839281085285]),
      np.array([1.0, -1.69065929318241, 0.73248077421585]))
K2 = (np.array([1.0, -2.0, 1.0]), np.array([1.0, -1.99004745483398, 0.99007225036621]))


def lufs(x):
    """ITU-R BS.1770-4 integrated loudness (48 kHz)."""
    y = signal.lfilter(*K2, signal.lfilter(*K1, x, axis=-1), axis=-1)
    blk, hop = int(0.4 * SR), int(0.1 * SR)
    ms = np.array([np.sum(np.mean(y[:, i:i + blk] ** 2, axis=1)) for i in range(0, y.shape[1] - blk + 1, hop)])
    ld = -0.691 + 10 * np.log10(ms + 1e-12)
    ms = ms[ld > -70]
    rel = -0.691 + 10 * np.log10(ms.mean()) - 10
    ms = ms[-0.691 + 10 * np.log10(ms) > rel]
    return -0.691 + 10 * np.log10(ms.mean())


def true_peak(x):
    return np.max(np.abs(signal.resample_poly(x, 4, 1, axis=-1)))


def glue(x, thr_db=-16, ratio=2.0):
    lvl = np.sqrt(uniform_filter1d(np.mean(x ** 2, axis=0), int(0.01 * SR)) + 1e-12)
    over = np.maximum(20 * np.log10(lvl) - thr_db, 0)
    gdb = -over * (1 - 1 / ratio)
    a = np.exp(-1 / (0.06 * SR))
    gdb = signal.lfilter([1 - a], [1, -a], gdb)
    return x * 10 ** (gdb / 20)


def limiter(x, ceil):
    up = np.abs(signal.resample_poly(x, 4, 1, axis=-1)).max(axis=0)
    peak = up[: x.shape[1] * 4].reshape(-1, 4).max(axis=1)
    g = np.minimum(1, ceil / np.maximum(peak, 1e-9))
    w = int(0.004 * SR)
    g = uniform_filter1d(minimum_filter1d(g, 2 * w + 1), w + 1)
    a = np.exp(-1 / (0.08 * SR))
    slow = signal.lfilter([1 - a], [1, -a], g, zi=[g[0] * a])[0]
    return x * np.minimum(g, slow)[None, :]


def master(stems, kicks):
    sc_hard = sidechain(kicks, 0.75)
    sc_mid = sidechain(kicks, 0.55, 0.14)
    sc_soft = sidechain(kicks, 0.35)
    mix = np.zeros((2, TOTAL))
    mix += stems["drums"].x
    mix += filt(stems["bass_sub"].x, "lowpass", 160) * sc_mid
    mix += filt(stems["bass_mid"].x, "highpass", 90) * sc_mid
    mix += filt(stems["chords"].x, "highpass", 180) * sc_hard
    mix += filt(stems["pad"].x, "highpass", 120) * sc_hard
    mix += stems["lead"].x * sc_soft
    mix += filt(stems["arp"].x, "highpass", 300) * sc_soft
    mix += stems["fx"].x
    wet = signal.fftconvolve(stems["rev"].x, reverb_ir(), axes=-1)[:, :TOTAL]
    mix += wet * sc_soft * 0.9
    mix += pingpong(stems["dly"].x) * sc_soft * 0.7
    # stereo width: keep lows mono, widen highs a touch
    lo = filt(mix, "lowpass", 140, 4)
    hi = mix - lo
    mid, side = hi.mean(axis=0), (hi[0] - hi[1]) / 2
    hi = np.stack([mid + 1.2 * side, mid - 1.2 * side])
    mix = lo.mean(axis=0)[None, :] + hi
    mix = filt(mix, "highpass", 28)
    mix -= mix.mean(axis=1, keepdims=True)
    # clean tail: everything gone by frame 900
    fade = np.ones(TOTAL)
    f0, f1 = F(885), F(899)
    fade[f0:f1] = np.linspace(1, 0, f1 - f0) ** 2
    fade[f1:] = 0
    mix *= fade

    ceil = 10 ** (CEIL_DBTP / 20) * 0.985
    gain_db = 0.0
    pre = glue(mix / np.max(np.abs(mix)) * 10 ** (-12 / 20))
    for _ in range(6):
        out = limiter(pre * 10 ** (gain_db / 20), ceil)
        L = lufs(out)
        if abs(L - TARGET_LUFS) < 0.05:
            break
        gain_db += TARGET_LUFS - L
    out *= fade
    return out, L


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    default = len(sys.argv) < 2
    path = sys.argv[1] if not default else os.path.join(root, "out/jibaru-promo-music.wav")
    stems, kicks = compose()
    out, L = master(stems, kicks)
    tp = 20 * np.log10(true_peak(out))
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    dither = (RNG.uniform(-0.5, 0.5, out.shape) + RNG.uniform(-0.5, 0.5, out.shape)) / 32767
    pcm = np.clip(np.round((out + dither) * 32767), -32768, 32767).astype(np.int16)
    wavfile.write(path, SR, pcm.T)
    print(f"{path}: {TOTAL / SR:.2f} s, {L:.2f} LUFS, true peak {tp:.2f} dBTP")
    if default and shutil.which("ffmpeg"):
        for mp3 in ("out/jibaru-promo-music.mp3", "public/music/jibaru-promo-music.mp3"):
            mp3 = os.path.join(root, mp3)
            os.makedirs(os.path.dirname(mp3), exist_ok=True)
            subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", path, "-c:a", "libmp3lame",
                            "-b:a", "320k", "-map_metadata", "-1", "-metadata", "title=Jibaru Promo (CC0)",
                            "-bitexact", mp3], check=True)
            print(mp3)


if __name__ == "__main__":
    main()
