//! A tiny PNG writer for window pictures (screencopy.rs, ipc.rs): 8-bit RGBA, zlib's "stored" blocks (no
//! compression; the pictures are small and live in $XDG_RUNTIME_DIR for a moment), so the compositor needs no
//! image library.

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// `px`: rows top to bottom, 4 bytes a pixel (R, G, B, A), at least w * h * 4 bytes.
pub fn encode_rgba(w: u32, h: u32, px: &[u8]) -> Vec<u8> {
    let row = w as usize * 4;
    let mut raw = Vec::with_capacity((row + 1) * h as usize);
    for y in 0..h as usize {
        raw.push(0); // filter: none
        raw.extend_from_slice(&px[y * row..(y + 1) * row]);
    }
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, b) in blocks.iter().enumerate() {
        z.push(if i + 1 == blocks.len() { 1 } else { 0 });
        let len = b.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(b);
    }
    if blocks.is_empty() {
        z.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8 bits, RGBA, deflate, adaptive filters, no interlace
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksums_are_the_standard_ones() {
        assert_eq!(crc32(b"IEND"), 0xae42_6082);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn a_picture_reads_back() {
        let (w, h) = (300u32, 250u32); // more than one stored block
        let px: Vec<u8> = (0..w * h * 4).map(|i| (i % 251) as u8).collect();
        let png = encode_rgba(w, h, &px);
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
        // Undo the stored blocks and the row filters: the pixels come back.
        let len = u32::from_be_bytes(png[33..37].try_into().unwrap()) as usize;
        assert_eq!(&png[37..41], b"IDAT");
        let z = &png[41..41 + len];
        let mut raw = Vec::new();
        let mut i = 2;
        loop {
            let last = z[i] & 1 == 1;
            let n = u16::from_le_bytes([z[i + 1], z[i + 2]]) as usize;
            raw.extend_from_slice(&z[i + 5..i + 5 + n]);
            i += 5 + n;
            if last {
                break;
            }
        }
        assert_eq!(adler32(&raw), u32::from_be_bytes(z[i..i + 4].try_into().unwrap()));
        let back: Vec<u8> = raw.chunks(w as usize * 4 + 1).flat_map(|r| r[1..].to_vec()).collect();
        assert_eq!(back, px);
    }
}
