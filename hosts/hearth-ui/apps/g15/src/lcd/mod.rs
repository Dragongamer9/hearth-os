//! The keyboard's screen: 160 x 43 dots, one bit each. A [`Frame`] is drawn with the 5x7 font, lines, boxes, bars
//! and pictures ([`screen`] draws whole screens from a list of elements), then [`pack`]ed into the report the
//! keyboard takes: output report 3, 32 bytes of header (the first is the report number, the rest 0), then 6 rows of
//! 160 bytes, each byte a column of 8 dots with bit 0 at the top (lcdproc's g15 driver and libg15 do the same; the 43rd
//! row is the third bit of the last row). 992 bytes, written to the G-key part's /dev/hidrawN.

pub mod font;
pub mod picture;
pub mod screen;

pub const W: i32 = 160;
pub const H: i32 = 43;
pub const REPORT_LEN: usize = 32 + 6 * 160;

#[derive(Clone, PartialEq, Eq)]
pub struct Frame {
    pub px: Vec<u8>,
}

impl Default for Frame {
    fn default() -> Self {
        Frame::new()
    }
}

impl Frame {
    pub fn new() -> Frame {
        Frame { px: vec![0; (W * H) as usize] }
    }
    pub fn get(&self, x: i32, y: i32) -> bool {
        (0..W).contains(&x) && (0..H).contains(&y) && self.px[(y * W + x) as usize] != 0
    }
    /// Sets a dot (on: dark) or clears it; outside the screen does nothing.
    pub fn dot(&mut self, x: i32, y: i32, on: bool) {
        if (0..W).contains(&x) && (0..H).contains(&y) {
            self.px[(y * W + x) as usize] = on as u8;
        }
    }
    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, on: bool) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.dot(xx, yy, on);
            }
        }
    }
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, on: bool) {
        if w <= 0 || h <= 0 {
            return;
        }
        self.fill(x, y, w, 1, on);
        self.fill(x, y + h - 1, w, 1, on);
        self.fill(x, y, 1, h, on);
        self.fill(x + w - 1, y, 1, h, on);
    }
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, on: bool) {
        let n = (x1 - x0).abs().max((y1 - y0).abs());
        if n == 0 {
            self.dot(x0, y0, on);
            return;
        }
        for j in 0..=n {
            let x = x0 as f32 + (x1 - x0) as f32 * j as f32 / n as f32;
            let y = y0 as f32 + (y1 - y0) as f32 * j as f32 / n as f32;
            self.dot(x.round() as i32, y.round() as i32, on);
        }
    }
    /// Text at (x, y), each dot `scale` x `scale`; returns where the next character would start.
    pub fn text(&mut self, s: &str, x: i32, y: i32, scale: i32, on: bool) -> i32 {
        let z = scale.max(1);
        let mut cx = x;
        for ch in s.chars() {
            let g = font::glyph(ch);
            for (col, bits) in g.iter().enumerate() {
                for r in 0..7 {
                    if *bits >> r & 1 == 1 {
                        self.fill(cx + col as i32 * z, y + r * z, z, z, on);
                    }
                }
            }
            cx += font::ADVANCE * z;
        }
        cx
    }
    /// Text cut to fit `w` dots (with "…" when it doesn't).
    pub fn text_fit(&mut self, s: &str, x: i32, y: i32, scale: i32, w: i32, on: bool) {
        self.text(&fit(s, scale, w), x, y, scale, on);
    }
    /// A bar like the mockups': a frame 6 dots high, filled to `frac` inside.
    pub fn bar(&mut self, x: i32, y: i32, w: i32, h: i32, frac: f32, on: bool) {
        let h = h.max(3);
        self.rect(x, y, w, h, on);
        let f = frac.clamp(0.0, 1.0);
        self.fill(x + 1, y + 1, ((w - 2) as f32 * f).round() as i32, h - 2, on);
    }
    /// A filled circle (the recording dot).
    pub fn disc(&mut self, cx: i32, cy: i32, r: i32, on: bool) {
        for y in -r..=r {
            for x in -r..=r {
                if x * x + y * y <= r * r + r / 2 {
                    self.dot(cx + x, cy + y, on);
                }
            }
        }
    }
    /// A picture: `bits` rows of `w` dots, row by row, the first dot in the highest bit of each byte (rows start on
    /// a new byte). The app turns pictures into these dots (dithered) when you add one.
    pub fn picture(&mut self, x: i32, y: i32, w: i32, h: i32, bits: &[u8]) {
        let stride = ((w + 7) / 8) as usize;
        for yy in 0..h {
            for xx in 0..w {
                let i = yy as usize * stride + (xx / 8) as usize;
                if i < bits.len() && bits[i] >> (7 - (xx % 8)) & 1 == 1 {
                    self.dot(x + xx, y + yy, true);
                }
            }
        }
    }
    /// A picture whose dots are bw x bh, stretched (nearest dot) to w x h: the editor while you drag a picture's corner,
    /// before it's turned into dots again at its new size.
    pub fn picture_scaled(&mut self, x: i32, y: i32, w: i32, h: i32, bits: &[u8], bw: i32, bh: i32) {
        if bw <= 0 || bh <= 0 || w <= 0 || h <= 0 {
            return;
        }
        if bw == w && bh == h {
            return self.picture(x, y, w, h, bits);
        }
        let stride = ((bw + 7) / 8) as usize;
        for yy in 0..h {
            let sy = (yy * bh / h).min(bh - 1);
            for xx in 0..w {
                let sx = (xx * bw / w).min(bw - 1);
                let i = sy as usize * stride + (sx / 8) as usize;
                if i < bits.len() && bits[i] >> (7 - (sx % 8)) & 1 == 1 {
                    self.dot(x + xx, y + yy, true);
                }
            }
        }
    }
    /// Turns every dot in a box the other way (white text on a dark bar).
    pub fn invert(&mut self, x: i32, y: i32, w: i32, h: i32) {
        for yy in y..y + h {
            for xx in x..x + w {
                let v = self.get(xx, yy);
                self.dot(xx, yy, !v);
            }
        }
    }
    pub fn lit(&self) -> usize {
        self.px.iter().filter(|&&p| p != 0).count()
    }
}

pub fn fit(s: &str, scale: i32, w: i32) -> String {
    if font::width(s, scale) <= w {
        return s.to_string();
    }
    let mut out: String = String::new();
    for ch in s.chars() {
        let mut t = out.clone();
        t.push(ch);
        t.push('…');
        if font::width(&t, scale) > w {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

/// The 992-byte report for the keyboard.
pub fn pack(f: &Frame) -> Vec<u8> {
    let mut buf = vec![0u8; REPORT_LEN];
    buf[0] = 0x03;
    for row in 0..6 {
        for x in 0..W {
            let mut b = 0u8;
            for bit in 0..8 {
                if f.get(x, row * 8 + bit) {
                    b |= 1 << bit;
                }
            }
            buf[32 + (row * W + x) as usize] = b;
        }
    }
    buf
}

/// The frame back from a report (the tests, and the pretend keyboard's pictures).
pub fn unpack(buf: &[u8]) -> Option<Frame> {
    if buf.len() < REPORT_LEN || buf[0] != 0x03 {
        return None;
    }
    let mut f = Frame::new();
    for row in 0..6 {
        for x in 0..W {
            let b = buf[32 + (row * W + x) as usize];
            for bit in 0..8 {
                if b >> bit & 1 == 1 {
                    f.dot(x, row * 8 + bit, true);
                }
            }
        }
    }
    Some(f)
}

/// The screen's contrast (0 low, 1 medium, 2 high): the G15's feature report 2 {2, 32, 129, 18 | 22 | 26}, as
/// libg15's setLCDContrast sends it, through the hidraw node (HIDIOCSFEATURE). hid-lg-g15 doesn't offer it.
pub fn set_contrast(hidraw: &std::path::Path, level: u32) -> std::io::Result<()> {
    use std::os::unix::io::AsRawFd;
    let f = std::fs::OpenOptions::new().write(true).open(hidraw)?;
    if f.metadata().map(|m| m.is_file()).unwrap_or(false) {
        return Ok(()); // the pretend keyboard
    }
    let mut buf = [2u8, 32, 129, [18u8, 22, 26][level.min(2) as usize]];
    const HIDIOCSFEATURE_4: u64 = 0xC004_4806;
    let r = unsafe { libc::ioctl(f.as_raw_fd(), HIDIOCSFEATURE_4 as _, buf.as_mut_ptr()) };
    if r < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// A picture turned into the screen's dots (the editor's "Pictures are turned into the screen's dots for you"):
/// `gray` is w x h brightness 0-255, scaled to out_w x out_h and dithered (Floyd-Steinberg); dark becomes a dot.
/// Returns rows of out_w dots, the first in the highest bit (Frame::picture's format).
pub fn dither(gray: &[u8], w: usize, h: usize, out_w: usize, out_h: usize) -> Vec<u8> {
    dither_with(gray, w, h, out_w, out_h, 128, true)
}

/// [`dither`] with a choice: `threshold` (0-255: below it is a dot; higher makes the picture darker) and `shade`
/// (Floyd-Steinberg, so greys become patterns of dots; false: every dot is just dark or light, sharp for logos and
/// text).
pub fn dither_with(gray: &[u8], w: usize, h: usize, out_w: usize, out_h: usize, threshold: u8, shade: bool) -> Vec<u8> {
    let (ow, oh) = (out_w.max(1), out_h.max(1));
    let mut v = vec![0f32; ow * oh];
    for y in 0..oh {
        for x in 0..ow {
            // The average of the source dots under this one.
            let (x0, x1) = (x * w / ow, ((x + 1) * w / ow).max(x * w / ow + 1).min(w.max(1)));
            let (y0, y1) = (y * h / oh, ((y + 1) * h / oh).max(y * h / oh + 1).min(h.max(1)));
            let (mut sum, mut n) = (0f32, 0f32);
            for yy in y0..y1 {
                for xx in x0..x1 {
                    if let Some(g) = gray.get(yy * w + xx) {
                        sum += *g as f32;
                        n += 1.0;
                    }
                }
            }
            let g = if n > 0.0 { sum / n } else { 255.0 };
            // Shaded, the dots average out to the picture's greys whatever the threshold, so the threshold makes the
            // whole picture lighter or darker instead.
            v[y * ow + x] = if shade { (g - (threshold as f32 - 128.0)).clamp(0.0, 255.0) } else { g };
        }
    }
    let stride = (ow + 7) / 8;
    let mut out = vec![0u8; stride * oh];
    for y in 0..oh {
        for x in 0..ow {
            let old = v[y * ow + x];
            let dark = old < if shade { 128.0 } else { threshold as f32 };
            if dark {
                out[y * stride + x / 8] |= 0x80 >> (x % 8);
            }
            if !shade {
                continue;
            }
            let err = old - if dark { 0.0 } else { 255.0 };
            let mut add = |dx: isize, dy: usize, f: f32| {
                let nx = x as isize + dx;
                if nx >= 0 && (nx as usize) < ow && y + dy < oh {
                    v[(y + dy) * ow + nx as usize] += err * f;
                }
            };
            add(1, 0, 7.0 / 16.0);
            add(-1, 1, 3.0 / 16.0);
            add(0, 1, 5.0 / 16.0);
            add(1, 1, 1.0 / 16.0);
        }
    }
    out
}

/// Bytes as base64 (the "bits" of a picture element in g15.json).
pub fn base64(b: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        s.push(A[(n >> 18) as usize & 63] as char);
        s.push(A[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 { A[(n >> 6) as usize & 63] as char } else { '=' });
        s.push(if c.len() > 2 { A[n as usize & 63] as char } else { '=' });
    }
    s
}

// ------------------------------------------------------------------ PNG (the CI pictures; standard library only)
fn crc32(data: &[u8]) -> u32 {
    let mut c: u32 = 0xffff_ffff;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut c = kind.to_vec();
    c.extend_from_slice(data);
    out.extend_from_slice(&c);
    out.extend_from_slice(&crc32(&c).to_be_bytes());
}

/// An RGB picture as a PNG (stored, not compressed: the pictures are small).
pub fn png(w: u32, h: u32, rgb: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((w * 3 + 1) as usize * h as usize);
    for y in 0..h as usize {
        raw.push(0);
        raw.extend_from_slice(&rgb[y * w as usize * 3..(y + 1) * w as usize * 3]);
    }
    let mut z = vec![0x78, 0x01];
    let chunks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, c) in chunks.iter().enumerate() {
        z.push(if i + 1 == chunks.len() { 1 } else { 0 });
        z.extend_from_slice(&(c.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(c.len() as u16)).to_le_bytes());
        z.extend_from_slice(c);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// The frame as the keyboard shows it, `scale` times bigger, in the mockups' colours (dark dots on the lit screen).
pub fn frame_png(f: &Frame, scale: u32) -> Vec<u8> {
    let s = scale.max(1);
    let pad = 4 * s;
    let (w, h) = (W as u32 * s + pad * 2, H as u32 * s + pad * 2);
    let (bg, ink) = ([0xe7u8, 0xa3, 0x3c], [0x2au8, 0x1a, 0x08]);
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let on = x >= pad && y >= pad && f.get(((x - pad) / s) as i32, ((y - pad) / s) as i32);
            rgb.extend_from_slice(if on { &ink } else { &bg });
        }
    }
    png(w, h, &rgb)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packs_as_the_keyboard_reads_it() {
        let mut f = Frame::new();
        f.dot(0, 0, true);
        f.dot(1, 7, true);
        f.dot(159, 42, true);
        f.dot(5, 8, true);
        let b = pack(&f);
        assert_eq!(b.len(), 992);
        assert_eq!(b[0], 3);
        assert!(b[1..32].iter().all(|&x| x == 0));
        assert_eq!(b[32], 0x01, "column 0, top dot is bit 0");
        assert_eq!(b[33], 0x80, "column 1, row 7 is bit 7");
        assert_eq!(b[32 + 160 + 5], 0x01, "row 8 starts the second band");
        assert_eq!(b[32 + 5 * 160 + 159], 0x04, "the 43rd row is bit 2 of the last band");
        assert!(unpack(&b).unwrap() == f);
    }
    #[test]
    fn draws_text_and_pngs() {
        let mut f = Frame::new();
        let end = f.text("Hi", 0, 0, 1, true);
        assert_eq!(end, 12);
        assert!(f.get(0, 0) && f.get(0, 6) && !f.get(0, 7));
        assert_eq!(fit("Hearth OS trailer music", 1, 60).chars().count(), 10);
        let p = frame_png(&f, 2);
        assert_eq!(&p[1..4], b"PNG");
        assert_eq!(crc32(b"IEND"), 0xae42_6082);
        // Black on the left half, white on the right: dots only on the left.
        let gray: Vec<u8> = (0..16 * 8).map(|i| if i % 16 < 8 { 0 } else { 255 }).collect();
        let bits = dither(&gray, 16, 8, 16, 8);
        assert_eq!(bits[0], 0xff);
        assert_eq!(bits[1], 0x00);
        assert_eq!(base64(&[0xff]), "/w==");
        assert_eq!(base64(b"Man"), "TWFu");
    }
}
