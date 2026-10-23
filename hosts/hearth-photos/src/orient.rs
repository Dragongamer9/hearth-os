//! Turning and flipping pictures. A picture's EXIF orientation (1 to 8) says how to turn the stored pixels to show
//! them upright; turning or flipping on screen is one more of those eight. For a JPEG, saving the turn just writes a
//! new orientation into its EXIF block: not one pixel is re-compressed, so nothing is lost.

/// How each EXIF orientation moves a point (x right, y down, about the centre): x' = a·x + b·y, y' = c·x + d·y.
const MATS: [[i8; 4]; 8] = [
    [1, 0, 0, 1],   // 1 as stored
    [-1, 0, 0, 1],  // 2 mirrored left to right
    [-1, 0, 0, -1], // 3 upside down
    [1, 0, 0, -1],  // 4 mirrored top to bottom
    [0, 1, 1, 0],   // 5 transposed
    [0, -1, 1, 0],  // 6 turned right (90° clockwise)
    [0, -1, -1, 0], // 7 transversed
    [0, 1, -1, 0],  // 8 turned left
];

pub const ROTATE_RIGHT: u8 = 6;
pub const ROTATE_LEFT: u8 = 8;
pub const FLIP_H: u8 = 2;
pub const FLIP_V: u8 = 4;

fn mat(o: u8) -> [i8; 4] { MATS[(o.clamp(1, 8) - 1) as usize] }

fn from_mat(m: [i8; 4]) -> u8 { MATS.iter().position(|x| *x == m).map(|i| i as u8 + 1).unwrap_or(1) }

/// `op` done after `o`: the orientation that shows the picture as `o` would and then turned by `op`.
pub fn then(o: u8, op: u8) -> u8 {
    let (u, m) = (mat(op), mat(o));
    from_mat([
        u[0] * m[0] + u[1] * m[2],
        u[0] * m[1] + u[1] * m[3],
        u[2] * m[0] + u[3] * m[2],
        u[2] * m[1] + u[3] * m[3],
    ])
}

/// Turns decoded pixels the way orientation `o` says (to show them upright, or to bake a turn in).
pub fn apply(img: image::DynamicImage, o: u8) -> image::DynamicImage {
    match o {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

// ------------------------------------------------------------------ the EXIF block inside a JPEG

#[derive(Debug, PartialEq)]
pub enum Patch {
    /// The file with its new orientation.
    Done(Vec<u8>),
    /// The JPEG has EXIF details but no orientation entry to change (rare): the caller saves a turned copy.
    NoTag,
}

struct Tiff<'a> { b: &'a mut [u8], le: bool }

impl Tiff<'_> {
    fn u16(&self, at: usize) -> Option<u16> {
        let s = self.b.get(at..at + 2)?;
        Some(if self.le { u16::from_le_bytes([s[0], s[1]]) } else { u16::from_be_bytes([s[0], s[1]]) })
    }
    fn u32(&self, at: usize) -> Option<u32> {
        let s = self.b.get(at..at + 4)?;
        Some(if self.le { u32::from_le_bytes([s[0], s[1], s[2], s[3]]) } else { u32::from_be_bytes([s[0], s[1], s[2], s[3]]) })
    }
    fn put16(&mut self, at: usize, v: u16) -> Option<()> {
        let bytes = if self.le { v.to_le_bytes() } else { v.to_be_bytes() };
        self.b.get_mut(at..at + 2)?.copy_from_slice(&bytes);
        Some(())
    }
    fn put32(&mut self, at: usize, v: u32) -> Option<()> {
        let bytes = if self.le { v.to_le_bytes() } else { v.to_be_bytes() };
        self.b.get_mut(at..at + 4)?.copy_from_slice(&bytes);
        Some(())
    }
}

/// Where each segment before the picture data starts: (marker, start of the FF, length of the data after the marker).
fn segments(j: &[u8]) -> Option<Vec<(u8, usize, usize)>> {
    if j.len() < 4 || j[0] != 0xFF || j[1] != 0xD8 { return None; }
    let mut out = Vec::new();
    let mut i = 2;
    while i + 4 <= j.len() {
        if j[i] != 0xFF { return None; }
        let m = j[i + 1];
        if m == 0xFF { i += 1; continue; } // fill bytes
        if m == 0xD9 || m == 0xDA { break; } // end, or the picture data starts
        if (0xD0..=0xD7).contains(&m) || m == 0x01 { i += 2; continue; }
        let len = u16::from_be_bytes([j[i + 2], j[i + 3]]) as usize;
        if len < 2 || i + 2 + len > j.len() { return None; }
        out.push((m, i, len));
        i += 2 + len;
    }
    Some(out)
}

/// The TIFF part of the JPEG's EXIF block (its offset in the file), if it has one.
fn exif_at(j: &[u8]) -> Option<(usize, usize)> {
    for (m, at, len) in segments(j)? {
        if m == 0xE1 && len >= 8 && &j[at + 4..at + 10] == b"Exif\0\0" { return Some((at + 10, len - 8)); }
    }
    None
}

/// The orientation a JPEG says (1 when it says nothing).
pub fn jpeg_orientation(j: &[u8]) -> u8 {
    let Some((start, len)) = exif_at(j) else { return 1 };
    let mut copy = j[start..start + len].to_vec();
    find_orientation(&mut copy).and_then(|(t, at)| t.u16(at)).map(|v| v as u8).filter(|v| (1..=8).contains(v)).unwrap_or(1)
}

/// The TIFF reader and where the orientation's value is.
fn find_orientation(b: &mut [u8]) -> Option<(Tiff<'_>, usize)> {
    let le = match b.get(0..4)? { [b'I', b'I', 42, 0] => true, [b'M', b'M', 0, 42] => false, _ => return None };
    let t = Tiff { b, le };
    let ifd = t.u32(4)? as usize;
    let n = t.u16(ifd)? as usize;
    for k in 0..n {
        let e = ifd + 2 + k * 12;
        if t.u16(e)? == 0x0112 && t.u16(e + 2)? == 3 { return Some((t, e + 8)); }
    }
    None
}

/// The JPEG with orientation `o` written into its EXIF block (one is added when it has none). Nothing else in the
/// file changes. `drop_thumbnail` also unlinks EXIF's little preview picture (after an edit it would show the old
/// picture in some apps).
pub fn set_jpeg_orientation(j: &[u8], o: u8, drop_thumbnail: bool) -> Result<Patch, String> {
    let segs = segments(j).ok_or("This JPEG looks damaged, so it wasn't changed")?;
    if let Some((start, len)) = exif_at(j) {
        let mut out = j.to_vec();
        let block = &mut out[start..start + len];
        let le = matches!(block.get(0..2), Some(b"II"));
        if drop_thumbnail {
            let mut t = Tiff { b: &mut *block, le };
            if let Some(ifd) = t.u32(4) {
                if let Some(n) = t.u16(ifd as usize) { let _ = t.put32(ifd as usize + 2 + n as usize * 12, 0); }
            }
        }
        return match find_orientation(block) {
            Some((mut t, at)) => { t.put16(at, o as u16).ok_or("EXIF block is cut short")?; Ok(Patch::Done(out)) }
            None => Ok(Patch::NoTag),
        };
    }
    // No EXIF block: a small one with just the orientation, after SOI (and after JFIF's APP0 if it's first).
    let mut app1 = vec![0xFF, 0xE1, 0, 34];
    app1.extend_from_slice(b"Exif\0\0");
    app1.extend_from_slice(&[b'M', b'M', 0, 42, 0, 0, 0, 8]); // big-endian TIFF, first IFD at 8
    app1.extend_from_slice(&[0, 1]); // one entry
    app1.extend_from_slice(&[0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, o, 0, 0]); // Orientation, SHORT, 1 value
    app1.extend_from_slice(&[0, 0, 0, 0]); // no next IFD
    debug_assert_eq!(app1.len(), 36);
    let at = match segs.first() { Some((0xE0, s, len)) => s + 2 + len, _ => 2 };
    let mut out = Vec::with_capacity(j.len() + app1.len());
    out.extend_from_slice(&j[..at]);
    out.extend_from_slice(&app1);
    out.extend_from_slice(&j[at..]);
    Ok(Patch::Done(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_adds_up() {
        assert_eq!(then(1, ROTATE_RIGHT), 6);
        assert_eq!(then(6, ROTATE_RIGHT), 3);
        assert_eq!(then(3, ROTATE_RIGHT), 8);
        assert_eq!(then(8, ROTATE_RIGHT), 1);
        assert_eq!(then(6, ROTATE_LEFT), 1);
        assert_eq!(then(1, FLIP_H), 2);
        assert_eq!(then(2, FLIP_H), 1);
        assert_eq!(then(1, FLIP_V), 4);
        // Flipping then turning right is "transverse"; turning right then flipping is "transpose".
        assert_eq!(then(FLIP_H, ROTATE_RIGHT), 7);
        assert_eq!(then(ROTATE_RIGHT, FLIP_H), 5);
        for o in 1..=8 { for op in 1..=8 { assert!((1..=8).contains(&then(o, op))); } }
    }

    #[test]
    fn pixels_follow_the_orientation() {
        // A 2×1 picture: red on the left, blue on the right.
        let mut img = image::RgbImage::new(2, 1);
        img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
        img.put_pixel(1, 0, image::Rgb([0, 0, 255]));
        let d = image::DynamicImage::ImageRgb8(img);
        let right = apply(d.clone(), 6).to_rgb8();
        assert_eq!((right.width(), right.height()), (1, 2));
        assert_eq!(right.get_pixel(0, 0).0, [255, 0, 0]); // turned right: the left end goes to the top
        let left = apply(d.clone(), 8).to_rgb8();
        assert_eq!(left.get_pixel(0, 0).0, [0, 0, 255]);
        assert_eq!(apply(d, 2).to_rgb8().get_pixel(0, 0).0, [0, 0, 255]);
    }

    fn tiny_jpeg() -> Vec<u8> {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(8, 4, image::Rgb([200, 100, 50])));
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Jpeg).unwrap();
        buf
    }

    #[test]
    fn jpeg_without_exif_gets_one() {
        let j = tiny_jpeg();
        assert_eq!(jpeg_orientation(&j), 1);
        let Patch::Done(out) = set_jpeg_orientation(&j, 6, false).unwrap() else { panic!() };
        assert_eq!(jpeg_orientation(&out), 6);
        // The picture data itself is untouched, and it still decodes (turned by readers that honour EXIF).
        assert!(out.ends_with(&j[j.len() - 64..]));
        assert_eq!(image::load_from_memory(&out).unwrap().width(), 8);
        // kamadak-exif reads what was written.
        let e = exif::Reader::new().read_from_container(&mut std::io::Cursor::new(&out)).unwrap();
        assert_eq!(e.get_field(exif::Tag::Orientation, exif::In::PRIMARY).unwrap().value.get_uint(0), Some(6));
        // And again: the existing entry is changed in place (same length).
        let Patch::Done(again) = set_jpeg_orientation(&out, 3, true).unwrap() else { panic!() };
        assert_eq!(again.len(), out.len());
        assert_eq!(jpeg_orientation(&again), 3);
    }

    #[test]
    fn little_endian_exif() {
        // An EXIF block written little-endian (as most cameras do) with orientation 8 and a thumbnail link.
        let j = tiny_jpeg();
        let mut app1 = vec![0xFF, 0xE1, 0, 34];
        app1.extend_from_slice(b"Exif\0\0");
        app1.extend_from_slice(&[b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0]);
        app1.extend_from_slice(&[0x12, 0x01, 3, 0, 1, 0, 0, 0, 8, 0, 0, 0]);
        app1.extend_from_slice(&[26, 0, 0, 0]);
        let mut withexif = j[..2].to_vec();
        withexif.extend_from_slice(&app1);
        withexif.extend_from_slice(&j[2..]);
        assert_eq!(jpeg_orientation(&withexif), 8);
        let Patch::Done(out) = set_jpeg_orientation(&withexif, then(8, ROTATE_RIGHT), true).unwrap() else { panic!() };
        assert_eq!(jpeg_orientation(&out), 1);
        // The thumbnail link (next IFD) is now 0.
        let at = 2 + 4 + 6 + 8 + 2 + 12;
        assert_eq!(&out[at..at + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn not_a_jpeg() {
        assert!(set_jpeg_orientation(b"\x89PNG....", 6, false).is_err());
    }
}
