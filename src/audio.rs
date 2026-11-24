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

pub fn sfx_eat() -> Vec<f32> {
    // mouse squeak + little crunch
    let mut b = vec![];
    tone(&mut b, 0.0, 0.06, 1900.0, 3000.0, 0.28, Wave::Sine, 1.0);
    tone(&mut b, 0.07, 0.09, 2700.0, 1800.0, 0.28, Wave::Sine, 1.3);
    tone(&mut b, 0.09, 0.05, 3500.0, 3500.0, 0.2, Wave::Noise, 3.0);
    tone(&mut b, 0.0, 0.1, 300.0, 140.0, 0.35, Wave::Sine, 1.8);
    b
}

pub fn sfx_fish() -> Vec<f32> {
    let mut b = meow(0.45, 520.0, 900.0, 600.0, 0.5);
    for (i, m) in [79.0, 83.0, 86.0, 91.0, 95.0].iter().enumerate() {
        let t = 0.18 + i as f32 * 0.06;
        tone(&mut b, t, 0.25, mtof(*m), mtof(*m), 0.2, Wave::Sine, 1.8);
    }
    b
}

pub fn sfx_yarn() -> Vec<f32> {
    let mut b = vec![];
    tone(&mut b, 0.0, 0.1, 180.0, 560.0, 0.4, Wave::Sine, 1.0);
    tone(&mut b, 0.1, 0.3, 560.0, 150.0, 0.4, Wave::Sine, 1.2);
    tone(&mut b, 0.1, 0.3, 1120.0, 300.0, 0.1, Wave::Tri, 1.5);
    b
}

pub fn sfx_turn() -> Vec<f32> {
    // soft paw pat
    let mut b = vec![];
    tone(&mut b, 0.0, 0.04, 2000.0, 2000.0, 0.07, Wave::Noise, 3.0);
    tone(&mut b, 0.0, 0.05, 200.0, 90.0, 0.2, Wave::Sine, 1.5);
    b
}

pub fn sfx_die() -> Vec<f32> {
    let mut b = meow(0.9, 700.0, 800.0, 220.0, 0.6);
    let mut hiss = vec![];
    tone(&mut hiss, 0.0, 0.5, 6000.0, 6000.0, 0.22, Wave::Noise, 1.0);
    mix(&mut b, &hiss, 0.05);
    b
}

pub fn sfx_pop(variant: usize) -> Vec<f32> {
    // fur "poof"
    let mut b = vec![];
    let base = [300.0, 380.0, 460.0][variant % 3];
    tone(&mut b, 0.0, 0.1, 3000.0, 3000.0, 0.12, Wave::Noise, 2.0);
    tone(&mut b, 0.0, 0.08, base, base * 0.4, 0.3, Wave::Sine, 1.5);
    b
}

pub fn sfx_level() -> Vec<f32> {
    let mut b = vec![];
    for (i, m) in [72.0, 76.0, 79.0, 84.0, 88.0].iter().enumerate() {
        let t = i as f32 * 0.08;
        tone(&mut b, t, 0.3, mtof(*m), mtof(*m), 0.2, Wave::Sine, 1.6);
        tone(&mut b, t, 0.3, mtof(*m + 12.0), mtof(*m + 12.0), 0.07, Wave::Tri, 2.0);
    }
    mix(&mut b, &purr(0.9, 0.5), 0.35);
    b
}

pub fn sfx_start() -> Vec<f32> {
    let mut b = meow(0.3, 600.0, 1000.0, 800.0, 0.5);
    tone(&mut b, 0.25, 0.2, mtof(84.0), mtof(84.0), 0.15, Wave::Sine, 1.5);
    b
}

pub fn sfx_over() -> Vec<f32> {
    let mut b = meow(1.2, 520.0, 640.0, 160.0, 0.6);
    for (i, m) in [64.0, 62.0, 59.0, 55.0].iter().enumerate() {
        let t = 0.7 + i as f32 * 0.25;
        tone(&mut b, t, 0.5, mtof(*m), mtof(*m) * 0.98, 0.18, Wave::Tri, 1.2);
    }
    b
}

/// Cheerful 8-bar loop: pizzicato walking bass, swung off-beat chords, soft brushes, bell-like lead.
pub fn music() -> Vec<f32> {
    let bpm = 112.0;
    let beat = 60.0 / bpm;
    let bar = beat * 4.0;
    // (root midi, is_minor)
    let prog: [(f32, bool); 8] = [
        (48.0, false),
        (45.0, true),
        (41.0, false),
        (43.0, false),
        (48.0, false),
        (45.0, true),
        (38.0, true),
        (43.0, false),
    ];
    let lead_notes = [0.0, 2.0, 4.0, 7.0, 9.0, 12.0, 14.0, 16.0];
    let swing = beat * 0.5 * 0.28;
    let mut b: Vec<f32> = Vec::new();
    let mut lcg = 11u32;
    for (bi, &(root, minor)) in prog.iter().enumerate() {
        let t0 = bi as f32 * bar;
        let third = if minor { 3.0 } else { 4.0 };
        let chord = [0.0, third, 7.0];
        for &c in &chord {
            let f = mtof(root + 24.0 + c);
            tone(&mut b, t0, bar, f, f, 0.03, Wave::Sine, 0.4);
        }
        // walking bass
        let walk = [0.0, 4.0, 7.0, 4.0];
        for k in 0..4 {
            let n = root + if minor && walk[k] == 4.0 { 3.0 } else { walk[k] };
            let tt = t0 + k as f32 * beat;
            tone(&mut b, tt, beat * 0.55, mtof(n), mtof(n), 0.32, Wave::Tri, 2.2);
            tone(&mut b, tt, beat * 0.2, mtof(n + 12.0), mtof(n + 12.0), 0.06, Wave::Sine, 2.5);
        }
        // swung off-beat comping
        for k in 0..4 {
            let tt = t0 + k as f32 * beat + beat * 0.5 + swing;
            for &c in &chord {
                let f = mtof(root + 24.0 + c);
                tone(&mut b, tt, beat * 0.22, f, f, 0.035, Wave::Square, 2.5);
            }
        }
        // drums: soft kick on 1 and 3, brush on 2 and 4, hats on swung 8ths
        for k in 0..4 {
            let tt = t0 + k as f32 * beat;
            if k % 2 == 0 {
                tone(&mut b, tt, 0.15, 120.0, 48.0, 0.38, Wave::Sine, 2.0);
            } else {
                tone(&mut b, tt, 0.12, 5000.0, 5000.0, 0.07, Wave::Noise, 1.6);
            }
            tone(&mut b, tt, 0.03, 9000.0, 9000.0, 0.03, Wave::Noise, 3.0);
            tone(&mut b, tt + beat * 0.5 + swing, 0.03, 9000.0, 9000.0, 0.02, Wave::Noise, 3.0);
        }
        // bell lead
        for k in 0..8 {
            lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
            let r = (lcg >> 16) as usize;
            let density = if bi < 4 { 3 } else { 5 };
            if r % 8 >= density && bi < 4 && k % 2 == 1 {
                continue;
            }
            if r % 6 == 0 {
                continue;
            }
            let n = 72.0 + lead_notes[r % lead_notes.len()];
            let tt = t0 + k as f32 * beat * 0.5 + if k % 2 == 1 { swing } else { 0.0 };
            tone(&mut b, tt, beat * 0.6, mtof(n), mtof(n), 0.07, Wave::Sine, 1.8);
            tone(&mut b, tt, beat * 0.3, mtof(n + 12.0), mtof(n + 12.0), 0.025, Wave::Sine, 2.5);
        }
    }
    let total = (bar * 8.0 * SR as f32) as usize;
    b.resize(total, 0.0);
    let d = (beat * 0.75 * SR as f32) as usize;
    for i in d..b.len() {
        b[i] += b[i - d] * 0.22;
    }
    let fade = 200.min(b.len());
    let len = b.len();
    for i in 0..fade {
        b[len - 1 - i] *= i as f32 / fade as f32;
    }
    b
}

#[cfg(test)]
mod tests {
    #[test]
    fn synth_ok() {
        let m = super::music();
        assert!(m.len() > 600_000);
        assert!(m.iter().all(|x| x.is_finite()));
        for s in [
            super::sfx_die(),
            super::sfx_fish(),
            super::sfx_level(),
            super::sfx_over(),
            super::sfx_eat(),
        ] {
            assert!(s.iter().all(|x| x.is_finite()));
            assert!(super::to_wav(&s, 1.0).len() > 44);
        }
    }
}
