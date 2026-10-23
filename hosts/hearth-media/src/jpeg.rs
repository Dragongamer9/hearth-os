//! JPEG frames out of a byte stream (ffmpeg's image2pipe, GStreamer's jpegenc, wf-recorder's mjpeg): one after
//! another with nothing between, so each frame is found by walking its markers to the end (EOI). Walking them,
//! rather than looking for the bytes FF D9, can't be fooled by a table that happens to hold them.

#[derive(Debug, PartialEq)]
enum End { Complete(usize), Incomplete, Corrupt }

/// Where the frame that starts at b[0] (FF D8) ends.
fn frame_end(b: &[u8]) -> End {
    let n = b.len();
    let mut i = 2;
    loop {
        if i + 1 >= n { return End::Incomplete; }
        if b[i] != 0xFF { return End::Corrupt; }
        let m = b[i + 1];
        if m == 0xFF { i += 1; continue; }
        if m == 0xD9 { return End::Complete(i + 2); }
        if m == 0x01 || (0xD0..=0xD7).contains(&m) { i += 2; continue; }
        if m == 0xD8 || m == 0x00 { return End::Corrupt; }
        if i + 3 >= n { return End::Incomplete; }
        let len = ((b[i + 2] as usize) << 8) | b[i + 3] as usize;
        if len < 2 { return End::Corrupt; }
        let seg_end = i + 2 + len;
        if m == 0xDA {
            // Start of scan: the compressed data runs to the next marker (FF then anything but 00 and RST0-7).
            let mut j = seg_end;
            loop {
                if j + 1 >= n { return End::Incomplete; }
                if b[j] == 0xFF {
                    let x = b[j + 1];
                    if x == 0x00 || (0xD0..=0xD7).contains(&x) { j += 2; continue; }
                    if x == 0xFF { j += 1; continue; }
                    break;
                }
                j += 1;
            }
            i = j;
            continue;
        }
        i = seg_end;
    }
}

/// Collects bytes and hands back whole frames.
#[derive(Default)]
pub struct Splitter { buf: Vec<u8> }

impl Splitter {
    pub fn new() -> Splitter { Splitter::default() }
    pub fn push(&mut self, data: &[u8]) -> Vec<Vec<u8>> {
        self.buf.extend_from_slice(data);
        let mut out = vec![];
        loop {
            // To the next start of image.
            match self.buf.windows(2).position(|w| w[0] == 0xFF && w[1] == 0xD8) {
                Some(0) => {}
                Some(p) => { self.buf.drain(..p); }
                None => { let keep = if self.buf.last() == Some(&0xFF) { 1 } else { 0 }; let l = self.buf.len(); self.buf.drain(..l - keep); return out; }
            }
            match frame_end(&self.buf) {
                End::Complete(e) => { out.push(self.buf[..e].to_vec()); self.buf.drain(..e); }
                End::Incomplete => {
                    // A frame never gets this big: something's wrong, start again at the next one.
                    if self.buf.len() > 64 << 20 { self.buf.drain(..2); continue; }
                    return out;
                }
                End::Corrupt => { self.buf.drain(..2); }
            }
        }
    }
}

/// Reads a stream to its end, handing each frame to `f`.
pub fn pump(mut r: impl std::io::Read, mut f: impl FnMut(Vec<u8>)) {
    let mut s = Splitter::new();
    let mut chunk = vec![0u8; 256 * 1024];
    loop {
        match r.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => for fr in s.push(&chunk[..n]) { f(fr) },
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    /// A tiny frame: SOI, a quantisation table holding FF D9 (to trip a naive search), SOS, data with a stuffed
    /// FF 00 and a restart marker, EOI.
    pub fn frame(tag: u8) -> Vec<u8> {
        let mut v = vec![0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x06, 0x00, 0xFF, 0xD9, tag];
        v.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x04, 0x01, 0x02, 0x11, 0xFF, 0x00, 0x22, 0xFF, 0xD0, 0x33, tag, 0xFF, 0xD9]);
        v
    }
    #[test]
    fn whole_frames() {
        let f = frame(7);
        assert_eq!(frame_end(&f), End::Complete(f.len()));
        assert_eq!(frame_end(&f[..f.len() - 1]), End::Incomplete);
        let mut s = Splitter::new();
        let mut all = b"junk".to_vec();
        all.extend(frame(1));
        all.extend(frame(2));
        all.extend(frame(3));
        let mut got = vec![];
        for c in all.chunks(5) { got.extend(s.push(c)); }
        assert_eq!(got, vec![frame(1), frame(2), frame(3)]);
    }
    #[test]
    fn recovers_from_garbage() {
        let mut s = Splitter::new();
        let mut all = vec![0xFF, 0xD8, 0x12, 0x34];
        all.extend(frame(9));
        assert_eq!(s.push(&all), vec![frame(9)]);
    }
}
