//! The short code for pairing over the internet: 10 characters like `K7M-4QX-92HC`, typed on the other device.
//!
//! - The alphabet leaves out what people mix up: no 0/O, no 1/I/L (31 characters: 2-9 and A-Z without I, L, O).
//!   Spaces, dashes and case don't matter.
//! - 9 random characters (about 45 bits), then a check character: the weighted sum of all 10 (weights 1..10) is 0
//!   modulo 31, a prime, so any one wrong character and any two neighbours swapped are caught as soon as it's typed
//!   ("that code doesn't look right"), before anything goes over the network.
//! - How the other device finds this one from it (lookup.rs): a key pair is derived from the code (slowly, on
//!   purpose: 200 000 rounds of BLAKE3), and this device publishes a small signed record under that key on public
//!   pkarr relays (n0's, which iroh itself uses, and pkarr's own, which puts it on the Mainline DHT) for as long as
//!   the code is showing (10 minutes). The record is sealed with a second key from the code: it holds this device's
//!   id, name, relay and addresses. Someone who sees the record without the code learns nothing from it and can't
//!   replace it (it's signed with the code's key).
//! - Pairing itself is still SPAKE2 with the code as the password (pair.rs): finding the record grants nothing.

/// The characters a code is made of.
pub const ALPHABET: &[u8; 31] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
pub const LEN: usize = 10;
const DATA: usize = LEN - 1;
/// Rounds of BLAKE3 between the code and the keys: makes trying codes against a record found on the DHT slow
/// (31^9 codes, each costing this much), while a device does it once in a few tens of milliseconds.
pub const ROUNDS: u32 = 200_000;

fn value(c: u8) -> Option<u32> { ALPHABET.iter().position(|a| *a == c).map(|i| i as u32) }

fn check_value(data: &[u32]) -> u32 {
    // Sum of (i+1)·v[i] over the 9 data characters, plus 10·c ≡ 0 (mod 31): c = -s · 10⁻¹. 10⁻¹ mod 31 = 28.
    let s: u32 = data.iter().enumerate().map(|(i, v)| (i as u32 + 1) * v).sum::<u32>() % 31;
    ((31 - s) % 31) * 28 % 31
}

/// A new code (10 characters, no dashes).
pub fn new_code() -> String {
    let mut data = Vec::with_capacity(DATA);
    while data.len() < DATA {
        for b in crate::random_bytes::<16>() {
            // 248 = 8 × 31: no bias.
            if b < 248 && data.len() < DATA { data.push(b as u32 % 31); }
        }
    }
    let c = check_value(&data);
    data.iter().chain(std::iter::once(&c)).map(|v| ALPHABET[*v as usize] as char).collect()
}

/// `K7M4QX92HC` as people read it: `K7M-4QX-92HC`.
pub fn format(code: &str) -> String {
    let c: Vec<char> = code.chars().collect();
    if c.len() != LEN { return code.to_string(); }
    format!("{}-{}-{}", c[..3].iter().collect::<String>(), c[3..6].iter().collect::<String>(), c[6..].iter().collect::<String>())
}

/// What was typed, without spaces, dashes and dots, in capitals.
pub fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace() && !matches!(c, '-' | '.' | '_' | '·')).flat_map(char::to_uppercase).collect()
}

/// A typed short code, checked: the code (10 characters) or why it can't be one, in words for the screen.
pub fn parse(s: &str) -> Result<String, String> {
    let c = squash(s);
    if c.is_empty() { return Err("Type the code the other device shows".into()); }
    if let Some(bad) = c.chars().find(|ch| matches!(ch, '0' | 'O' | '1' | 'I' | 'L')) {
        return Err(format!("That code doesn't look right: codes never have {bad} in them (no 0, O, 1, I or L). Check it on the other device"));
    }
    if let Some(bad) = c.chars().find(|ch| !ch.is_ascii() || value(*ch as u8).is_none()) {
        return Err(format!("That code doesn't look right: \"{bad}\" isn't in codes. Check it on the other device"));
    }
    if c.len() != LEN { return Err(format!("That code doesn't look right: it's {LEN} characters, and this is {}", c.len())); }
    let v: Vec<u32> = c.bytes().map(|b| value(b).unwrap_or(0)).collect();
    let total: u32 = v.iter().enumerate().map(|(i, x)| (i as u32 + 1) * x).sum::<u32>() % 31;
    if total != 0 { return Err("That code doesn't look right: one of the characters is off. Check it on the other device".into()); }
    Ok(c)
}

/// Looks like someone typed a short code (rather than a 6-digit code).
pub fn looks_like(s: &str) -> bool {
    let c = squash(s);
    c.len() >= 8 && !c.chars().all(|ch| ch.is_ascii_digit())
}

/// The two keys from a code: the record's signing key (its public half is where the record is found) and the key
/// that seals what's in it.
pub fn keys(code: &str) -> (iroh::SecretKey, [u8; 32]) {
    let mut x = blake3::derive_key("hearth-link short code v1", code.as_bytes());
    for i in 0..ROUNDS {
        let mut h = blake3::Hasher::new_keyed(&x);
        h.update(&i.to_le_bytes());
        x = *h.finalize().as_bytes();
    }
    let sign = blake3::derive_key("hearth-link short code record key v1", &x);
    let seal = blake3::derive_key("hearth-link short code seal v1", &x);
    (iroh::SecretKey::from_bytes(&sign), seal)
}

fn mac(key: &[u8; 32], data: &[u8]) -> [u8; 16] {
    let k = blake3::derive_key("hearth-link short code mac v1", key);
    let h = blake3::keyed_hash(&k, data);
    let mut out = [0u8; 16];
    out.copy_from_slice(&h.as_bytes()[..16]);
    out
}

fn stream(key: &[u8; 32], n: usize) -> Vec<u8> {
    let mut out = vec![0u8; n];
    blake3::Hasher::new_keyed(key).update(b"stream").finalize_xof().fill(&mut out);
    out
}

/// Seals `plain` (BLAKE3 as a stream cipher, then a MAC over what's sealed). The key is new for every code, so one
/// key never seals two different things.
pub fn seal(key: &[u8; 32], plain: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = plain.iter().zip(stream(key, plain.len())).map(|(a, b)| a ^ b).collect();
    let m = mac(key, &out);
    out.extend_from_slice(&m);
    out
}

pub fn open(key: &[u8; 32], sealed: &[u8]) -> Option<Vec<u8>> {
    if sealed.len() < 16 { return None; }
    let (ct, m) = sealed.split_at(sealed.len() - 16);
    let want = mac(key, ct);
    if want.iter().zip(m).fold(0u8, |acc, (a, b)| acc | (a ^ b)) != 0 { return None; }
    Some(ct.iter().zip(stream(key, ct.len())).map(|(a, b)| a ^ b).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_codes_check_out() {
        for _ in 0..200 {
            let c = new_code();
            assert_eq!(c.len(), LEN);
            assert!(c.bytes().all(|b| ALPHABET.contains(&b)));
            assert_eq!(parse(&c), Ok(c.clone()));
            assert_eq!(parse(&format(&c).to_lowercase()), Ok(c.clone()));
            assert!(looks_like(&format(&c)));
        }
    }
    #[test]
    fn known_code() {
        // The same as the page's own check (ui/index.html lkShortCheck): 23456789A plus its check character.
        let data: Vec<u32> = b"23456789A".iter().map(|b| value(*b).unwrap()).collect();
        let c = ALPHABET[check_value(&data) as usize] as char;
        assert_eq!(c, '9');
        assert_eq!(parse("234-567-89a9"), Ok("23456789A9".into()));
        assert_eq!(format("23456789A9"), "234-567-89A9");
    }
    #[test]
    fn typos_are_caught() {
        let c = new_code();
        let b = c.as_bytes();
        for i in 0..LEN {
            for a in ALPHABET.iter() {
                if *a == b[i] { continue; }
                let mut t = b.to_vec();
                t[i] = *a;
                assert!(parse(std::str::from_utf8(&t).unwrap()).is_err(), "one wrong character got through");
            }
            if i + 1 < LEN && b[i] != b[i + 1] {
                let mut t = b.to_vec();
                t.swap(i, i + 1);
                assert!(parse(std::str::from_utf8(&t).unwrap()).is_err(), "two swapped characters got through");
            }
        }
        assert!(parse("K7M-4QX-92H").unwrap_err().contains("10 characters"));
        assert!(parse("K7M-4QX-92H0").unwrap_err().contains("never"));
        assert!(parse("").is_err());
        assert!(!looks_like("123 456"));
    }
    #[test]
    fn sealing() {
        let (sk, k) = keys("23456789A9");
        let (sk2, k2) = keys("23456789A9");
        assert_eq!(sk.public(), sk2.public());
        assert_eq!(k, k2);
        let (other, _) = keys(&new_code());
        assert_ne!(other.public(), sk.public());
        let s = seal(&k, b"hello there");
        assert_eq!(open(&k, &s).as_deref(), Some(&b"hello there"[..]));
        let mut bad = s.clone();
        bad[0] ^= 1;
        assert_eq!(open(&k, &bad), None);
        assert_eq!(open(&[9; 32], &s), None);
    }
}
