//! "Show the game's own screen when it has one" (Keyboard screen, g15-3): programs made for the G15's screen on Linux
//! draw through g15daemon's socket (lcdproc's g15 driver, g15stats, games and tools built with libg15daemon_client).
//! hearth-g15 answers on the same place, 127.0.0.1:15550, while the switch is on (gameScreens, on unless turned off),
//! so their screen shows on the keyboard while they send it; Hearth's pages come back when they stop or close.
//!
//! The protocol (libg15daemon_client): the server says "G15 daemon HELLO" (16 bytes), the program answers with the
//! kind of screen it draws, then sends frames of that kind:
//!   GBUF  6880 bytes, one per dot (row by row, not 0: dark)
//!   RBUF  1048 bytes, libg15render's canvas: 20 bytes a row, the first dot in the highest bit
//!   WBUF  865 bytes, a WBMP: 5 bytes of header, then rows as in RBUF
//! (TBUF, text, isn't supported: no program in use needs it.) Only programs on this PC can connect.

use crate::lcd::{Frame, H, W};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::{Duration, Instant};

pub const PORT: u16 = 15550;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Pixels,
    Raw,
    Wbmp,
}

impl Kind {
    fn of(tag: &[u8]) -> Option<Kind> {
        match tag {
            b"GBUF" => Some(Kind::Pixels),
            b"RBUF" => Some(Kind::Raw),
            b"WBUF" => Some(Kind::Wbmp),
            _ => None,
        }
    }
    pub fn len(self) -> usize {
        match self {
            Kind::Pixels => (W * H) as usize,
            Kind::Raw => 1048,
            Kind::Wbmp => 865,
        }
    }
}

/// A frame in one of the kinds, as the keyboard's dots.
pub fn decode(kind: Kind, b: &[u8]) -> Frame {
    let mut f = Frame::new();
    match kind {
        Kind::Pixels => {
            for y in 0..H {
                for x in 0..W {
                    if b.get((y * W + x) as usize).copied().unwrap_or(0) != 0 {
                        f.dot(x, y, true);
                    }
                }
            }
        }
        Kind::Raw | Kind::Wbmp => {
            let off = if kind == Kind::Wbmp { 5 } else { 0 };
            for y in 0..H {
                for x in 0..W {
                    let i = off + (y * 20 + x / 8) as usize;
                    if b.get(i).map(|v| v >> (7 - (x % 8)) & 1 == 1).unwrap_or(false) {
                        f.dot(x, y, true);
                    }
                }
            }
        }
    }
    f
}

struct Client {
    s: TcpStream,
    kind: Option<Kind>,
    buf: Vec<u8>,
}

pub struct Server {
    l: TcpListener,
    clients: Vec<Client>,
    /// The newest frame a program sent, and when.
    pub frame: Option<(Frame, Instant)>,
}

impl Server {
    /// Listening on 127.0.0.1:15550 (None when something else already does: a real g15daemon).
    pub fn start() -> Option<Server> {
        let l = TcpListener::bind(("127.0.0.1", PORT)).ok()?;
        l.set_nonblocking(true).ok()?;
        Some(Server { l, clients: Vec::new(), frame: None })
    }
    pub fn fds(&self) -> Vec<RawFd> {
        let mut v = vec![self.l.as_raw_fd()];
        v.extend(self.clients.iter().map(|c| c.s.as_raw_fd()));
        v
    }
    /// A program's screen is showing: it sent a frame in the last 5 seconds and is still connected.
    pub fn showing(&self) -> Option<&Frame> {
        match &self.frame {
            Some((f, at)) if !self.clients.is_empty() && at.elapsed() < Duration::from_secs(5) => Some(f),
            _ => None,
        }
    }
    /// Takes new programs and their frames (non-blocking). True when the screen should be drawn again.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok((mut s, addr)) = self.l.accept() {
            if !addr.ip().is_loopback() || self.clients.len() >= 4 {
                continue;
            }
            let _ = s.set_nonblocking(true);
            let _ = s.write_all(b"G15 daemon HELLO");
            self.clients.push(Client { s, kind: None, buf: Vec::new() });
        }
        let mut gone = Vec::new();
        for (i, c) in self.clients.iter_mut().enumerate() {
            let mut tmp = [0u8; 8192];
            loop {
                match c.s.read(&mut tmp) {
                    Ok(0) => {
                        gone.push(i);
                        break;
                    }
                    Ok(n) => {
                        c.buf.extend_from_slice(&tmp[..n]);
                        if c.buf.len() > 1 << 20 {
                            gone.push(i);
                            break;
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(_) => {
                        gone.push(i);
                        break;
                    }
                }
            }
            if c.kind.is_none() && c.buf.len() >= 4 {
                match Kind::of(&c.buf[..4]) {
                    Some(k) => {
                        c.kind = Some(k);
                        c.buf.drain(..4);
                    }
                    None => {
                        gone.push(i);
                        continue;
                    }
                }
            }
            if let Some(k) = c.kind {
                let n = k.len();
                // Only the newest whole frame matters.
                let whole = c.buf.len() / n;
                if whole > 0 {
                    let start = (whole - 1) * n;
                    let f = decode(k, &c.buf[start..start + n]);
                    c.buf.drain(..whole * n);
                    self.frame = Some((f, Instant::now()));
                    changed = true;
                }
            }
        }
        gone.sort_unstable();
        gone.dedup();
        for i in gone.into_iter().rev() {
            self.clients.remove(i);
            changed = true;
        }
        if self.clients.is_empty() && self.frame.is_some() {
            self.frame = None;
            changed = true;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_each_kind() {
        let mut g = vec![0u8; 6880];
        g[0] = 1;
        g[160 * 42 + 159] = 1;
        let f = decode(Kind::Pixels, &g);
        assert!(f.get(0, 0) && f.get(159, 42) && f.lit() == 2);
        let mut r = vec![0u8; 1048];
        r[0] = 0x80;
        r[20 + 1] = 0x01;
        let f = decode(Kind::Raw, &r);
        assert!(f.get(0, 0) && f.get(15, 1) && f.lit() == 2);
        assert_eq!(Kind::of(b"WBUF"), Some(Kind::Wbmp));
        assert_eq!(Kind::of(b"TBUF"), None);
    }
}
