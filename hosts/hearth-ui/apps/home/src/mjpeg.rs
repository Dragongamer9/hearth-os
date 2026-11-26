//! The cameras' moving pictures: Home Assistant's own stream of a camera, /api/camera_proxy_stream/<id> (MJPEG:
//! `multipart/x-mixed-replace`, one JPEG after another, each a part with its own headers). One thread a camera reads
//! it while the window is open and the tile is shown (the subscription ends when it isn't, and the thread with it);
//! at most `FPS` pictures a second are handed to the window, the rest are read and dropped, so a 30 fps camera
//! doesn't make the window draw 30 times a second. When the stream ends or can't start, it waits and tries again
//! (2 s, then up to 30 s), and meanwhile the tile keeps its last picture.

use futures::channel::mpsc;
use futures::Stream;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Pictures a second handed to the window (the board's tiles are small; 8 looks like moving video).
pub const FPS: u32 = 8;

#[derive(Debug, Clone)]
pub struct Cam {
    pub base: String,
    pub token: String,
    pub id: String,
}

// The token isn't part of which stream it is (and isn't hashed into anything kept).
impl Hash for Cam {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.base.hash(h);
        self.id.hash(h);
    }
}

pub struct Frames {
    rx: mpsc::UnboundedReceiver<(String, Vec<u8>)>,
    stop: Arc<AtomicBool>,
}

impl Stream for Frames {
    type Item = (String, Vec<u8>);
    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.rx).poll_next(cx)
    }
}

impl Drop for Frames {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// The boundary of a multipart Content-Type ("multipart/x-mixed-replace;boundary=frame" → "frame").
pub fn boundary(ctype: &str) -> Option<String> {
    let i = ctype.to_ascii_lowercase().find("boundary=")?;
    let b = ctype[i + 9..].split(';').next()?.trim().trim_matches('"').trim_start_matches("--");
    (!b.is_empty()).then(|| b.to_string())
}

/// Reads the parts of a multipart stream: each part's body, given to `each` (false from it stops).
pub fn parts<R: BufRead>(r: &mut R, bnd: &str, mut each: impl FnMut(Vec<u8>) -> bool) -> std::io::Result<()> {
    let mark = format!("--{bnd}");
    let mut line = String::new();
    loop {
        // to the next boundary line
        loop {
            line.clear();
            if r.read_line(&mut line)? == 0 {
                return Ok(());
            }
            if line.trim_end().starts_with(&mark) {
                break;
            }
        }
        // the part's headers
        let mut len: Option<usize> = None;
        loop {
            line.clear();
            if r.read_line(&mut line)? == 0 {
                return Ok(());
            }
            let l = line.trim_end();
            if l.is_empty() {
                break;
            }
            if let Some((k, v)) = l.split_once(':') {
                if k.trim().eq_ignore_ascii_case("content-length") {
                    len = v.trim().parse().ok();
                }
            }
        }
        let body = match len {
            Some(n) if n <= 16 << 20 => {
                let mut b = vec![0u8; n];
                r.read_exact(&mut b)?;
                b
            }
            _ => {
                // no length: a JPEG ends with FF D9
                let mut b = Vec::new();
                let mut one = [0u8; 1];
                while r.read(&mut one)? == 1 {
                    b.push(one[0]);
                    if b.len() >= 2 && b[b.len() - 2] == 0xFF && b[b.len() - 1] == 0xD9 {
                        break;
                    }
                    if b.len() > 16 << 20 {
                        break;
                    }
                }
                b
            }
        };
        if !each(body) {
            return Ok(());
        }
    }
}

fn read_stream(c: &Cam, stop: &AtomicBool, tx: &mpsc::UnboundedSender<(String, Vec<u8>)>) -> Result<(), String> {
    let (host, port, path) = hearth_oss::http::split(&format!("{}/api/camera_proxy_stream/{}", c.base, c.id))?;
    let addr = (host.as_str(), port).to_socket_addrs().map_err(|e| e.to_string())?.next().ok_or("no address")?;
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(6)).map_err(|e| e.to_string())?;
    let _ = s.set_read_timeout(Some(Duration::from_secs(20)));
    if c.token.contains(['\r', '\n']) || c.id.contains(['\r', '\n', ' ']) {
        return Err("a token or name with a line break".into());
    }
    let auth = if c.token.is_empty() { String::new() } else { format!("Authorization: Bearer {}\r\n", c.token) };
    s.write_all(format!("GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nUser-Agent: Hearth\r\nAccept: multipart/x-mixed-replace\r\n{auth}Connection: close\r\n\r\n").as_bytes()).map_err(|e| e.to_string())?;
    let mut r = BufReader::with_capacity(256 * 1024, s);
    let mut line = String::new();
    r.read_line(&mut line).map_err(|e| e.to_string())?;
    let status: u16 = line.split_whitespace().nth(1).and_then(|x| x.parse().ok()).unwrap_or(0);
    let mut ctype = String::new();
    loop {
        line.clear();
        if r.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            break;
        }
        let l = line.trim_end();
        if l.is_empty() {
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-type") {
                ctype = v.trim().to_string();
            }
        }
    }
    if status != 200 {
        return Err(format!("Home Assistant answered {status}"));
    }
    let bnd = boundary(&ctype).ok_or("not a moving picture")?;
    let every = Duration::from_millis(1000 / FPS as u64);
    let mut last: Option<Instant> = None;
    parts(&mut r, &bnd, |b| {
        if stop.load(Ordering::SeqCst) {
            return false;
        }
        if b.len() > 4 && b[0] == 0xFF && b[1] == 0xD8 && last.map(|t| t.elapsed() >= every).unwrap_or(true) {
            last = Some(Instant::now());
            return tx.unbounded_send((c.id.clone(), b)).is_ok();
        }
        true
    })
    .map_err(|e| e.to_string())
}

/// The camera's stream, as pictures (the subscription's).
pub fn frames(c: &Cam) -> Frames {
    let (tx, rx) = mpsc::unbounded();
    let stop = Arc::new(AtomicBool::new(false));
    let (c, st) = (c.clone(), stop.clone());
    let _ = std::thread::Builder::new().name("hearth-home-camera".into()).stack_size(512 * 1024).spawn(move || {
        let mut wait = 2u64;
        while !st.load(Ordering::SeqCst) {
            let t0 = Instant::now();
            let _ = read_stream(&c, &st, &tx);
            if tx.is_closed() {
                return;
            }
            if t0.elapsed() > Duration::from_secs(30) {
                wait = 2;
            }
            for _ in 0..wait * 10 {
                if st.load(Ordering::SeqCst) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            wait = (wait * 2).min(30);
        }
    });
    Frames { rx, stop }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multipart_pictures_are_read() {
        assert_eq!(boundary("multipart/x-mixed-replace;boundary=frame").as_deref(), Some("frame"));
        assert_eq!(boundary("multipart/x-mixed-replace; boundary=\"--abc\"").as_deref(), Some("abc"));
        let mut s = Vec::new();
        s.extend_from_slice(b"--frame\r\nContent-Type: image/jpeg\r\nContent-Length: 4\r\n\r\n\xFF\xD8\xFF\xD9\r\n");
        s.extend_from_slice(b"--frame\r\nContent-Type: image/jpeg\r\n\r\n\xFF\xD8\x01\x02\xFF\xD9\r\n--frame--\r\n");
        let mut got = Vec::new();
        parts(&mut std::io::Cursor::new(s), "frame", |b| {
            got.push(b);
            true
        })
        .unwrap();
        assert_eq!(got, vec![vec![0xFF, 0xD8, 0xFF, 0xD9], vec![0xFF, 0xD8, 1, 2, 0xFF, 0xD9]]);
    }
}
