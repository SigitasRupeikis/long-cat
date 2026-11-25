//! Fully procedural audio: every sound effect and the music loop are synthesised at startup.
use std::f32::consts::{PI, TAU};

pub const SR: u32 = 44100;

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum Wave {
    Sine,
    Square,
    Saw,
    Tri,
    Noise,
}

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

pub fn mtof(m: f32) -> f32 {
    440.0 * 2f32.powf((m - 69.0) / 12.0)
}

pub fn tone(buf: &mut Vec<f32>, start: f32, dur: f32, f0: f32, f1: f32, vol: f32, wave: Wave, decay: f32) {
    let s0 = (start * SR as f32) as usize;
    let n = (dur * SR as f32) as usize;
    if buf.len() < s0 + n {
        buf.resize(s0 + n, 0.0);
    }
    let mut phase = 0.0f32;
    let mut rng = Rng((0x1234_5678u32 ^ (s0 as u32).wrapping_mul(2654435761)) | 1);
    for i in 0..n {
        let x = i as f32 / n as f32;
        let f = f0 * (f1 / f0).powf(x);
        phase = (phase + f / SR as f32).fract();
        let v = match wave {
            Wave::Sine => (phase * TAU).sin(),
            Wave::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Wave::Saw => phase * 2.0 - 1.0,
            Wave::Tri => (phase * 2.0 - 1.0).abs() * 2.0 - 1.0,
            Wave::Noise => rng.next(),
        };
        let att = (i as f32 / (0.003 * SR as f32)).min(1.0);
        let env = att * (1.0 - x).powf(decay);
        buf[s0 + i] += v * env * vol;
    }
}

pub fn to_wav(buf: &[f32], gain: f32) -> Vec<u8> {
    let mut data = Vec::with_capacity(buf.len() * 2);
    for &s in buf {
        let v = ((s * gain).tanh() * 32000.0) as i16;
        data.extend_from_slice(&v.to_le_bytes());
    }
    let mut out = Vec::with_capacity(44 + data.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SR.to_le_bytes());
    out.extend_from_slice(&(SR * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}



fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn ensure(buf: &mut Vec<f32>, start: f32, dur: f32) -> (usize, usize) {
    let s0 = (start * SR as f32) as usize;
    let n = (dur * SR as f32) as usize;
    if buf.len() < s0 + n {
        buf.resize(s0 + n, 0.0);
    }
    (s0, n)
}

/// FM bell / celesta
pub fn bell(buf: &mut Vec<f32>, start: f32, dur: f32, f: f32, vol: f32) {
    let (s0, n) = ensure(buf, start, dur);
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let x = i as f32 / n as f32;
        let idx = 3.2 * (-t * 5.0).exp();
        let m = (TAU * f * 3.5 * t).sin() * idx;
        let v = (TAU * f * t + m).sin() + 0.3 * (TAU * f * 2.0 * t).sin() * (-t * 6.0).exp();
        let att = (i as f32 / (0.002 * SR as f32)).min(1.0);
        let env = att * (-x * 4.0).exp() * (1.0 - x);
        buf[s0 + i] += v * env * vol;
    }
}

/// harp pluck
pub fn pluck(buf: &mut Vec<f32>, start: f32, dur: f32, f: f32, vol: f32) {
    let (s0, n) = ensure(buf, start, dur);
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let x = i as f32 / n as f32;
        let v = (TAU * f * t).sin() * (-t * 4.0).exp()
            + 0.5 * (TAU * f * 2.0 * t).sin() * (-t * 7.0).exp()
            + 0.25 * (TAU * f * 3.0 * t).sin() * (-t * 10.0).exp();
        let att = (i as f32 / (0.001 * SR as f32)).min(1.0);
        buf[s0 + i] += v * att * (1.0 - x) * vol;
    }
}

/// shimmering pad: detuned sines + tremolo, slow swell
pub fn shimmer(buf: &mut Vec<f32>, start: f32, dur: f32, f: f32, vol: f32) {
    let (s0, n) = ensure(buf, start, dur);
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let x = i as f32 / n as f32;
        let mut v = 0.0;
        for k in 0..5 {
            let d = 1.0 + (k as f32 - 2.0) * 0.0032;
            v += (TAU * f * d * t + k as f32).sin();
        }
        let trem = 0.75 + 0.25 * (TAU * 5.5 * t).sin();
        let env = (PI * x).sin().powf(0.7);
        buf[s0 + i] += v * 0.2 * trem * env * vol;
    }
}

/// airy "ooh" choir: harmonics with vibrato and slow envelope
pub fn choir(buf: &mut Vec<f32>, start: f32, dur: f32, f: f32, vol: f32) {
    let (s0, n) = ensure(buf, start, dur);
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let x = i as f32 / n as f32;
        let vib = 1.0 + 0.006 * (TAU * 5.0 * t).sin() * (x * 4.0).min(1.0);
        let mut v = 0.0;
        for (h, a) in [(1.0, 1.0), (2.0, 0.55), (3.0, 0.3), (4.0, 0.12), (5.0, 0.08)] {
            v += a * (TAU * f * h * vib * t).sin() + a * 0.6 * (TAU * f * h * vib * 1.004 * t).sin();
        }
        let env = (PI * x).sin().powf(0.6);
        buf[s0 + i] += v * 0.3 * env * vol;
    }
}

/// band-passed noise sweep (whoosh). `peak` = where in the sweep the volume peaks (0..1).
pub fn whoosh(buf: &mut Vec<f32>, start: f32, dur: f32, f0: f32, f1: f32, vol: f32, peak: f32) {
    let (s0, n) = ensure(buf, start, dur);
    let mut rng = Rng(0xFACE | 1);
    let (mut low, mut band) = (0.0f32, 0.0f32);
    for i in 0..n {
        let x = i as f32 / n as f32;
        let fc = f0 * (f1 / f0).powf(x);
        let f = 2.0 * (PI * fc / SR as f32).sin();
        let inp = rng.next();
        low += f * band;
        let high = inp - low - 0.18 * band;
        band += f * high;
        let env = if x < peak { x / peak } else { (1.0 - x) / (1.0 - peak) };
        buf[s0 + i] += band * env.max(0.0).powf(1.5) * vol * 0.5;
    }
}

/// Schroeder reverb: adds `tail` seconds and mixes `wet`
pub fn reverb(buf: &mut Vec<f32>, tail: f32, wet: f32) {
    let n = buf.len() + (tail * SR as f32) as usize;
    buf.resize(n, 0.0);
    let delays = [1557usize, 1617, 1491, 1422, 1277, 1356];
    let fb = 0.84;
    let damp = 0.3;
    let mut combs: Vec<Vec<f32>> = delays.iter().map(|&d| vec![0.0; d]).collect();
    let mut idx = vec![0usize; delays.len()];
    let mut lp = vec![0.0f32; delays.len()];
    let mut ap = [vec![0.0f32; 225], vec![0.0f32; 556]];
    let mut api = [0usize; 2];
    let mut out = vec![0.0f32; n];
    for i in 0..n {
        let x = buf[i];
        let mut s = 0.0;
        for c in 0..delays.len() {
            let y = combs[c][idx[c]];
            lp[c] = y * (1.0 - damp) + lp[c] * damp;
            combs[c][idx[c]] = x * 0.2 + lp[c] * fb;
            idx[c] = (idx[c] + 1) % delays[c];
            s += y;
        }
        let mut y = s * 0.2;
        for a in 0..2 {
            let d = ap[a][api[a]];
            let v = y + 0.5 * d;
            ap[a][api[a]] = v;
            api[a] = (api[a] + 1) % ap[a].len();
            y = -0.5 * v + d;
        }
        out[i] = x + y * wet;
    }
    *buf = out;
}

/// A synthesised "meow": pitch contour + vibrato + a sweeping low-pass that acts like a vowel (m-ee-ow).
pub fn meow(dur: f32, f_start: f32, f_peak: f32, f_end: f32, vol: f32) -> Vec<f32> {
    let n = (dur * SR as f32) as usize;
    let mut out = vec![0.0; n];
    let mut ph = 0.0f32;
    let mut lp = 0.0f32;
    let mut rng = Rng(0xBEEF);
    for i in 0..n {
        let x = i as f32 / n as f32;
        let f = if x < 0.3 {
            lerp(f_start, f_peak, x / 0.3)
        } else {
            lerp(f_peak, f_end, (x - 0.3) / 0.7)
        };
        let f = f * (1.0 + 0.015 * (x * dur * 40.0).sin());
        ph = (ph + f / SR as f32).fract();
        let src = (ph * 2.0 - 1.0) * 0.7 + (ph * TAU).sin() * 0.3 + rng.next() * 0.03;
        let a = 0.08 + 0.4 * (PI * x).sin().powf(1.2);
        lp += a * (src - lp);
        let env = (x * 25.0).min(1.0) * (1.0 - x).powf(0.5);
        out[i] = lp * env * vol * 1.6;
    }
    out
}

fn mix(dst: &mut Vec<f32>, src: &[f32], start: f32) {
    let s0 = (start * SR as f32) as usize;
    if dst.len() < s0 + src.len() {
        dst.resize(s0 + src.len(), 0.0);
    }
    for (i, v) in src.iter().enumerate() {
        dst[s0 + i] += v;
    }
}

pub fn purr(dur: f32, vol: f32) -> Vec<f32> {
    let n = (dur * SR as f32) as usize;
    let mut out = vec![0.0; n];
    let mut rng = Rng(0xCA7);
    let mut lp = 0.0f32;
    let mut ph = 0.0f32;
    let mut sub = 0.0f32;
    for i in 0..n {
        let x = i as f32 / n as f32;
        ph = (ph + 25.0 / SR as f32).fract();
        sub = (sub + 52.0 / SR as f32).fract();
        let am = ((ph * TAU).sin() * 0.5 + 0.5).powi(3);
        lp += 0.06 * (rng.next() - lp);
        let env = (x * 8.0).min(1.0) * (1.0 - x).min(0.3) / 0.3;
        out[i] = (lp * 1.4 + (sub * TAU).sin() * 0.25) * am * env * vol;
    }
    out
}

/// A5-minor pentatonic note by index (so every combo level sounds consonant and higher)
fn penta(idx: usize) -> f32 {
    let sc = [0.0, 3.0, 5.0, 7.0, 10.0];
    69.0 + 12.0 * (idx / 5) as f32 + sc[idx % 5]
}

pub fn sfx_eat(combo: usize) -> Vec<f32> {
    let mut b = vec![];
    tone(&mut b, 0.0, 0.05, 2200.0, 3200.0, 0.1, Wave::Sine, 1.0); // tiny squeak
    for k in 0..4 {
        let m = penta(combo + k * 2);
        pluck(&mut b, 0.03 + k as f32 * 0.065, 0.5, mtof(m), 0.32);
    }
    let top = penta(combo + 8);
    bell(&mut b, 0.26, 0.9, mtof(top), 0.16);
    shimmer(&mut b, 0.0, 0.7, mtof(penta(combo + 5) + 12.0), 0.06);
    reverb(&mut b, 0.7, 0.4);
    b
}

pub fn sfx_fish() -> Vec<f32> {
    let mut b = meow(0.45, 520.0, 900.0, 600.0, 0.4);
    for k in 0..9 {
        pluck(&mut b, 0.1 + k as f32 * 0.05, 0.6, mtof(penta(4 + k)), 0.28);
    }
    for (i, m) in [81.0, 84.0, 88.0, 93.0].iter().enumerate() {
        bell(&mut b, 0.4 + i as f32 * 0.07, 1.2, mtof(*m), 0.14);
    }
    whoosh(&mut b, 0.0, 0.7, 500.0, 6000.0, 0.25, 0.7);
    shimmer(&mut b, 0.2, 1.2, mtof(93.0), 0.08);
    reverb(&mut b, 1.0, 0.45);
    b
}

pub fn sfx_yarn() -> Vec<f32> {
    let mut b = vec![];
    tone(&mut b, 0.0, 0.1, 180.0, 560.0, 0.3, Wave::Sine, 1.0);
    tone(&mut b, 0.1, 0.3, 560.0, 150.0, 0.3, Wave::Sine, 1.2);
    whoosh(&mut b, 0.0, 0.8, 5000.0, 400.0, 0.3, 0.2);
    for k in 0..6 {
        bell(&mut b, 0.15 + k as f32 * 0.09, 1.0, mtof(penta(14 - k * 2)), 0.14);
    }
    shimmer(&mut b, 0.0, 1.5, mtof(64.0), 0.08);
    reverb(&mut b, 1.0, 0.5);
    b
}

pub fn sfx_turn() -> Vec<f32> {
    let mut b = vec![];
    whoosh(&mut b, 0.0, 0.14, 3500.0, 1200.0, 0.1, 0.3);
    tone(&mut b, 0.0, 0.05, 200.0, 90.0, 0.12, Wave::Sine, 1.5);
    reverb(&mut b, 0.2, 0.15);
    b
}

pub fn sfx_die() -> Vec<f32> {
    let mut b = vec![];
    whoosh(&mut b, 0.0, 0.5, 300.0, 8000.0, 0.35, 0.9);
    mix(&mut b, &meow(0.9, 700.0, 800.0, 220.0, 0.45), 0.35);
    bell(&mut b, 0.45, 2.5, 110.0, 0.35);
    bell(&mut b, 0.45, 2.5, 164.8, 0.2);
    choir(&mut b, 0.5, 1.8, mtof(45.0), 0.2);
    reverb(&mut b, 1.5, 0.5);
    b
}

pub fn sfx_pop(variant: usize) -> Vec<f32> {
    let mut b = vec![];
    let notes = [[84.0, 91.0], [88.0, 93.0], [86.0, 96.0]][variant % 3];
    for (i, m) in notes.iter().enumerate() {
        bell(&mut b, i as f32 * 0.04, 0.4, mtof(*m), 0.18);
    }
    tone(&mut b, 0.0, 0.05, 6000.0, 6000.0, 0.05, Wave::Noise, 3.0);
    reverb(&mut b, 0.4, 0.4);
    b
}

pub fn sfx_level() -> Vec<f32> {
    let mut b = vec![];
    for k in 0..14 {
        pluck(&mut b, k as f32 * 0.045, 0.9, mtof(penta(k + 2)), 0.28);
    }
    for (i, m) in [57.0, 60.0, 64.0, 69.0].iter().enumerate() {
        choir(&mut b, 0.1, 2.0, mtof(*m + 12.0 * (i / 2) as f32), 0.14);
    }
    for (i, m) in [93.0, 96.0, 100.0].iter().enumerate() {
        bell(&mut b, 0.5 + i as f32 * 0.1, 1.5, mtof(*m), 0.12);
    }
    whoosh(&mut b, 0.0, 0.8, 300.0, 9000.0, 0.3, 0.85);
    mix(&mut b, &purr(0.9, 0.35), 0.5);
    reverb(&mut b, 1.5, 0.5);
    b
}

pub fn sfx_start() -> Vec<f32> {
    let mut b = vec![];
    whoosh(&mut b, 0.0, 0.5, 400.0, 7000.0, 0.3, 0.85);
    for (i, m) in [69.0, 72.0, 76.0, 81.0].iter().enumerate() {
        bell(&mut b, 0.3 + i as f32 * 0.06, 1.4, mtof(*m), 0.16);
    }
    mix(&mut b, &meow(0.3, 600.0, 1000.0, 800.0, 0.3), 0.35);
    shimmer(&mut b, 0.2, 1.4, mtof(81.0), 0.07);
    reverb(&mut b, 1.0, 0.45);
    b
}

pub fn sfx_over() -> Vec<f32> {
    let mut b = vec![];
    for (i, m) in [57.0, 60.0, 64.0].iter().enumerate() {
        choir(&mut b, i as f32 * 0.02, 2.6, mtof(*m), 0.2);
    }
    mix(&mut b, &meow(1.2, 520.0, 640.0, 160.0, 0.35), 0.1);
    for (i, m) in [81.0, 79.0, 76.0, 72.0, 69.0].iter().enumerate() {
        bell(&mut b, 0.6 + i as f32 * 0.25, 1.6, mtof(*m), 0.14);
    }
    reverb(&mut b, 1.8, 0.55);
    b
}

/// Ethereal 8-bar loop: harp arpeggios, shimmering pads, choir swells, celesta melody, heartbeat and shaker.
pub fn music() -> Vec<f32> {
    let bpm = 92.0;
    let beat = 60.0 / bpm;
    let bar = beat * 4.0;
    let prog: [(f32, bool); 8] = [
        (45.0, true),
        (41.0, false),
        (48.0, false),
        (43.0, false),
        (45.0, true),
        (41.0, false),
        (43.0, false),
        (45.0, true),
    ];
    // (beat position, midi note) per bar
    let melody: [&[(f32, f32)]; 8] = [
        &[(0.0, 81.0), (1.5, 84.0), (2.0, 83.0), (3.0, 81.0)],
        &[(0.0, 77.0), (1.0, 81.0), (2.0, 84.0), (3.5, 81.0)],
        &[(0.0, 79.0), (1.5, 84.0), (2.0, 88.0), (3.0, 84.0)],
        &[(0.0, 83.0), (1.0, 81.0), (2.0, 79.0), (3.0, 76.0)],
        &[(0.0, 81.0), (0.5, 84.0), (1.0, 88.0), (2.0, 86.0), (3.0, 84.0)],
        &[(0.0, 84.0), (1.5, 81.0), (2.0, 77.0), (3.0, 81.0)],
        &[(0.0, 83.0), (1.0, 86.0), (2.0, 88.0), (3.0, 86.0)],
        &[(0.0, 81.0), (2.0, 76.0), (3.0, 72.0)],
    ];
    let pat = [0usize, 1, 2, 3, 2, 1, 2, 3];
    let mut b: Vec<f32> = Vec::new();
    for (bi, &(root, minor)) in prog.iter().enumerate() {
        let t0 = bi as f32 * bar;
        let third = if minor { 3.0 } else { 4.0 };
        let chord = [0.0, third, 7.0, 14.0];
        for &c in &chord[..3] {
            shimmer(&mut b, t0, bar * 1.05, mtof(root + 12.0 + c), 0.05);
        }
        if bi % 2 == 0 {
            choir(&mut b, t0, bar * 2.0, mtof(root + 24.0), 0.07);
            choir(&mut b, t0, bar * 2.0, mtof(root + 31.0), 0.05);
        }
        tone(&mut b, t0, beat * 3.8, mtof(root), mtof(root), 0.24, Wave::Sine, 0.7);
        for k in 0..8 {
            let tt = t0 + k as f32 * beat * 0.5;
            let m = root + 24.0 + chord[pat[k]] + if k >= 4 { 12.0 } else { 0.0 };
            pluck(&mut b, tt, beat * 1.4, mtof(m), 0.09);
            if bi >= 4 {
                pluck(&mut b, tt + beat * 0.375, beat * 1.0, mtof(m + 12.0), 0.03);
            }
        }
        for &(bp, m) in melody[bi] {
            bell(&mut b, t0 + bp * beat, beat * 2.2, mtof(m), 0.08);
            if bi >= 4 {
                shimmer(&mut b, t0 + bp * beat, beat * 1.5, mtof(m + 12.0), 0.02);
            }
        }
        tone(&mut b, t0, 0.2, 95.0, 38.0, 0.28, Wave::Sine, 2.0);
        tone(&mut b, t0 + beat * 2.0, 0.2, 85.0, 38.0, 0.2, Wave::Sine, 2.0);
        for k in 0..8 {
            tone(&mut b, t0 + k as f32 * beat * 0.5, 0.04, 8000.0, 8000.0, 0.012 + if k % 2 == 1 { 0.01 } else { 0.0 }, Wave::Noise, 3.0);
        }
        if bi == 3 || bi == 7 {
            whoosh(&mut b, t0 + beat * 2.5, beat * 1.5, 500.0, 9000.0, 0.12, 0.9);
        }
    }
    let total = (bar * 8.0 * SR as f32) as usize;
    b.resize(total, 0.0);
    // reverb with wrap-around tail: process the loop twice, keep the second pass
    let mut twice = b.clone();
    twice.extend_from_slice(&b);
    reverb(&mut twice, 0.0, 0.5);
    let mut out = twice[total..2 * total].to_vec();
    let fade = 128.min(out.len());
    let len = out.len();
    for i in 0..fade {
        out[len - 1 - i] *= i as f32 / fade as f32;
        out[i] *= i as f32 / fade as f32;
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn synth_ok() {
        let m = super::music();
        assert!(m.len() > 800_000);
        assert!(m.iter().all(|x| x.is_finite()));
        let mut all = vec![
            super::sfx_die(),
            super::sfx_fish(),
            super::sfx_level(),
            super::sfx_over(),
            super::sfx_yarn(),
            super::sfx_start(),
            super::sfx_turn(),
            super::sfx_pop(1),
        ];
        for c in 0..8 {
            all.push(super::sfx_eat(c));
        }
        for s in all {
            assert!(s.iter().all(|x| x.is_finite()));
            assert!(super::to_wav(&s, 1.0).len() > 44);
        }
    }
}
