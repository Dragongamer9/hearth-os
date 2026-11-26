//! Base64 (standard alphabet, with padding) both ways, and base32 (RFC 4648) for 2-step code secrets.

const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode(b: &[u8]) -> String {
    let mut o = String::with_capacity(b.len() * 4 / 3 + 4);
    for ch in b.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            if k <= ch.len() {
                o.push(T[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                o.push('=');
            }
        }
    }
    o
}

/// Decodes, skipping anything outside the alphabet (line breaks); `-` and `_` (the URL alphabet) count too.
pub fn decode(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => continue,
        } as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    out
}

/// Base32 (A-Z, 2-7; spaces, dashes and padding ignored; any case).
pub fn base32(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = 0u64;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c.to_ascii_uppercase() {
            x @ b'A'..=b'Z' => x - b'A',
            x @ b'2'..=b'7' => x - b'2' + 26,
            _ => continue,
        } as u64;
        buf = (buf << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn both_ways() {
        for s in ["", "a", "ab", "abc", "Hearth ✓"] {
            assert_eq!(super::decode(&super::encode(s.as_bytes())), s.as_bytes());
        }
        assert_eq!(super::base32("JBSWY3DPEHPK3PXP"), b"Hello!\xde\xad\xbe\xef");
    }
}
