#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio;

use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, PlaySoundParams, Sound};
use macroquad::color::hsl_to_rgb;
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

const COLS: i32 = 28;
const ROWS: i32 = 20;
const CELL: f32 = 32.0;
const HUD: f32 = 64.0;
const FW: f32 = COLS as f32 * CELL;
const FH: f32 = ROWS as f32 * CELL;
const MUSIC_VOL: f32 = 0.45;
const BONUS_TIME: f32 = 9.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "Long Cat".to_owned(),
        window_width: FW as i32,
        window_height: (FH + HUD) as i32,
        window_resizable: false,
        sample_count: 4,
        ..Default::default()
    }
}

// ---------- helpers ----------
fn rnd(a: f32, b: f32) -> f32 {
    rand::gen_range(a, b)
}
fn col(h: f32, s: f32, l: f32, a: f32) -> Color {
    let mut c = hsl_to_rgb(h.rem_euclid(1.0), s, l);
    c.a = a;
    c
}
fn with_a(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}
fn lerpf(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn rot(v: Vec2, a: f32) -> Vec2 {
    vec2(v.x * a.cos() - v.y * a.sin(), v.x * a.sin() + v.y * a.cos())
}
fn ease_out_back(x: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (x - 1.0).powi(3) + c1 * (x - 1.0).powi(2)
}
fn cell_center(c: IVec2) -> Vec2 {
    vec2((c.x as f32 + 0.5) * CELL, (c.y as f32 + 0.5) * CELL)
}
fn step_for(level: u32) -> f32 {
    (0.16 - 0.008 * (level as f32 - 1.0)).max(0.06)
}
fn hash(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393).wrapping_add((y as u32).wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
}
fn circ(o: Vec2, p: Vec2, r: f32, c: Color) {
    draw_circle(p.x + o.x, p.y + o.y, r, c);
}
fn circ_lines(o: Vec2, p: Vec2, r: f32, th: f32, c: Color) {
    draw_circle_lines(p.x + o.x, p.y + o.y, r, th, c);
}
fn line(o: Vec2, a: Vec2, b: Vec2, t: f32, c: Color) {
    draw_line(a.x + o.x, a.y + o.y, b.x + o.x, b.y + o.y, t, c);
}
fn tri(o: Vec2, a: Vec2, b: Vec2, c: Vec2, col: Color) {
    draw_triangle(a + o, b + o, c + o, col);
}
fn text_c(s: &str, cx: f32, y: f32, size: f32, c: Color) {
    let d = measure_text(s, None, size as u16, 1.0);
    draw_text(s, cx - d.width / 2.0, y, size, c);
}
fn play(s: &Sound, v: f32) {
    play_sound(s, PlaySoundParams { looped: false, volume: v });
}
fn draw_heart(o: Vec2, p: Vec2, s: f32, c: Color) {
    circ(o, p + vec2(-0.5 * s, -0.3 * s), 0.58 * s, c);
    circ(o, p + vec2(0.5 * s, -0.3 * s), 0.58 * s, c);
    tri(o, p + vec2(-1.04 * s, -0.05 * s), p + vec2(1.04 * s, -0.05 * s), p + vec2(0.0, 1.0 * s), c);
}
fn draw_paw_icon(x: f32, y: f32, s: f32, c: Color) {
    draw_circle(x, y + s * 0.25, s * 0.5, c);
    for k in 0..4 {
        let a = -PI / 2.0 + (k as f32 - 1.5) * 0.62;
        draw_circle(x + a.cos() * s * 0.85, y + a.sin() * s * 0.85 - s * 0.05, s * 0.27, c);
    }
}

// ---------- entities ----------
struct Particle {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    max: f32,
    size: f32,
    color: Color,
    grav: f32,
    kind: u8, // 0 dot, 1 heart, 2 sparkle, 3 fur puff
    spin: f32,
}
struct Ring {
    pos: Vec2,
    age: f32,
    max: f32,
    r: f32,
    color: Color,
}
struct Popup {
    pos: Vec2,
    text: String,
    age: f32,
    color: Color,
    size: f32,
}
struct Mote {
    pos: Vec2,
    vel: Vec2,
    ph: f32,
    size: f32,
}
struct Paw {
    pos: Vec2,
    age: f32,
    ang: f32,
}
struct Sounds {
    eat: Sound,
    fish: Sound,
    yarn: Sound,
    turn: Sound,
    die: Sound,
    pop: [Sound; 3],
    level: Sound,
    start: Sound,
    over: Sound,
    music: Sound,
}

#[derive(PartialEq, Clone, Copy)]
enum State {
    Menu,
    Playing,
    Paused,
    Dead,
}
#[derive(PartialEq, Clone, Copy)]
enum Bonus {
    Fish,
    Yarn,
}
#[derive(PartialEq, Clone, Copy)]
enum Eat {
    Mouse,
    Fish,
    Yarn,
}

struct Game {
    state: State,
    snake: Vec<IVec2>,
    prev: Vec<IVec2>,
    dir: IVec2,
    queue: Vec<IVec2>,
    timer: f32,
    step: f32,
    mouse: IVec2,
    mouse_age: f32,
    bonus: Option<(IVec2, f32, Bonus)>,
    bonus_age: f32,
    grow: u32,
    score: u32,
    high: u32,
    new_high: bool,
    level: u32,
    eaten: u32,
    combo: u32,
    combo_t: f32,
    zoomies: f32,
    slow: f32,
    chomp: f32,
    bulges: Vec<f32>,
    particles: Vec<Particle>,
    rings: Vec<Ring>,
    popups: Vec<Popup>,
    motes: Vec<Mote>,
    paws: Vec<Paw>,
    paw_flip: bool,
    shake: f32,
    flash: f32,
    score_pop: f32,
    border_pulse: f32,
    t: f32,
    pop_t: f32,
    pop_n: usize,
    over_t: f32,
    last_head: Vec2,
    music_on: bool,
}

impl Game {
    fn new(high: u32) -> Self {
        let motes = (0..45)
            .map(|_| Mote {
                pos: vec2(rnd(0.0, FW), rnd(0.0, FH)),
                vel: vec2(rnd(-5.0, 5.0), rnd(-14.0, -3.0)),
                ph: rnd(0.0, TAU),
                size: rnd(1.0, 2.8),
            })
            .collect();
        let mut g = Game {
            state: State::Menu,
            snake: vec![],
            prev: vec![],
            dir: ivec2(1, 0),
            queue: vec![],
            timer: 0.0,
            step: step_for(1),
            mouse: ivec2(-1, -1),
            mouse_age: 0.0,
            bonus: None,
            bonus_age: 0.0,
            grow: 0,
            score: 0,
            high,
            new_high: false,
            level: 1,
            eaten: 0,
            combo: 0,
            combo_t: 0.0,
            zoomies: 0.0,
            slow: 0.0,
            chomp: 0.0,
            bulges: vec![],
            particles: vec![],
            rings: vec![],
            popups: vec![],
            motes,
            paws: vec![],
            paw_flip: false,
            shake: 0.0,
            flash: 0.0,
            score_pop: 0.0,
            border_pulse: 0.0,
            t: 0.0,
            pop_t: 0.0,
            pop_n: 0,
            over_t: 0.0,
            last_head: vec2(FW / 2.0, FH / 2.0),
            music_on: true,
        };
        g.reset();
        g.state = State::Menu;
        g
    }

    fn reset(&mut self) {
        self.snake = vec![ivec2(8, 10), ivec2(7, 10), ivec2(6, 10), ivec2(5, 10)];
        self.prev = self.snake.clone();
        self.dir = ivec2(1, 0);
        self.queue.clear();
        self.timer = 0.0;
        self.level = 1;
        self.step = step_for(1);
        self.score = 0;
        self.eaten = 0;
        self.combo = 0;
        self.combo_t = 0.0;
        self.bonus = None;
        self.grow = 0;
        self.zoomies = 0.0;
        self.slow = 0.0;
        self.chomp = 0.0;
        self.new_high = false;
        self.over_t = 0.0;
        self.pop_n = 0;
        self.bulges.clear();
        self.particles.clear();
        self.popups.clear();
        self.rings.clear();
        self.paws.clear();
        self.mouse = self.free_cell();
        self.mouse_age = 0.0;
    }

    /// floor/room colour changes every level
    fn theme(&self) -> f32 {
        [0.075, 0.52, 0.93, 0.72, 0.33][((self.level - 1) % 5) as usize]
    }

    fn cur_step(&self) -> f32 {
        self.step * if self.slow > 0.0 { 1.7 } else { 1.0 }
    }

    fn free_cell(&self) -> IVec2 {
        loop {
            let c = ivec2(rand::gen_range(0, COLS), rand::gen_range(0, ROWS));
            if !self.snake.contains(&c) && c != self.mouse && self.bonus.map_or(true, |g| g.0 != c) {
                return c;
            }
        }
    }

    fn burst(&mut self, pos: Vec2, hue: f32, n: usize, speed: f32, grav: f32, kind: u8) {
        for _ in 0..n {
            let a = rnd(0.0, TAU);
            let s = rnd(0.2, 1.0) * speed;
            let l = rnd(0.5, 1.2);
            let size = match kind {
                1 => rnd(4.0, 8.0),
                2 => rnd(4.0, 7.0),
                3 => rnd(6.0, 12.0),
                _ => rnd(2.0, 5.0),
            };
            let (sat, lig) = if kind == 3 { (0.8, 0.55) } else { (0.9, 0.65) };
            self.particles.push(Particle {
                pos,
                vel: vec2(a.cos(), a.sin()) * s,
                life: l,
                max: l,
                size,
                color: col(hue + rnd(-0.04, 0.04), sat, lig, 1.0),
                grav: if kind == 1 { -90.0 } else { grav },
                kind,
                spin: rnd(0.0, TAU),
            });
        }
    }

    fn visual_points(&self) -> Vec<Vec2> {
        let tt = if self.state == State::Playing {
            (self.timer / self.cur_step()).clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.snake
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                let a = cell_center(self.prev.get(i).copied().unwrap_or(c));
                let b = cell_center(c);
                a + (b - a) * tt
            })
            .collect()
    }

    fn die(&mut self, snd: &Sounds) {
        self.state = State::Dead;
        self.prev = self.snake.clone();
        self.shake = 1.2;
        self.flash = 0.7;
        self.pop_t = 0.0;
        self.pop_n = 0;
        self.combo = 0;
        play(&snd.die, 0.9);
        let hp = cell_center(self.snake[0]);
        self.last_head = hp;
        self.rings.push(Ring { pos: hp, age: 0.0, max: 0.8, r: CELL * 6.0, color: col(0.0, 0.9, 0.55, 1.0) });
        if self.score > self.high {
            self.high = self.score;
            self.new_high = true;
            let _ = std::fs::write("longcat_highscore.txt", self.high.to_string());
        }
    }

    fn on_eat(&mut self, pos: Vec2, what: Eat, snd: &Sounds) {
        if self.combo_t > 0.0 {
            self.combo += 1;
        } else {
            self.combo = 1;
        }
        self.combo_t = 3.0;
        let mult = self.combo.min(8);
        let base = match what {
            Eat::Mouse => 10,
            Eat::Fish => 50,
            Eat::Yarn => 30,
        };
        let pts = base * mult;
        self.score += pts;
        self.score_pop = 1.0;
        self.chomp = 0.35;
        self.bulges.push(0.0);
        let hue = match what {
            Eat::Mouse => 0.95,
            Eat::Fish => 0.55,
            Eat::Yarn => 0.85,
        };
        self.burst(pos, hue, 7, 160.0, 0.0, 1);
        self.burst(pos, 0.13, 16, 280.0, 250.0, 2);
        self.burst(pos, 0.08, 6, 120.0, 0.0, 3);
        self.rings.push(Ring { pos, age: 0.0, max: 0.5, r: CELL * 2.4, color: col(hue, 0.9, 0.65, 1.0) });
        self.popups.push(Popup {
            pos: pos + vec2(0.0, -24.0),
            text: format!("+{}", pts),
            age: 0.0,
            color: col(0.12, 0.95, 0.7, 1.0),
            size: 30.0,
        });
        if mult > 1 {
            self.popups.push(Popup {
                pos: pos + vec2(0.0, -54.0),
                text: format!("x{} COMBO", mult),
                age: 0.0,
                color: col(0.95, 0.9, 0.7, 1.0),
                size: 22.0,
            });
        }
        self.shake = self.shake.max(0.2);
        self.border_pulse = 1.0;
        match what {
            Eat::Fish => {
                self.bonus = None;
                self.grow = 2;
                self.zoomies = 6.0;
                self.flash = 0.25;
                play(&snd.fish, 0.8);
                self.popups.push(Popup {
                    pos: pos + vec2(0.0, -84.0),
                    text: "ZOOMIES!".to_string(),
                    age: 0.0,
                    color: col(0.55, 0.9, 0.7, 1.0),
                    size: 26.0,
                });
            }
            Eat::Yarn => {
                self.bonus = None;
                self.slow = 6.0;
                play(&snd.yarn, 0.8);
                self.popups.push(Popup {
                    pos: pos + vec2(0.0, -84.0),
                    text: "CHILL...".to_string(),
                    age: 0.0,
                    color: col(0.85, 0.8, 0.75, 1.0),
                    size: 26.0,
                });
            }
            Eat::Mouse => {
                self.eaten += 1;
                self.mouse = self.free_cell();
                self.mouse_age = 0.0;
                play(&snd.eat, 0.7);
                if self.eaten % 5 == 0 {
                    self.level += 1;
                    self.step = step_for(self.level);
                    self.flash = 0.35;
                    play(&snd.level, 0.7);
                    let c = vec2(FW / 2.0, FH / 2.0);
                    self.rings.push(Ring {
                        pos: c,
                        age: 0.0,
                        max: 1.0,
                        r: FW * 0.8,
                        color: col(self.theme(), 0.9, 0.6, 1.0),
                    });
                    self.popups.push(Popup {
                        pos: c,
                        text: format!("LEVEL {}", self.level),
                        age: 0.0,
                        color: WHITE,
                        size: 72.0,
                    });
                }
                if self.bonus.is_none() && rnd(0.0, 1.0) < 0.22 {
                    let kind = if rnd(0.0, 1.0) < 0.6 { Bonus::Fish } else { Bonus::Yarn };
                    self.bonus = Some((self.free_cell(), BONUS_TIME, kind));
                    self.bonus_age = 0.0;
                }
            }
        }
    }

    fn step_once(&mut self, snd: &Sounds) {
        if !self.queue.is_empty() {
            self.dir = self.queue.remove(0);
        }
        let old = self.snake.clone();
        let head = self.snake[0] + self.dir;
        let ate = head == self.mouse;
        let bonus_hit = self.bonus.filter(|g| g.0 == head).map(|g| g.2);
        let growing = ate || bonus_hit.is_some() || self.grow > 0;
        let check_len = if growing { self.snake.len() } else { self.snake.len() - 1 };
        if head.x < 0
            || head.y < 0
            || head.x >= COLS
            || head.y >= ROWS
            || self.snake[..check_len].contains(&head)
        {
            self.die(snd);
            return;
        }
        // paw print on the cell we leave
        self.paw_flip = !self.paw_flip;
        let dv = vec2(self.dir.x as f32, self.dir.y as f32);
        let nv = vec2(-dv.y, dv.x);
        let side = if self.paw_flip { 1.0 } else { -1.0 };
        self.paws.push(Paw { pos: cell_center(old[0]) + nv * side * 7.0, age: 0.0, ang: dv.y.atan2(dv.x) });

        self.snake.insert(0, head);
        if ate || bonus_hit.is_some() {
        } else if self.grow > 0 {
            self.grow -= 1;
        } else {
            self.snake.pop();
        }
        let mut prev = old;
        while prev.len() < self.snake.len() {
            let l = *prev.last().unwrap();
            prev.push(l);
        }
        self.prev = prev;
        let hp = cell_center(head);
        if ate {
            self.on_eat(hp, Eat::Mouse, snd);
        }
        match bonus_hit {
            Some(Bonus::Fish) => self.on_eat(hp, Eat::Fish, snd),
            Some(Bonus::Yarn) => self.on_eat(hp, Eat::Yarn, snd),
            None => {}
        }
    }

    fn update(&mut self, dt: f32, snd: &Sounds) {
        self.t += dt;
        self.shake = (self.shake - dt * 1.8).max(0.0);
        self.flash = (self.flash - dt * 2.5).max(0.0);
        self.score_pop = (self.score_pop - dt * 3.0).max(0.0);
        self.border_pulse = (self.border_pulse - dt * 2.0).max(0.0);
        self.chomp = (self.chomp - dt).max(0.0);
        self.mouse_age += dt;
        self.bonus_age += dt;

        for p in &mut self.particles {
            p.life -= dt;
            p.vel.y += p.grav * dt;
            p.vel *= 0.3f32.powf(dt);
            p.pos += p.vel * dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        for r in &mut self.rings {
            r.age += dt;
        }
        self.rings.retain(|r| r.age < r.max);
        for p in &mut self.popups {
            p.age += dt;
            p.pos.y -= 28.0 * dt;
        }
        self.popups.retain(|p| p.age < 1.2);
        for p in &mut self.paws {
            p.age += dt;
        }
        self.paws.retain(|p| p.age < 7.0);
        for m in &mut self.motes {
            m.pos += m.vel * dt;
            m.pos.x += (self.t * 0.6 + m.ph).sin() * 8.0 * dt;
            if m.pos.y < 0.0 {
                m.pos.y = FH;
            }
            if m.pos.x < 0.0 {
                m.pos.x = FW;
            }
            if m.pos.x > FW {
                m.pos.x = 0.0;
            }
        }

        if is_key_pressed(KeyCode::M) {
            self.music_on = !self.music_on;
            set_sound_volume(&snd.music, if self.music_on { MUSIC_VOL } else { 0.0 });
        }

        match self.state {
            State::Menu => {
                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                    self.reset();
                    self.state = State::Playing;
                    play(&snd.start, 0.8);
                }
            }
            State::Paused => {
                if is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Escape) {
                    self.state = State::Playing;
                }
            }
            State::Playing => {
                if is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Escape) {
                    self.state = State::Paused;
                    return;
                }
                let dirs = [
                    ([KeyCode::Up, KeyCode::W], ivec2(0, -1)),
                    ([KeyCode::Down, KeyCode::S], ivec2(0, 1)),
                    ([KeyCode::Left, KeyCode::A], ivec2(-1, 0)),
                    ([KeyCode::Right, KeyCode::D], ivec2(1, 0)),
                ];
                for (keys, d) in dirs.iter() {
                    if keys.iter().any(|k| is_key_pressed(*k)) {
                        let last = self.queue.last().copied().unwrap_or(self.dir);
                        if *d != last && *d != -last && self.queue.len() < 3 {
                            self.queue.push(*d);
                            play(&snd.turn, 0.5);
                        }
                    }
                }
                if self.combo_t > 0.0 {
                    self.combo_t -= dt;
                    if self.combo_t <= 0.0 {
                        self.combo = 0;
                    }
                }
                self.zoomies = (self.zoomies - dt).max(0.0);
                self.slow = (self.slow - dt).max(0.0);
                if let Some(g) = self.bonus.as_mut() {
                    g.1 -= dt;
                }
                if self.bonus.map_or(false, |g| g.1 <= 0.0) {
                    let p = cell_center(self.bonus.unwrap().0);
                    self.burst(p, 0.13, 16, 150.0, 100.0, 2);
                    self.burst(p, 0.1, 5, 90.0, 0.0, 3);
                    self.bonus = None;
                }
                self.timer += dt;
                while self.timer >= self.cur_step() && self.state == State::Playing {
                    self.timer -= self.cur_step();
                    self.step_once(snd);
                }
                let len = self.snake.len() as f32;
                let cs = self.cur_step();
                for b in &mut self.bulges {
                    *b += dt / cs;
                }
                self.bulges.retain(|b| *b < len + 1.0);
                if self.state == State::Playing && self.particles.len() < 2500 {
                    let head = self.visual_points()[0];
                    self.last_head = head;
                    if self.zoomies > 0.0 {
                        if rnd(0.0, 1.0) < 0.5 {
                            self.burst(head, rnd(0.0, 1.0), 1, 50.0, 0.0, 1);
                        }
                        self.burst(head, rnd(0.0, 1.0), 2, 60.0, 0.0, 2);
                    } else if rnd(0.0, 1.0) < 0.3 {
                        self.burst(head, 0.1, 1, 30.0, 0.0, 2);
                    }
                }
            }
            State::Dead => {
                self.pop_t -= dt;
                while self.pop_t <= 0.0 && !self.snake.is_empty() {
                    self.pop_t += 0.045;
                    let seg = self.snake.remove(0);
                    self.prev = self.snake.clone();
                    let p = cell_center(seg);
                    self.burst(p, 0.08, 6, 260.0, 300.0, 3);
                    self.burst(p, 0.12, 6, 300.0, 380.0, 2);
                    if self.pop_n % 2 == 0 {
                        play(&snd.pop[self.pop_n / 2 % 3], 0.35);
                    }
                    self.pop_n += 1;
                    if self.snake.is_empty() {
                        play(&snd.over, 0.7);
                    }
                }
                if self.snake.is_empty() {
                    self.over_t += dt;
                    if self.over_t > 0.7 && (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter)) {
                        self.reset();
                        self.state = State::Playing;
                        play(&snd.start, 0.8);
                    }
                }
            }
        }
    }

    // ---------- drawing ----------
    fn draw(&self) {
        let shake_off = if self.shake > 0.0 {
            vec2(rnd(-1.0, 1.0), rnd(-1.0, 1.0)) * self.shake.min(1.0) * 10.0
        } else {
            Vec2::ZERO
        };
        let o = vec2(0.0, HUD) + shake_off;
        let t = self.t;
        let theme = self.theme();
        clear_background(col(theme, 0.5, 0.05, 1.0));

        let pts: Vec<Vec2> = if self.state == State::Menu {
            (0..30)
                .map(|i| {
                    let tt = t * 0.9 - i as f32 * 0.075;
                    vec2(FW / 2.0 + (tt * 0.7).cos() * FW * 0.36, FH / 2.0 + (tt * 1.1).sin() * FH * 0.34)
                })
                .collect()
        } else {
            self.visual_points()
        };
        let focus = pts.first().copied().unwrap_or(self.last_head);

        draw_floor(o, t, theme, focus, self.border_pulse);
        for p in &self.paws {
            let a = (1.0 - p.age / 7.0).clamp(0.0, 1.0) * 0.28;
            draw_paw_print(o, p.pos, p.ang, a);
        }
        for m in &self.motes {
            let a = 0.3 + 0.3 * (t * 2.0 + m.ph).sin();
            circ(o, m.pos, m.size * 2.4, col(0.12, 0.8, 0.8, a * 0.12));
            circ(o, m.pos, m.size, col(0.12, 0.5, 0.92, a));
        }

        if self.state != State::Menu {
            let mc = cell_center(self.mouse);
            let away = mc - focus;
            let dist = away.length() / CELL;
            let fear = ((6.0 - dist) / 6.0).clamp(0.0, 1.0);
            draw_mouse(o, mc, t, self.mouse_age, away, fear);
            if let Some((c, left, kind)) = self.bonus {
                match kind {
                    Bonus::Fish => draw_fish(o, cell_center(c), t, self.bonus_age, left / BONUS_TIME),
                    Bonus::Yarn => draw_yarn(o, cell_center(c), t, self.bonus_age, left / BONUS_TIME),
                }
            }
        }

        let look = if self.state == State::Menu { vec2(FW / 2.0, FH / 2.0) } else { cell_center(self.mouse) };
        let mood = if self.state == State::Dead {
            2
        } else if self.zoomies > 0.0 {
            1
        } else {
            0
        };
        draw_cat(o, &pts, t, mood, &self.bulges, look, self.chomp / 0.35);

        for r in &self.rings {
            let k = r.age / r.max;
            let e = 1.0 - (1.0 - k).powi(3);
            circ_lines(o, r.pos, r.r * e, 2.0 + 6.0 * (1.0 - k), with_a(r.color, (1.0 - k) * 0.8));
        }
        for p in &self.particles {
            let a = (p.life / p.max).clamp(0.0, 1.0);
            let s = p.size * (0.4 + 0.6 * a);
            match p.kind {
                1 => draw_heart(o, p.pos, s, with_a(p.color, a)),
                2 => {
                    let c = with_a(p.color, a);
                    let k = s * 1.8;
                    let ang = p.spin + p.life * 4.0;
                    for j in 0..2 {
                        let d = rot(vec2(k, 0.0), ang + j as f32 * PI / 2.0);
                        line(o, p.pos - d, p.pos + d, 2.0, c);
                    }
                    circ(o, p.pos, s * 0.5, with_a(WHITE, a));
                }
                3 => {
                    circ(o, p.pos, s * 1.6, with_a(p.color, a * 0.3));
                    circ(o, p.pos, s, with_a(p.color, a * 0.6));
                }
                _ => {
                    circ(o, p.pos, s * 2.2, with_a(p.color, a * 0.15));
                    circ(o, p.pos, s, with_a(p.color, a));
                }
            }
        }
        for p in &self.popups {
            let k = p.age / 1.2;
            let s = p.size * (1.0 + 0.4 * (1.0 - (k * 6.0).min(1.0)));
            let a = (1.0 - k).clamp(0.0, 1.0);
            let sp = p.pos + o;
            text_c(&p.text, sp.x + 2.0, sp.y + 2.0, s, Color::new(0.0, 0.0, 0.0, a * 0.6));
            text_c(&p.text, sp.x, sp.y, s, with_a(p.color, a));
        }

        for i in 0..14 {
            let a = (1.0 - i as f32 / 14.0).powi(2) * 0.4;
            let inset = i as f32 * 6.0;
            draw_rectangle_lines(
                o.x + inset,
                o.y + inset,
                FW - inset * 2.0,
                FH - inset * 2.0,
                7.0,
                Color::new(0.0, 0.0, 0.0, a),
            );
        }
        if self.flash > 0.0 {
            draw_rectangle(o.x, o.y, FW, FH, Color::new(1.0, 1.0, 1.0, self.flash * 0.45));
        }

        self.draw_hud();
        self.draw_overlays();
    }

    fn draw_hud(&self) {
        let th = self.theme();
        let wood = col(0.07, 0.5, 0.13, 1.0);
        draw_rectangle(0.0, 0.0, FW, HUD, wood);
        for i in 0..8 {
            draw_rectangle(0.0, i as f32 * 8.0, FW, 1.0, col(0.07, 0.5, 0.1, 0.6));
        }
        draw_rectangle(0.0, HUD - 4.0, FW, 4.0, col(0.08, 0.7, 0.3, 1.0));
        draw_rectangle(0.0, HUD - 4.0, FW, 1.5, col(0.09, 0.8, 0.5, 1.0));
        for (i, ch) in "LONG CAT".chars().enumerate() {
            let x = 52.0 + i as f32 * 25.0;
            let y = 44.0 + (self.t * 3.0 - i as f32 * 0.5).sin() * 3.0;
            draw_text(&ch.to_string(), x + 2.0, y + 2.0, 42.0, Color::new(0.0, 0.0, 0.0, 0.5));
            draw_text(&ch.to_string(), x, y, 42.0, col(0.08 + i as f32 * 0.005, 0.95, 0.6, 1.0));
        }
        draw_paw_icon(30.0, 34.0, 11.0 + (self.t * 4.0).sin() * 0.8, col(0.08, 0.95, 0.62, 1.0));
        let lab = col(0.1, 0.5, 0.75, 1.0);
        draw_text("SCORE", 330.0, 24.0, 18.0, lab);
        draw_text(&format!("{}", self.score), 330.0, 54.0, 34.0 + self.score_pop * 12.0, WHITE);
        draw_text("LEVEL", 470.0, 24.0, 18.0, lab);
        draw_text(&format!("{}", self.level), 470.0, 54.0, 34.0, col(0.14, 0.9, 0.7, 1.0));
        draw_text("BEST", 580.0, 24.0, 18.0, lab);
        draw_text(&format!("{}", self.high.max(self.score)), 580.0, 54.0, 34.0, col(0.95, 0.6, 0.75, 1.0));
        let _ = th;
        if self.combo > 1 && self.state == State::Playing {
            let w = 110.0 * (self.combo_t / 3.0).clamp(0.0, 1.0);
            draw_text(&format!("COMBO x{}", self.combo.min(8)), 730.0, 28.0, 22.0, col(0.14, 0.95, 0.65, 1.0));
            draw_rectangle(730.0, 38.0, 110.0, 8.0, Color::new(0.0, 0.0, 0.0, 0.3));
            draw_rectangle(730.0, 38.0, w, 8.0, col(0.14 - 0.1 * (1.0 - w / 110.0), 0.95, 0.55, 1.0));
        }
        if self.zoomies > 0.0 {
            draw_text("ZOOMIES!", 730.0, 62.0, 16.0, col(self.t * 1.5, 0.9, 0.7, 1.0));
        } else if self.slow > 0.0 {
            draw_text("chill...", 730.0, 62.0, 16.0, col(0.85, 0.7, 0.75, 1.0));
        }
    }

    fn draw_overlays(&self) {
        let cx = FW / 2.0;
        let cy = HUD + FH / 2.0;
        match self.state {
            State::Menu => {
                draw_rectangle(0.0, HUD, FW, FH, Color::new(0.0, 0.0, 0.0, 0.3));
                let title = "LONG CAT";
                let size = 120.0;
                let mut total = 0.0;
                for ch in title.chars() {
                    total += measure_text(&ch.to_string(), None, size as u16, 1.0).width.max(size * 0.3);
                }
                let mut x = cx - total / 2.0;
                for (i, ch) in title.chars().enumerate() {
                    let s = ch.to_string();
                    let w = measure_text(&s, None, size as u16, 1.0).width.max(size * 0.3);
                    let y = cy - 60.0 + (self.t * 3.0 - i as f32 * 0.45).sin() * 12.0;
                    draw_text(&s, x + 5.0, y + 6.0, size, Color::new(0.0, 0.0, 0.0, 0.7));
                    draw_text(&s, x, y, size, col(0.07 + (self.t * 0.6 - i as f32 * 0.25).sin() * 0.02, 0.95, 0.58, 1.0));
                    x += w;
                }
                text_c("~ the stretchiest cat in town ~", cx, cy - 10.0, 28.0, col(0.1, 0.6, 0.85, 1.0));
                let a = 0.55 + 0.45 * (self.t * 4.0).sin();
                text_c("PRESS SPACE TO START", cx, cy + 55.0, 36.0, Color::new(1.0, 1.0, 1.0, a));
                text_c("Arrows / WASD: steer    P: pause    M: music", cx, cy + 105.0, 22.0, Color::new(1.0, 1.0, 1.0, 0.65));
                text_c(
                    "Catch mice  |  fish = zoomies  |  yarn = chill  |  don't trip on your tail",
                    cx,
                    cy + 135.0,
                    20.0,
                    col(0.1, 0.7, 0.8, 0.9),
                );
            }
            State::Paused => {
                draw_rectangle(0.0, HUD, FW, FH, Color::new(0.0, 0.0, 0.0, 0.55));
                text_c("NAP TIME", cx, cy, 80.0, WHITE);
                text_c("press P to wake up", cx, cy + 45.0, 26.0, Color::new(1.0, 1.0, 1.0, 0.7));
            }
            State::Dead => {
                if self.snake.is_empty() {
                    let k = (self.over_t / 0.6).clamp(0.0, 1.0);
                    draw_rectangle(0.0, HUD, FW, FH, Color::new(0.0, 0.0, 0.0, 0.55 * k));
                    let title = "CAT-ASTROPHE!";
                    let size = 84.0;
                    let mut total = 0.0;
                    for ch in title.chars() {
                        total += measure_text(&ch.to_string(), None, size as u16, 1.0).width.max(size * 0.3);
                    }
                    let mut x = cx - total / 2.0;
                    for (i, ch) in title.chars().enumerate() {
                        let s = ch.to_string();
                        let w = measure_text(&s, None, size as u16, 1.0).width.max(size * 0.3);
                        let y = cy - 40.0 + (self.t * 4.0 + i as f32 * 0.6).sin() * 6.0 - (1.0 - k) * 60.0;
                        draw_text(&s, x + 3.0, y + 4.0, size, Color::new(0.0, 0.0, 0.0, 0.7 * k));
                        draw_text(&s, x, y, size, col(0.07, 0.9, 0.58, k));
                        x += w;
                    }
                    text_c(&format!("SCORE  {}", self.score), cx, cy + 25.0, 40.0, with_a(WHITE, k));
                    if self.new_high {
                        let c = col(self.t * 2.0, 0.9, 0.7, k);
                        text_c("NEW HIGH SCORE!", cx, cy + 70.0, 34.0, c);
                    } else {
                        text_c(&format!("BEST  {}", self.high), cx, cy + 70.0, 28.0, with_a(Color::new(0.85, 0.8, 0.8, 1.0), k));
                    }
                    if self.over_t > 0.7 {
                        let a = 0.55 + 0.45 * (self.t * 4.0).sin();
                        text_c("PRESS SPACE FOR ANOTHER LIFE", cx, cy + 125.0, 28.0, Color::new(1.0, 1.0, 1.0, a));
                    }
                }
            }
            State::Playing => {}
        }
    }
}

// ---------- scenery ----------
fn draw_paw_print(o: Vec2, p: Vec2, ang: f32, a: f32) {
    let c = Color::new(0.08, 0.04, 0.02, a);
    circ(o, p, 5.5, c);
    for k in 0..4 {
        let aa = ang + (-0.95 + 0.63 * k as f32);
        circ(o, p + vec2(aa.cos(), aa.sin()) * 9.0, 2.8, c);
    }
}

fn draw_floor(o: Vec2, t: f32, theme: f32, focus: Vec2, pulse: f32) {
    for cy in 0..ROWS {
        for cx in 0..COLS {
            let x = o.x + cx as f32 * CELL;
            let y = o.y + cy as f32 * CELL;
            let off = cy % 2;
            let hsh = hash((cx + off) / 2, cy);
            let c = cell_center(ivec2(cx, cy));
            let d = (c - focus).length() / CELL;
            let l = 0.30 + 0.05 * hsh + 0.02 * (cx as f32 * 0.9 + cy as f32 * 0.3).sin() + 0.06 / (1.0 + d * d * 0.2);
            draw_rectangle(x, y, CELL + 0.5, CELL + 0.5, col(theme + 0.01 * hsh, 0.45, l, 1.0));
            let gy = y + CELL * (0.25 + 0.5 * hsh);
            draw_line(x + 3.0, gy, x + CELL - 3.0, gy, 1.0, col(theme, 0.5, l - 0.06, 0.45));
            draw_line(x, y + CELL, x + CELL, y + CELL, 1.5, Color::new(0.0, 0.0, 0.0, 0.32));
            if (cx + off) % 2 == 0 {
                draw_line(x, y, x, y + CELL, 1.5, Color::new(0.0, 0.0, 0.0, 0.28));
            }
        }
    }
    // rug
    let rc = col(theme + 0.5, 0.5, 0.38, 0.3);
    let (rx, ry, rw, rh) = (o.x + 6.0 * CELL, o.y + 4.0 * CELL, 16.0 * CELL, 12.0 * CELL);
    draw_rectangle(rx, ry, rw, rh, rc);
    draw_rectangle_lines(rx + 6.0, ry + 6.0, rw - 12.0, rh - 12.0, 4.0, col(theme + 0.5, 0.5, 0.65, 0.35));
    draw_rectangle_lines(rx + 18.0, ry + 18.0, rw - 36.0, rh - 36.0, 2.0, col(theme + 0.5, 0.5, 0.7, 0.3));
    for k in 0..16 {
        let fx = rx + k as f32 * CELL + CELL * 0.5;
        draw_line(fx, ry - 5.0, fx, ry, 1.5, col(theme + 0.5, 0.5, 0.65, 0.35));
        draw_line(fx, ry + rh, fx, ry + rh + 5.0, 1.5, col(theme + 0.5, 0.5, 0.65, 0.35));
    }
    // sunbeams
    for k in 0..3 {
        let x0 = 140.0 + k as f32 * 330.0 + (t * 0.15 + k as f32).sin() * 40.0;
        let a = vec2(x0, 0.0);
        let b = vec2(x0 + 90.0, 0.0);
        let c = vec2(x0 + 90.0 - 260.0, FH);
        let d = vec2(x0 - 260.0, FH);
        let col_b = col(0.12, 0.9, 0.7, 0.05);
        tri(o, a, b, c, col_b);
        tri(o, a, c, d, col_b);
    }
    // warm glow around the cat
    for i in 0..5 {
        circ(o, focus, CELL * (1.4 + i as f32 * 0.9), col(0.1, 0.8, 0.6, 0.02));
    }
    // baseboard
    let bb = col(0.07, 0.55, 0.12, 1.0);
    let hi = col(0.08, 0.6, 0.28 + 0.2 * pulse, 1.0);
    draw_rectangle(o.x, o.y, FW, 10.0, bb);
    draw_rectangle(o.x, o.y + FH - 10.0, FW, 10.0, bb);
    draw_rectangle(o.x, o.y, 10.0, FH, bb);
    draw_rectangle(o.x + FW - 10.0, o.y, 10.0, FH, bb);
    draw_rectangle_lines(o.x + 10.0, o.y + 10.0, FW - 20.0, FH - 20.0, 2.0, hi);
    draw_rectangle_lines(o.x, o.y, FW, FH, 3.0, Color::new(0.0, 0.0, 0.0, 0.6));
}

// ---------- food ----------
fn draw_mouse(o: Vec2, p: Vec2, t: f32, age: f32, away: Vec2, fear: f32) {
    let pop = ease_out_back((age * 2.5).min(1.0));
    let r = CELL * 0.3 * pop;
    let jit = vec2(rnd(-1.0, 1.0), rnd(-1.0, 1.0)) * fear * 1.6;
    let hop = (t * 6.0 + p.x * 0.1).sin().max(0.0) * 3.0 * (0.4 + fear);
    let c = p + jit + vec2(0.0, -hop);
    let dir = away.try_normalize().unwrap_or(vec2(1.0, 0.0));
    let side = vec2(-dir.y, dir.x);
    circ(o, c, r * 2.8, col(0.12, 0.8, 0.6, 0.05));
    circ(o, c, r * 1.8, col(0.12, 0.8, 0.6, 0.06));
    draw_ellipse(c.x + o.x + 2.0, p.y + o.y + r * 0.9, r * 1.2, r * 0.5, 0.0, Color::new(0.0, 0.0, 0.0, 0.3));
    let mut prev = c - dir * r * 0.9;
    for k in 1..=7 {
        let kk = k as f32;
        let pt = c - dir * r * (0.9 + 0.4 * kk) + side * (t * 9.0 - kk * 0.9).sin() * r * 0.12 * kk;
        line(o, prev, pt, 2.2, col(0.0, 0.35, 0.72, 1.0));
        prev = pt;
    }
    let body = col(0.08, 0.1, 0.6, 1.0);
    let dark = col(0.08, 0.15, 0.25, 1.0);
    let pink = col(0.97, 0.55, 0.75, 1.0);
    for s in [-1.0f32, 1.0] {
        let ep = c + dir * r * 0.25 + side * s * r * 0.8;
        circ(o, ep, r * 0.55 + 1.5, dark);
        circ(o, ep, r * 0.55, body);
        circ(o, ep, r * 0.32, pink);
    }
    circ(o, c - dir * r * 0.25, r + 1.5, dark);
    circ(o, c + dir * r * 0.55, r * 0.7 + 1.5, dark);
    circ(o, c - dir * r * 0.25, r, body);
    circ(o, c + dir * r * 0.55, r * 0.7, body);
    circ(o, c - dir * r * 0.4 + vec2(-r * 0.2, -r * 0.35), r * 0.45, col(0.08, 0.1, 0.78, 0.5));
    for s in [-1.0f32, 1.0] {
        let e = c + dir * r * 0.8 + side * s * r * 0.3;
        circ(o, e, r * 0.16, BLACK);
        circ(o, e + vec2(-0.5, -0.8), r * 0.05, WHITE);
        for k in 0..2 {
            let d = rot(side * s, (k as f32 - 0.5) * 0.5 * s);
            let b0 = c + dir * r * 1.15 + side * s * r * 0.1;
            line(o, b0, b0 + (d + dir * 0.4) * r * 0.9, 1.0, Color::new(1.0, 1.0, 1.0, 0.7));
        }
    }
    circ(o, c + dir * r * 1.25, r * 0.17, pink);
}

fn draw_fish(o: Vec2, p: Vec2, t: f32, age: f32, life: f32) {
    let pop = ease_out_back((age * 2.5).min(1.0));
    let r = CELL * 0.34 * pop;
    let bob = (t * 3.0).sin() * 2.0;
    let c = p + vec2(0.0, bob);
    let ang = (t * 3.0).sin() * 0.3;
    let lp = |v: Vec2| c + rot(v, ang);
    for k in 0..5 {
        circ(o, c, r * (3.0 - k as f32 * 0.5), col(0.55, 0.9, 0.6, 0.05));
    }
    let wag = (t * 10.0).sin() * r * 0.35;
    let body = col(0.55, 0.65, 0.58, 1.0);
    let dark = col(0.58, 0.7, 0.28, 1.0);
    let belly = col(0.55, 0.5, 0.82, 1.0);
    // tail + fins (drawn first)
    let tb = lp(vec2(-r * 1.1, 0.0));
    tri(o, tb, lp(vec2(-r * 1.9, -r * 0.8 + wag)), lp(vec2(-r * 1.9, r * 0.8 + wag)), dark);
    tri(o, lp(vec2(-r * 0.2, -r * 0.7)), lp(vec2(r * 0.2, -r * 1.3)), lp(vec2(r * 0.5, -r * 0.7)), dark);
    for (dx, rr) in [(-0.55, 0.7), (0.0, 0.85), (0.55, 0.68)] {
        circ(o, lp(vec2(r * dx, 0.0)), r * rr + 1.5, dark);
    }
    for (dx, rr) in [(-0.55, 0.7), (0.0, 0.85), (0.55, 0.68)] {
        circ(o, lp(vec2(r * dx, 0.0)), r * rr, body);
    }
    circ(o, lp(vec2(0.0, r * 0.3)), r * 0.55, belly);
    for k in 0..3 {
        let x = -r * 0.5 + k as f32 * r * 0.45;
        line(o, lp(vec2(x, -r * 0.7)), lp(vec2(x + r * 0.1, r * 0.1)), 2.0, Color::new(0.58, 0.7, 0.3, 0.6));
    }
    circ(o, lp(vec2(r * 0.7, -r * 0.15)), r * 0.2, WHITE);
    circ(o, lp(vec2(r * 0.74, -r * 0.15)), r * 0.1, BLACK);
    circ(o, lp(vec2(-r * 0.1, -r * 0.5)), r * 0.3, Color::new(1.0, 1.0, 1.0, 0.3));
    // timer ring
    let segs = 40;
    let sweep = life.clamp(0.0, 1.0) * TAU;
    for k in 0..segs {
        let a0 = -PI / 2.0 + sweep * k as f32 / segs as f32;
        let a1 = -PI / 2.0 + sweep * (k + 1) as f32 / segs as f32;
        let rr = CELL * 0.9;
        line(
            o,
            p + vec2(a0.cos(), a0.sin()) * rr,
            p + vec2(a1.cos(), a1.sin()) * rr,
            3.0,
            col(lerpf(0.0, 0.55, life), 0.9, 0.6, 0.9),
        );
    }
}

fn draw_yarn(o: Vec2, p: Vec2, t: f32, age: f32, life: f32) {
    let pop = ease_out_back((age * 2.5).min(1.0));
    let r = CELL * 0.36 * pop;
    let c = p + vec2(0.0, (t * 3.0).sin() * 2.0);
    for k in 0..5 {
        circ(o, c, r * (3.0 - k as f32 * 0.5), col(0.85, 0.8, 0.6, 0.05));
    }
    // thread
    let mut prev = c + vec2(r * 0.6, r * 0.8);
    for k in 1..=7 {
        let kk = k as f32;
        let pt = c + vec2(r * 0.6 + kk * r * 0.3, r * 0.8 + (t * 4.0 + kk).sin() * r * 0.2 + kk * r * 0.05);
        line(o, prev, pt, 2.5, col(0.85, 0.7, 0.55, 1.0));
        prev = pt;
    }
    circ(o, c + vec2(2.0, 4.0), r, Color::new(0.0, 0.0, 0.0, 0.3));
    circ(o, c, r + 1.5, col(0.85, 0.7, 0.25, 1.0));
    circ(o, c, r, col(0.85, 0.7, 0.55, 1.0));
    let spin = t * 1.2;
    for k in 0..6 {
        let a = spin + k as f32 * 0.55;
        line(
            o,
            c + rot(vec2(r * 0.95, 0.0), a),
            c + rot(vec2(r * 0.95, 0.0), a + 2.3),
            2.0,
            col(0.85, 0.7, 0.32, 0.8),
        );
    }
    circ(o, c + vec2(-r * 0.35, -r * 0.4), r * 0.28, Color::new(1.0, 1.0, 1.0, 0.4));
    let segs = 40;
    let sweep = life.clamp(0.0, 1.0) * TAU;
    for k in 0..segs {
        let a0 = -PI / 2.0 + sweep * k as f32 / segs as f32;
        let a1 = -PI / 2.0 + sweep * (k + 1) as f32 / segs as f32;
        let rr = CELL * 0.9;
        line(
            o,
            p + vec2(a0.cos(), a0.sin()) * rr,
            p + vec2(a1.cos(), a1.sin()) * rr,
            3.0,
            col(0.85, 0.7, 0.6, 0.9),
        );
    }
}

// ---------- the cat ----------
fn chaikin(p: &[Vec2]) -> Vec<Vec2> {
    if p.len() < 3 {
        return p.to_vec();
    }
    let mut o = vec![p[0]];
    for w in p.windows(2) {
        o.push(w[0] * 0.75 + w[1] * 0.25);
        o.push(w[0] * 0.25 + w[1] * 0.75);
    }
    o.push(*p.last().unwrap());
    o
}

/// mood: 0 normal tabby, 1 zoomies (rainbow fur), 2 knocked out
fn draw_cat(o: Vec2, pts: &[Vec2], t: f32, mood: u8, bulges: &[f32], look: Vec2, chomp: f32) {
    if pts.is_empty() {
        return;
    }
    let mut raw = pts.to_vec();
    if raw.len() == 1 {
        raw.push(raw[0] - vec2(1.0, 0.0));
    }
    let mut sm = chaikin(&chaikin(&raw));
    let n = sm.len();
    let tan: Vec<Vec2> = (0..n)
        .map(|i| (sm[i.saturating_sub(1)] - sm[(i + 1).min(n - 1)]).try_normalize().unwrap_or(vec2(1.0, 0.0)))
        .collect();
    // swishing tail
    let start = (n as f32 * 0.6) as usize;
    for i in start..n {
        let k = (i - start) as f32 / (n - start).max(1) as f32;
        let nm = vec2(-tan[i].y, tan[i].x);
        sm[i] += nm * (t * 7.0 - i as f32 * 0.25).sin() * k.powf(1.5) * 7.0;
    }
    let nrm = |i: usize| vec2(-tan[i].y, tan[i].x);
    let info = |i: usize| -> (f32, Color, f32) {
        let seg = i as f32 / 4.0;
        let frac = i as f32 / (n - 1).max(1) as f32;
        let mut r = CELL * (0.40 - 0.26 * frac.powf(1.8));
        for b in bulges {
            let d = seg - b;
            r += 0.18 * CELL * (-d * d / 1.2).exp();
        }
        let (h, s, l) = match mood {
            1 => (t * 0.8 - seg * 0.06, 0.8, 0.58),
            2 => (0.08, 0.08, 0.42),
            _ => (0.075, 0.88, 0.55 + 0.04 * (seg * 0.8 - t * 4.0).sin()),
        };
        let mut c = col(h, s, l, 1.0);
        if mood != 2 && i + 6 > n {
            c = col(0.1, 0.25, 0.92, 1.0);
        }
        (r, c, h)
    };
    let cream = col(0.1, 0.4, 0.88, 1.0);

    if mood == 1 {
        for i in (0..n).rev().step_by(2) {
            let (r, c, _) = info(i);
            circ(o, sm[i], r * 2.0, with_a(c, 0.04));
        }
    }
    for i in (0..n).rev() {
        let (r, _, _) = info(i);
        circ(o, sm[i] + vec2(3.0, 5.0), r, Color::new(0.0, 0.0, 0.0, 0.25));
    }
    // little walking paws
    for i in (10..n.saturating_sub(10)).step_by(12) {
        let (r, _, h) = info(i);
        let nm = nrm(i);
        for s in [-1.0f32, 1.0] {
            let off = (t * 9.0 + i as f32 + s * 1.5).sin() * 5.0;
            let pp = sm[i] + nm * s * r * 0.95 + tan[i] * off;
            circ(o, pp, r * 0.46 + 1.8, col(h, 0.7, 0.18, 1.0));
            circ(o, pp, r * 0.46, cream);
        }
    }
    for i in (0..n).rev() {
        let (r, _, h) = info(i);
        circ(o, sm[i], r + 2.0, col(h, 0.7, 0.16, 1.0));
    }
    for i in (0..n).rev() {
        let (r, c, h) = info(i);
        circ(o, sm[i], r, c);
        circ(o, sm[i] + vec2(-r * 0.25, -r * 0.3), r * 0.55, col(h, 0.7, 0.68, 0.3));
    }
    // tabby stripes
    for i in (5..n.saturating_sub(5)).step_by(5) {
        let (r, _, h) = info(i);
        let nm = nrm(i);
        line(o, sm[i] + nm * r * 0.88, sm[i] - nm * r * 0.88, 3.5, col(h, 0.9, 0.26, 0.55));
    }
    // collar with a jingly bell
    if n > 12 {
        let i = 7;
        let (r, _, _) = info(i);
        let nm = nrm(i);
        line(o, sm[i] + nm * r * 1.0, sm[i] - nm * r * 1.0, 6.0, col(0.98, 0.85, 0.42, 1.0));
        let bp = sm[i] + vec2((t * 12.0).sin() * 1.2, 0.0);
        circ(o, bp, 6.5, col(0.12, 0.9, 0.3, 1.0));
        circ(o, bp, 5.0, col(0.13, 0.95, 0.6, 1.0));
        circ(o, bp + vec2(-1.5, -1.8), 1.8, WHITE);
        line(o, bp + vec2(-3.0, 1.0), bp + vec2(3.0, 1.0), 1.0, col(0.1, 0.8, 0.25, 1.0));
    }

    // ---- head ----
    let head = raw[0];
    let dir = (head - sm[3.min(n - 1)]).try_normalize().unwrap_or(vec2(1.0, 0.0));
    let side = vec2(-dir.y, dir.x);
    let ang = dir.y.atan2(dir.x);
    let (_, fur, h) = info(0);
    let dark = col(h, 0.7, 0.16, 1.0);
    let hr = CELL * 0.6;
    let pinkc = col(0.97, 0.55, 0.75, 1.0);

    // ears
    for s in [-1.0f32, 1.0] {
        let twitch = if (t * 0.7 + s).fract() < 0.1 { 0.25 } else { 0.0 };
        let wig = (t * 2.3 + s).sin() * 0.05 + twitch * (t * 40.0).sin();
        let pt = |a: f32, rad: f32| head + vec2((ang + s * a).cos(), (ang + s * a).sin()) * rad;
        let b1 = pt(0.7, hr * 0.92);
        let b2 = pt(1.9, hr * 0.92);
        let tip = pt(1.3 + wig, hr * 1.75);
        let cen = (b1 + b2 + tip) / 3.0;
        let grow = |p: Vec2| cen + (p - cen) * 1.2;
        tri(o, grow(b1), grow(b2), grow(tip), dark);
        tri(o, b1, b2, tip, fur);
        tri(o, pt(0.95, hr * 0.85), pt(1.7, hr * 0.85), pt(1.3 + wig, hr * 1.4), pinkc);
    }
    // head + cheek fluff
    let cheeks = [head + dir * hr * 0.25 + side * hr * 0.72, head + dir * hr * 0.25 - side * hr * 0.72];
    circ(o, head, hr + 2.5, dark);
    for c in cheeks {
        circ(o, c, hr * 0.42 + 2.0, dark);
    }
    circ(o, head, hr, fur);
    for c in cheeks {
        circ(o, c, hr * 0.42, fur);
    }
    circ(o, head + vec2(-hr * 0.2, -hr * 0.3), hr * 0.55, col(h, 0.7, 0.7, 0.28));
    circ(o, head + dir * hr * 0.55, hr * 0.4, cream);
    if mood != 2 {
        for k in -1..=1 {
            let b0 = head + dir * hr * 0.05 + side * hr * 0.26 * k as f32;
            line(o, b0, b0 - dir * hr * 0.42, 2.2, col(h, 0.9, 0.26, 0.6));
        }
    }
    // eyes
    let blink = (t % 4.1) < 0.13;
    let lv = (look - head).try_normalize().unwrap_or(dir);
    for s in [-1.0f32, 1.0] {
        let ep = head + dir * hr * 0.28 + side * s * hr * 0.44;
        if mood == 2 {
            let d = hr * 0.17;
            line(o, ep + vec2(-d, -d), ep + vec2(d, d), 3.0, BLACK);
            line(o, ep + vec2(-d, d), ep + vec2(d, -d), 3.0, BLACK);
        } else if blink {
            line(o, ep - dir * hr * 0.2, ep + dir * hr * 0.2, 3.0, dark);
        } else {
            circ(o, ep, hr * 0.3 + 1.5, dark);
            circ(o, ep, hr * 0.3, col(0.2, 0.85, 0.55, 1.0));
            circ(o, ep + dir * hr * 0.03, hr * 0.2, col(0.17, 0.9, 0.62, 1.0));
            let pc = ep + lv * hr * 0.06;
            let w = if chomp > 0.0 { hr * 0.19 } else { hr * 0.08 };
            let hl = hr * 0.23;
            line(o, pc - dir * hl, pc + dir * hl, w * 1.6, BLACK);
            circ(o, pc - dir * hl, w * 0.8, BLACK);
            circ(o, pc + dir * hl, w * 0.8, BLACK);
            circ(o, ep + vec2(-hr * 0.1, -hr * 0.12), hr * 0.07, WHITE);
        }
    }
    // nose and mouth
    let nc = head + dir * hr * 0.8;
    let apex = nc + dir * hr * 0.15;
    tri(o, nc - side * hr * 0.14, nc + side * hr * 0.14, apex, col(0.97, 0.6, 0.7, 1.0));
    let m1 = apex + dir * hr * 0.08;
    let ink = col(h, 0.7, 0.14, 1.0);
    if chomp > 0.0 {
        let mc = m1 + dir * hr * 0.12;
        circ(o, mc, hr * 0.2 * chomp.min(1.0) + 1.5, col(0.0, 0.7, 0.15, 1.0));
        circ(o, mc + dir * hr * 0.05, hr * 0.1 * chomp.min(1.0), pinkc);
    } else {
        line(o, apex, m1, 1.8, ink);
        for s in [-1.0f32, 1.0] {
            line(o, m1, m1 + side * s * hr * 0.2 + dir * hr * 0.07, 1.8, ink);
        }
    }
    // whiskers
    for s in [-1.0f32, 1.0] {
        let base = head + dir * hr * 0.6 + side * s * hr * 0.3;
        for k in 0..3 {
            let a = (k as f32 - 1.0) * 0.4 * s + (t * 3.0 + k as f32).sin() * 0.05;
            let d = rot(side * s + dir * 0.15, a).normalize();
            line(o, base, base + d * hr * 1.2, 1.5, Color::new(1.0, 1.0, 1.0, 0.85));
        }
    }
}

async fn load(buf: Vec<f32>, gain: f32) -> Sound {
    load_sound_from_bytes(&audio::to_wav(&buf, gain)).await.expect("sound")
}

#[macroquad::main(window_conf)]
async fn main() {
    rand::srand(macroquad::miniquad::date::now() as u64);
    let snd = Sounds {
        eat: load(audio::sfx_eat(), 1.0).await,
        fish: load(audio::sfx_fish(), 1.0).await,
        yarn: load(audio::sfx_yarn(), 1.0).await,
        turn: load(audio::sfx_turn(), 1.0).await,
        die: load(audio::sfx_die(), 1.0).await,
        pop: [
            load(audio::sfx_pop(0), 1.0).await,
            load(audio::sfx_pop(1), 1.0).await,
            load(audio::sfx_pop(2), 1.0).await,
        ],
        level: load(audio::sfx_level(), 1.0).await,
        start: load(audio::sfx_start(), 1.0).await,
        over: load(audio::sfx_over(), 1.0).await,
        music: load(audio::music(), 1.1).await,
    };
    play_sound(&snd.music, PlaySoundParams { looped: true, volume: MUSIC_VOL });

    let high = std::fs::read_to_string("longcat_highscore.txt")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let mut game = Game::new(high);

    loop {
        let dt = get_frame_time().min(0.05);
        game.update(dt, &snd);
        game.draw();
        next_frame().await;
    }
}
