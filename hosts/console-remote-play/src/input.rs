//! The controller, as the Xbox reads it on the stream's `input` channel: xbox-xcloud-player's packets (MIT), byte for
//! byte. A 14-byte header (report type u16, sequence u32, time f64, little-endian), then per kind its frames; a
//! gamepad frame is 23 bytes (index, buttons u16, four sticks i16, two triggers u16, two u32 zeros).

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pad {
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub lb: bool,
    pub rb: bool,
    pub view: bool,
    pub menu: bool,
    pub nexus: bool,
    pub ls: bool,
    pub rs: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    /// -1..1, up is negative (as evdev and the Gamepad API give it).
    pub lx: f32,
    pub ly: f32,
    pub rx: f32,
    pub ry: f32,
    /// 0..1.
    pub lt: f32,
    pub rt: f32,
}

pub const METADATA: u16 = 1;
pub const GAMEPAD: u16 = 2;
pub const CLIENT_METADATA: u16 = 8;
pub const VIBRATION: u8 = 128;

impl Pad {
    pub fn buttons(&self) -> u16 {
        let mut m = 0u16;
        let set = |m: &mut u16, on: bool, bit: u16| {
            if on {
                *m |= bit
            }
        };
        set(&mut m, self.nexus, 2);
        set(&mut m, self.menu, 4);
        set(&mut m, self.view, 8);
        set(&mut m, self.a, 16);
        set(&mut m, self.b, 32);
        set(&mut m, self.x, 64);
        set(&mut m, self.y, 128);
        set(&mut m, self.up, 256);
        set(&mut m, self.down, 512);
        set(&mut m, self.left, 1024);
        set(&mut m, self.right, 2048);
        set(&mut m, self.lb, 4096);
        set(&mut m, self.rb, 8192);
        set(&mut m, self.ls, 16384);
        set(&mut m, self.rs, 32768);
        m
    }
}

fn axis(v: f32) -> i16 {
    (v.clamp(-1.0, 1.0) * 32767.0) as i16
}

fn trigger(v: f32) -> u16 {
    (v.clamp(0.0, 1.0) * 65535.0) as u16
}

fn header(kind: u16, seq: u32, ms: f64, size: usize) -> Vec<u8> {
    let mut b = Vec::with_capacity(size);
    b.extend_from_slice(&kind.to_le_bytes());
    b.extend_from_slice(&seq.to_le_bytes());
    b.extend_from_slice(&ms.to_le_bytes());
    b
}

/// The first packet: client metadata (how many touch points), 15 bytes.
pub fn client_metadata(seq: u32, ms: f64, touch_points: u8) -> Vec<u8> {
    let mut b = header(CLIENT_METADATA, seq, ms, 15);
    b.push(touch_points);
    b
}

/// One gamepad frame (or several) in a packet.
pub fn gamepad(seq: u32, ms: f64, pads: &[(u8, Pad)]) -> Vec<u8> {
    let mut b = header(GAMEPAD, seq, ms, 14 + 1 + 23 * pads.len());
    b.push(pads.len() as u8);
    for (i, p) in pads {
        b.push(*i);
        b.extend_from_slice(&p.buttons().to_le_bytes());
        b.extend_from_slice(&axis(p.lx).to_le_bytes());
        // The Xbox's Y axes point up (xbox-xcloud-player sends -Y).
        b.extend_from_slice(&axis(-p.ly).to_le_bytes());
        b.extend_from_slice(&axis(p.rx).to_le_bytes());
        b.extend_from_slice(&axis(-p.ry).to_le_bytes());
        b.extend_from_slice(&trigger(p.lt).to_le_bytes());
        b.extend_from_slice(&trigger(p.rt).to_le_bytes());
        b.extend_from_slice(&[0u8; 8]);
    }
    b
}

/// A rumble the game asked for (on the input channel).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rumble {
    pub pad: u8,
    /// 0..1: the big (left) motor and the small (right) one.
    pub strong: f32,
    pub weak: f32,
    pub ms: u16,
    pub repeat: u8,
}

pub fn rumble(b: &[u8]) -> Option<Rumble> {
    if b.len() < 13 || b[0] != VIBRATION {
        return None;
    }
    let i = 4;
    Some(Rumble {
        pad: b[3],
        strong: b[i] as f32 / 100.0,
        weak: b[i + 1] as f32 / 100.0,
        ms: u16::from_le_bytes([b[i + 4], b[i + 5]]),
        repeat: b[i + 8],
    })
}

/// The quick menu's combo: View and Menu held together for a second (the owner's choice, Nov 20).
#[derive(Debug, Default)]
pub struct Combo {
    since: Option<u64>,
    fired: bool,
}

pub const HOLD_MS: u64 = 1000;

impl Combo {
    /// Fed every change (and the reader's one-second check while both are down): true once, when it's been held 1 s.
    pub fn feed(&mut self, p: &Pad, now_ms: u64) -> bool {
        if p.view && p.menu {
            let s = *self.since.get_or_insert(now_ms);
            if !self.fired && now_ms.saturating_sub(s) >= HOLD_MS {
                self.fired = true;
                return true;
            }
        } else {
            self.since = None;
            self.fired = false;
        }
        false
    }
    pub fn holding(&self) -> bool {
        self.since.is_some() && !self.fired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gamepad_packet_is_xcloud_players() {
        let p = Pad { a: true, menu: true, rb: true, lx: 1.0, ly: -1.0, rt: 1.0, ..Pad::default() };
        let b = gamepad(7, 1.5, &[(0, p)]);
        assert_eq!(b.len(), 14 + 1 + 23);
        assert_eq!(u16::from_le_bytes([b[0], b[1]]), GAMEPAD);
        assert_eq!(u32::from_le_bytes([b[2], b[3], b[4], b[5]]), 7);
        assert_eq!(f64::from_le_bytes(b[6..14].try_into().unwrap()), 1.5);
        assert_eq!(b[14], 1);
        assert_eq!(b[15], 0);
        assert_eq!(u16::from_le_bytes([b[16], b[17]]), 16 | 4 | 8192);
        assert_eq!(i16::from_le_bytes([b[18], b[19]]), 32767);
        assert_eq!(i16::from_le_bytes([b[20], b[21]]), 32767); // stick up → +Y for the Xbox
        assert_eq!(u16::from_le_bytes([b[28], b[29]]), 65535);
        assert_eq!(client_metadata(0, 0.0, 2).len(), 15);
    }

    #[test]
    fn rumble_and_the_combo() {
        let mut v = vec![128, 0, 0, 0, 50, 25, 0, 0, 200, 0, 0, 0, 1];
        let r = rumble(&v).unwrap();
        assert_eq!((r.strong, r.weak, r.ms, r.repeat), (0.5, 0.25, 200, 1));
        v[0] = 2;
        assert!(rumble(&v).is_none());
        let mut c = Combo::default();
        let both = Pad { view: true, menu: true, ..Pad::default() };
        assert!(!c.feed(&both, 100));
        assert!(c.holding());
        assert!(!c.feed(&both, 900));
        assert!(c.feed(&both, 1100));
        assert!(!c.feed(&both, 2500));
        assert!(!c.feed(&Pad::default(), 2600));
        assert!(!c.feed(&Pad { view: true, ..Pad::default() }, 2700));
    }
}
