//! What goes over a QUIC stream between two Hearth devices.
//!
//! Messages: a 4-byte big-endian length, then that much JSON (at most 64 MiB). A stream starts with the request
//! ({"op": ...}); the answer is a message too, except for "chunks", whose answer is a run of chunk frames:
//!
//!   0      type      1 = chunk, 2 = done (end of the answer), 3 = error (the payload is the message)
//!   1      flags     bit 0: the payload is zstd-compressed
//!   2..4   0
//!   4..8   item      which item of the request (u32, big-endian)
//!   8..12  chunk     the chunk's number in its file
//!   12..16 raw       the chunk's length (after decompressing)
//!   16..20 wire      the payload's length (as sent)
//!   20..52 hash      BLAKE3 of the chunk's data: checked before it's written
//!   52..56 0
//!   then `wire` bytes.

use serde_json::Value;

pub const MAX_MESSAGE: usize = 64 << 20;
pub const HEADER: usize = 56;
/// A chunk: what's hashed, resumed, skipped and compressed as one.
pub const CHUNK: u64 = 1 << 20;

pub const T_CHUNK: u8 = 1;
pub const T_DONE: u8 = 2;
pub const T_ERROR: u8 = 3;
pub const F_ZSTD: u8 = 1;

pub async fn write_msg(s: &mut iroh::endpoint::SendStream, v: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(v).map_err(|e| e.to_string())?;
    if body.len() > MAX_MESSAGE { return Err("message too big".into()); }
    let mut buf = Vec::with_capacity(4 + body.len());
    buf.extend_from_slice(&(body.len() as u32).to_be_bytes());
    buf.extend_from_slice(&body);
    s.write_all(&buf).await.map_err(|e| e.to_string())
}

pub async fn read_msg(r: &mut iroh::endpoint::RecvStream) -> Result<Value, String> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len).await.map_err(|e| format!("reading: {e}"))?;
    let n = u32::from_be_bytes(len) as usize;
    if n > MAX_MESSAGE { return Err("message too big".into()); }
    let mut body = vec![0u8; n];
    r.read_exact(&mut body).await.map_err(|e| format!("reading: {e}"))?;
    serde_json::from_slice(&body).map_err(|e| format!("not a message: {e}"))
}

/// One request and its answer on a new stream of `conn`.
/// Errors of the connection itself (not an answer from the other device) start with "connection: ".
pub async fn request(conn: &iroh::endpoint::Connection, req: &Value) -> Result<Value, String> {
    let (mut s, mut r) = conn.open_bi().await.map_err(|e| format!("connection: {e}"))?;
    write_msg(&mut s, req).await.map_err(|e| format!("connection: {e}"))?;
    let _ = s.finish();
    let v = read_msg(&mut r).await.map_err(|e| format!("connection: {e}"))?;
    if let Some(e) = v.get("error").and_then(Value::as_str) { return Err(e.to_string()); }
    Ok(v)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Header { pub kind: u8, pub flags: u8, pub item: u32, pub chunk: u32, pub raw: u32, pub wire: u32, pub hash: [u8; 32] }

impl Header {
    pub fn encode(&self) -> [u8; HEADER] {
        let mut b = [0u8; HEADER];
        b[0] = self.kind;
        b[1] = self.flags;
        b[4..8].copy_from_slice(&self.item.to_be_bytes());
        b[8..12].copy_from_slice(&self.chunk.to_be_bytes());
        b[12..16].copy_from_slice(&self.raw.to_be_bytes());
        b[16..20].copy_from_slice(&self.wire.to_be_bytes());
        b[20..52].copy_from_slice(&self.hash);
        b
    }
    pub fn decode(b: &[u8; HEADER]) -> Result<Header, String> {
        let u = |i: usize| u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        let h = Header { kind: b[0], flags: b[1], item: u(4), chunk: u(8), raw: u(12), wire: u(16), hash: b[20..52].try_into().unwrap() };
        if !(T_CHUNK..=T_ERROR).contains(&h.kind) { return Err("not a chunk frame".into()); }
        // A chunk is never bigger than CHUNK, compressed or not (compressed is only sent when smaller).
        if h.raw as u64 > CHUNK || h.wire as u64 > CHUNK + 4096 { return Err("chunk too big".into()); }
        Ok(h)
    }
}

/// zstd for a chunk, only when it helps: a 64 KiB sample is tried first (already-compressed data, which is most
/// of a game, costs nothing more than that), then the whole chunk, kept only if it's 10% smaller or more.
pub fn maybe_compress(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 8192 { return None; }
    let sample = &data[..data.len().min(64 << 10)];
    let s = zstd::bulk::compress(sample, 1).ok()?;
    if s.len() * 100 > sample.len() * 85 { return None; }
    let c = zstd::bulk::compress(data, 1).ok()?;
    if c.len() * 100 > data.len() * 90 { return None; }
    Some(c)
}

pub fn decompress(wire: &[u8], raw: usize) -> Result<Vec<u8>, String> {
    let v = zstd::bulk::decompress(wire, raw).map_err(|e| format!("decompressing: {e}"))?;
    if v.len() != raw { return Err("a chunk decompressed to the wrong size".into()); }
    Ok(v)
}

/// A whole frame (header and payload) for a chunk of `data`.
pub fn chunk_frame(item: u32, chunk: u32, data: &[u8], compress: bool) -> Vec<u8> {
    let hash = *blake3::hash(data).as_bytes();
    let packed = if compress { maybe_compress(data) } else { None };
    let (flags, payload): (u8, &[u8]) = match &packed { Some(c) => (F_ZSTD, c.as_slice()), None => (0, data) };
    let h = Header { kind: T_CHUNK, flags, item, chunk, raw: data.len() as u32, wire: payload.len() as u32, hash };
    let mut out = Vec::with_capacity(HEADER + payload.len());
    out.extend_from_slice(&h.encode());
    out.extend_from_slice(payload);
    out
}

pub fn control_frame(kind: u8, msg: &str) -> Vec<u8> {
    let m = msg.as_bytes();
    let m = &m[..m.len().min(4000)];
    let h = Header { kind, flags: 0, item: 0, chunk: 0, raw: 0, wire: m.len() as u32, hash: [0; 32] };
    let mut out = h.encode().to_vec();
    out.extend_from_slice(m);
    out
}

/// A received chunk's data, decompressed and checked against its hash.
pub fn open_chunk(h: &Header, payload: Vec<u8>) -> Result<Vec<u8>, String> {
    let data = if h.flags & F_ZSTD != 0 { decompress(&payload, h.raw as usize)? } else { payload };
    if data.len() != h.raw as usize { return Err("a chunk arrived the wrong size".into()); }
    if blake3::hash(&data).as_bytes() != &h.hash { return Err("a chunk didn't match its hash".into()); }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn headers_both_ways() {
        let h = Header { kind: T_CHUNK, flags: F_ZSTD, item: 7, chunk: 123456, raw: 1 << 20, wire: 4000, hash: [9; 32] };
        assert_eq!(Header::decode(&h.encode()).unwrap(), h);
        let mut bad = h.encode(); bad[0] = 9;
        assert!(Header::decode(&bad).is_err());
        let big = Header { raw: (1 << 20) + 1, ..h };
        assert!(Header::decode(&big.encode()).is_err());
    }
    #[test]
    fn compresses_only_what_it_helps() {
        let text = b"Hearth Link copies games fast. ".repeat(40000);
        let text = &text[..1 << 20];
        let c = maybe_compress(text).expect("text compresses");
        assert!(c.len() < text.len() / 10);
        assert_eq!(decompress(&c, text.len()).unwrap(), text);
        // Random bytes (like most of a game: already compressed) are sent as they are.
        let mut rnd = vec![0u8; 1 << 20];
        let mut x: u64 = 0x9e3779b97f4a7c15;
        for b in rnd.iter_mut() { x ^= x << 13; x ^= x >> 7; x ^= x << 17; *b = x as u8; }
        assert!(maybe_compress(&rnd).is_none());
        assert!(maybe_compress(b"short").is_none());
    }
    #[test]
    fn frames_are_checked_when_they_arrive() {
        let data = b"abc".repeat(100000);
        let f = chunk_frame(1, 2, &data, true);
        let h = Header::decode(f[..HEADER].try_into().unwrap()).unwrap();
        assert_eq!(h.flags & F_ZSTD, F_ZSTD);
        assert_eq!(open_chunk(&h, f[HEADER..].to_vec()).unwrap(), data);
        // Changed on the way: refused.
        let mut f2 = chunk_frame(1, 2, &data, false);
        let h2 = Header::decode(f2[..HEADER].try_into().unwrap()).unwrap();
        f2[HEADER + 5] ^= 1;
        assert!(open_chunk(&h2, f2[HEADER..].to_vec()).is_err());
    }
}
