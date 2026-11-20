//! Mantel's motion, as its QML Behaviors have it (Theme.qml quick 140 ms, calm 260 ms; Qt's easing curves), for the
//! Hearth menu's open / close (RadialMenu.qml) and the keyboard's ember (OskBoard.qml: the glide, its sparks and the
//! pulsing warmth). Frames are asked for only while something moves (main.rs frame_every); the clock can be held
//! still at one moment for CI's pictures (--at MS).

use std::time::Instant;

pub const QUICK: f32 = 140.0;
pub const CALM: f32 = 260.0;
/// RadialMenu.qml: the menu stays on screen this long after it closes, while it fades (Theme.calm + 60).
pub const CLOSING: f32 = CALM + 60.0;
/// OskBoard.qml: the ember's glide (x, y OutCubic; width linear).
pub const GLIDE: f32 = 130.0;
/// A spark lives this long (fades, rises 14 px, shrinks to 0.3).
pub const SPARK: f32 = 450.0;
/// One spark at most this often.
pub const SPARK_GAP: f32 = 70.0;

fn c01(t: f32) -> f32 {
    if t.is_nan() {
        1.0
    } else {
        t.clamp(0.0, 1.0)
    }
}
pub fn out_cubic(t: f32) -> f32 {
    let t = c01(t);
    1.0 - (1.0 - t).powi(3)
}
/// Qt's Easing.OutBack (overshoot 1.70158): a little past the end, then back.
pub fn out_back(t: f32) -> f32 {
    let t = c01(t);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}
pub fn in_out_sine(t: f32) -> f32 {
    let t = c01(t);
    -((std::f32::consts::PI * t).cos() - 1.0) / 2.0
}
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The clock the overlays move by: the real one, or one moment held still (--at MS: every animation is drawn as it
/// is that many milliseconds after it started, and nothing asks for frames).
#[derive(Debug, Clone, Copy)]
pub struct Clock {
    pub now: Instant,
    pub frozen: Option<f32>,
}

impl Clock {
    pub fn new(frozen: Option<f32>) -> Clock {
        Clock { now: Instant::now(), frozen }
    }
    /// Milliseconds since `t0` (or the held moment).
    pub fn ms(&self, t0: Instant) -> f32 {
        self.frozen.unwrap_or_else(|| self.now.saturating_duration_since(t0).as_secs_f32() * 1000.0)
    }
}

/// The Hearth menu's motion (RadialMenu.qml): the ring's scale (0.85 → 1, OutBack, calm) and opacity (quick), the
/// dimmed screen (0 → 0.55, OutCubic, calm); closing runs them back from wherever they were.
#[derive(Debug, Clone, Copy)]
pub struct Radial {
    pub opened: Instant,
    pub closing: Option<Instant>,
    /// the item that was lit before, fading out while the new one lights up (scale 1 → 1.14 and colour, quick)
    pub lit_prev: i32,
    pub lit_at: Instant,
}

/// (ring scale, ring opacity, dim opacity)
pub fn radial_look(r: &Radial, c: &Clock) -> (f32, f32, f32) {
    let open_at = |e: f32| (0.85 + 0.15 * out_back(e / CALM), c01(e / QUICK), 0.55 * out_cubic(e / CALM));
    match r.closing {
        None => open_at(c.ms(r.opened)),
        Some(t) => {
            // Where it was when it started closing (real time: the held clock is for the close itself).
            let (s0, o0, d0) = open_at(t.saturating_duration_since(r.opened).as_secs_f32() * 1000.0);
            let e = c.ms(t);
            (lerp(s0, 0.85, out_back(e / CALM)), o0 * (1.0 - c01(e / QUICK)), d0 * (1.0 - out_cubic(e / CALM)))
        }
    }
}

/// How lit item `i` is (0..1) while the light moves from one item to another.
pub fn lit_amount(r: &Radial, sel: i32, i: i32, c: &Clock) -> f32 {
    let e = out_cubic(c.ms(r.lit_at) / QUICK);
    if i == sel {
        e
    } else if i == r.lit_prev {
        1.0 - e
    } else {
        0.0
    }
}

pub fn radial_moving(r: &Radial, c: &Clock) -> bool {
    match r.closing {
        Some(t) => c.ms(t) < CLOSING,
        None => c.ms(r.opened) < CALM || c.ms(r.lit_at) < QUICK,
    }
}

/// A key's place on the board: x, y, width, height.
pub type Rect4 = (f32, f32, f32, f32);

/// One move of the ember from key to key.
#[derive(Debug, Clone, Copy)]
pub struct Glide {
    pub from: Rect4,
    pub to: Rect4,
    pub at: Instant,
    /// the trail setting (osk.json trail): off, the ember jumps (only its width still eases) and leaves no sparks
    pub trail: bool,
}

pub fn glide_at(g: &Glide, e: f32) -> Rect4 {
    let p = if g.trail { out_cubic(e / GLIDE) } else { 1.0 };
    let w = lerp(g.from.2, g.to.2, c01(e / GLIDE));
    (lerp(g.from.0, g.to.0, p), lerp(g.from.1, g.to.1, p), w, g.to.3)
}

/// The ember now: the newest glide, where it has got to.
pub fn ember_rect(glides: &[Glide], c: &Clock) -> Option<Rect4> {
    let g = glides.last()?;
    Some(glide_at(g, c.ms(g.at)))
}

/// The sparks alive now: (centre x, centre y, scale, opacity). OskBoard.qml drops one at the ember's centre each time
/// its x moves (at most every 70 ms), which rises 14 px, shrinks to 0.3 and fades over 450 ms.
pub fn sparks(glides: &[Glide], c: &Clock) -> Vec<(f32, f32, f32, f32)> {
    let mut out = Vec::new();
    for g in glides {
        if !g.trail || (g.from.0 - g.to.0).abs() < 0.5 {
            continue;
        }
        let e = c.ms(g.at);
        let mut born = 16.0; // the first frame of the glide
        while born < GLIDE {
            let age = e - born;
            if (0.0..SPARK).contains(&age) {
                let r = glide_at(g, born);
                let k = age / SPARK;
                out.push((r.0 + r.2 / 2.0, r.1 + r.3 / 2.0 - 14.0 * k, 1.0 - 0.7 * k, 0.9 * (1.0 - k)));
            }
            born += SPARK_GAP;
        }
    }
    out
}

pub fn ember_moving(glides: &[Glide], c: &Clock) -> bool {
    glides.last().map(|g| c.ms(g.at) < GLIDE + SPARK).unwrap_or(false)
}

/// The warmth under the ember (OskBoard.qml): from 0.22 up to 0.32 (1.1 s), then down to 0.18 (1.3 s) and up again,
/// for as long as the ember shows.
pub fn warmth(since: Instant, c: &Clock) -> f32 {
    let e = c.ms(since);
    if e < 1100.0 {
        return lerp(0.22, 0.32, in_out_sine(e / 1100.0));
    }
    let k = (e - 1100.0) % 2400.0;
    if k < 1300.0 {
        lerp(0.32, 0.18, in_out_sine(k / 1300.0))
    } else {
        lerp(0.18, 0.32, in_out_sine((k - 1300.0) / 1100.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn menu_opens_from_small_and_faint() {
        let t0 = Instant::now();
        let r = Radial { opened: t0, closing: None, lit_prev: -1, lit_at: t0 };
        let (s, o, d) = radial_look(&r, &Clock { now: t0, frozen: Some(0.0) });
        assert!((s - 0.85).abs() < 1e-3 && o.abs() < 1e-3 && d.abs() < 1e-3);
        let (s, o, d) = radial_look(&r, &Clock { now: t0, frozen: Some(400.0) });
        assert!((s - 1.0).abs() < 1e-3 && (o - 1.0).abs() < 1e-3 && (d - 0.55).abs() < 1e-3);
        // OutBack: it goes a little past full size on the way
        let (s, _, _) = radial_look(&r, &Clock { now: t0, frozen: Some(180.0) });
        assert!(s > 1.0);
    }

    #[test]
    fn menu_closes_back_to_nothing() {
        let t0 = Instant::now();
        let r = Radial { opened: t0, closing: Some(t0 + Duration::from_secs(2)), lit_prev: -1, lit_at: t0 };
        let (_, o, d) = radial_look(&r, &Clock { now: t0, frozen: Some(CLOSING) });
        assert!(o.abs() < 1e-3 && d.abs() < 1e-3);
    }

    #[test]
    fn ember_glides_and_sparks() {
        let t0 = Instant::now();
        let g = Glide { from: (0.0, 0.0, 50.0, 40.0), to: (100.0, 0.0, 70.0, 40.0), at: t0, trail: true };
        let r = ember_rect(&[g], &Clock { now: t0, frozen: Some(65.0) }).unwrap();
        assert!(r.0 > 50.0 && r.0 < 100.0);
        assert!(!sparks(&[g], &Clock { now: t0, frozen: Some(100.0) }).is_empty());
        assert!(sparks(&[g], &Clock { now: t0, frozen: Some(1000.0) }).is_empty());
        let off = Glide { trail: false, ..g };
        assert_eq!(ember_rect(&[off], &Clock { now: t0, frozen: Some(10.0) }).unwrap().0, 100.0);
        assert!(sparks(&[off], &Clock { now: t0, frozen: Some(100.0) }).is_empty());
    }

    #[test]
    fn warmth_pulses_between_its_limits() {
        let t0 = Instant::now();
        for ms in (0..10_000).step_by(37) {
            let w = warmth(t0, &Clock { now: t0, frozen: Some(ms as f32) });
            assert!((0.179..=0.321).contains(&w), "{ms}: {w}");
        }
    }
}
