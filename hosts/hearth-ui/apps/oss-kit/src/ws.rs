//! A small WebSocket reader (RFC 6455, std only) for a service's live news on the person's own PC: ComfyUI's /ws
//! (which step a picture is on), Home Assistant's /api/websocket. Text messages are handed over one by one on a
//! channel; it reconnects by itself until it's dropped. http:// (ws://) only, as http.rs.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc;
use std::time::Duration;

fn key() -> String {
    let mut b = [0u8; 16];
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(7);
    for (i, x) in b.iter_mut().enumerate() {
        *x = ((t >> ((i % 8) * 8)) as u8).wrapping_add(i as u8 * 31);
    }
    crate::b64::encode(&b)
}

/// One connection: its socket after the handshake.
pub struct Conn {
    s: TcpStream,
    buf: Vec<u8>,
}

impl Conn {
    /// `url` is http://host:port/path?query.
    pub fn open(url: &str, headers: &[(String, String)]) -> Result<Conn, String> {
        let (host, port, path) = crate::http::split(url)?;
        let addr = (host.as_str(), port).to_socket_addrs().map_err(|e| e.to_string())?.next().ok_or("no address")?;
        let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(5)).map_err(|e| format!("{host}:{port}: {e}"))?;
        let mut req = format!("GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {}\r\nSec-WebSocket-Version: 13\r\n", key());
        for (k, v) in headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str("\r\n");
        s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
        let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
        let mut head = Vec::new();
        let mut one = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            if s.read(&mut one).map_err(|e| e.to_string())? == 0 {
                return Err("closed during the handshake".into());
            }
            head.push(one[0]);
            if head.len() > 8192 {
                return Err("not a WebSocket".into());
            }
        }
        if !String::from_utf8_lossy(&head).starts_with("HTTP/1.1 101") {
            return Err("the service didn't open a WebSocket".into());
        }
        let _ = s.set_read_timeout(None);
        Ok(Conn { s, buf: Vec::new() })
    }

    /// Sends a text message (masked, as a client must).
    pub fn send(&mut self, text: &str) -> Result<(), String> {
        let p = text.as_bytes();
        let mut f = vec![0x81u8];
        if p.len() < 126 {
            f.push(0x80 | p.len() as u8);
        } else if p.len() < 65536 {
            f.push(0x80 | 126);
            f.extend_from_slice(&(p.len() as u16).to_be_bytes());
        } else {
            f.push(0x80 | 127);
            f.extend_from_slice(&(p.len() as u64).to_be_bytes());
        }
        let m: [u8; 4] = [0x37, 0xa1, 0x5c, 0x92];
        f.extend_from_slice(&m);
        f.extend(p.iter().enumerate().map(|(i, b)| b ^ m[i % 4]));
        self.s.write_all(&f).map_err(|e| e.to_string())
    }

    fn fill(&mut self, n: usize) -> Result<(), String> {
        let mut chunk = [0u8; 8192];
        while self.buf.len() < n {
            let k = self.s.read(&mut chunk).map_err(|e| e.to_string())?;
            if k == 0 {
                return Err("closed".into());
            }
            self.buf.extend_from_slice(&chunk[..k]);
        }
        Ok(())
    }

    /// The next text message (binary ones, pings and pongs are passed over; a ping is answered).
    pub fn next_text(&mut self) -> Result<String, String> {
        let mut msg: Vec<u8> = Vec::new();
        loop {
            self.fill(2)?;
            let (b0, b1) = (self.buf[0], self.buf[1]);
            let op = b0 & 0x0f;
            let fin = b0 & 0x80 != 0;
            let mut len = (b1 & 0x7f) as usize;
            let mut at = 2;
            if len == 126 {
                self.fill(4)?;
                len = u16::from_be_bytes([self.buf[2], self.buf[3]]) as usize;
                at = 4;
            } else if len == 127 {
                self.fill(10)?;
                let mut x = [0u8; 8];
                x.copy_from_slice(&self.buf[2..10]);
                len = u64::from_be_bytes(x) as usize;
                at = 10;
            }
            if len > 64 << 20 {
                return Err("a message too big".into());
            }
            self.fill(at + len)?;
            let payload: Vec<u8> = self.buf[at..at + len].to_vec();
            self.buf.drain(..at + len);
            match op {
                0x8 => return Err("closed".into()),
                0x9 => {
                    let mut f = vec![0x8au8, 0x80 | payload.len().min(125) as u8, 0, 0, 0, 0];
                    f.extend_from_slice(&payload[..payload.len().min(125)]);
                    let _ = self.s.write_all(&f);
                }
                0x1 | 0x0 => {
                    msg.extend_from_slice(&payload);
                    if fin {
                        return Ok(String::from_utf8_lossy(&msg).to_string());
                    }
                }
                _ => {}
            }
        }
    }
}

/// Listens on its own thread: every text message goes into the channel; after a drop it tries again every 5 s,
/// `hello` sent first each time. It ends when the receiver is dropped.
pub fn listen(url: String, headers: Vec<(String, String)>, hello: Vec<String>) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new().name("hearth-oss-ws".into()).spawn(move || loop {
        if let Ok(mut c) = Conn::open(&url, &headers) {
            let _ = tx.send("{\"hearth\":\"connected\"}".to_string());
            for h in &hello {
                let _ = c.send(h);
            }
            while let Ok(m) = c.next_text() {
                if tx.send(m).is_err() {
                    return;
                }
            }
        }
        if tx.send("{\"hearth\":\"closed\"}".to_string()).is_err() {
            return;
        }
        std::thread::sleep(Duration::from_secs(5));
    });
    rx
}
