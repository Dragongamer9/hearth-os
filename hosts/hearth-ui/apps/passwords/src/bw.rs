//! A Bitwarden-protocol client for Vaultwarden (AGPL-3, github.com/dani-garcia/vaultwarden) on your own server, with
//! the well-known RustCrypto crates: your master password makes your master key (PBKDF2-SHA256 or Argon2id, as your
//! account says), which opens your vault's own key; every item stays encrypted (AES-256-CBC with HMAC-SHA256, Bitwarden's
//! "type 2" EncString) on the server and on this PC, and is opened only in memory while the app is unlocked.
//!
//! The master password is never logged, never written anywhere, never sent: only its hash (PBKDF2 of the master key,
//! as Bitwarden clients do) goes to your server to sign in. Keys are wiped from memory when the app locks.

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hearth_oss::b64;
use hearth_oss::http::{self, Req};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use zeroize::Zeroize;

type HmacSha256 = Hmac<Sha256>;

/// Two 32-byte keys: one encrypts, one signs.
#[derive(Clone)]
pub struct Keys {
    pub enc: [u8; 32],
    pub mac: [u8; 32],
}
impl Drop for Keys {
    fn drop(&mut self) {
        self.enc.zeroize();
        self.mac.zeroize();
    }
}
impl std::fmt::Debug for Keys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Keys(…)")
    }
}
impl Keys {
    pub fn from64(b: &[u8]) -> Option<Keys> {
        if b.len() < 64 {
            return None;
        }
        let mut k = Keys { enc: [0; 32], mac: [0; 32] };
        k.enc.copy_from_slice(&b[..32]);
        k.mac.copy_from_slice(&b[32..64]);
        Some(k)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Kdf {
    /// 0 PBKDF2-SHA256, 1 Argon2id
    pub kind: u8,
    pub iterations: u32,
    pub memory: u32,
    pub parallelism: u32,
}

pub fn kdf_of(v: &Value) -> Kdf {
    let n = |k: &[&str], d: u64| hearth_oss::ci(v, k).and_then(Value::as_u64).unwrap_or(d) as u32;
    Kdf { kind: n(&["kdf", "Kdf"], 0) as u8, iterations: n(&["kdfIterations", "KdfIterations"], 600_000), memory: n(&["kdfMemory", "KdfMemory"], 64), parallelism: n(&["kdfParallelism", "KdfParallelism"], 4) }
}

/// Your master key, from your master password and your email (the account's KDF).
pub fn master_key(password: &str, email: &str, kdf: &Kdf) -> Result<[u8; 32], String> {
    let mut out = [0u8; 32];
    let email = email.trim().to_lowercase();
    match kdf.kind {
        0 => pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), email.as_bytes(), kdf.iterations.max(1), &mut out),
        1 => {
            use sha2::Digest;
            let salt = Sha256::digest(email.as_bytes());
            let params = argon2::Params::new(kdf.memory.max(15) * 1024, kdf.iterations.max(1), kdf.parallelism.max(1), Some(32)).map_err(|e| e.to_string())?;
            argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params).hash_password_into(password.as_bytes(), &salt, &mut out).map_err(|e| e.to_string())?;
        }
        k => return Err(format!("Your account uses a way of making keys Hearth doesn't know ({k})")),
    }
    Ok(out)
}

/// What your server checks to sign you in (never the password itself).
pub fn master_hash(key: &[u8; 32], password: &str) -> String {
    let mut out = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(key, password.as_bytes(), 1, &mut out);
    let s = b64::encode(&out);
    out.zeroize();
    s
}

/// The master key stretched into two (HKDF-Expand with "enc" and "mac").
pub fn stretch(key: &[u8; 32]) -> Keys {
    let hk = hkdf::Hkdf::<Sha256>::from_prk(key).expect("a 32-byte key");
    let mut k = Keys { enc: [0; 32], mac: [0; 32] };
    let _ = hk.expand(b"enc", &mut k.enc);
    let _ = hk.expand(b"mac", &mut k.mac);
    k
}

/// "2.iv|data|mac" opened with these keys.
pub fn decrypt(s: &str, k: &Keys) -> Result<Vec<u8>, String> {
    let (kind, rest) = s.split_once('.').ok_or("not an encrypted value")?;
    if kind != "2" {
        return Err(format!("an encrypted value of a kind Hearth doesn't open ({kind})"));
    }
    let parts: Vec<&str> = rest.split('|').collect();
    if parts.len() < 3 {
        return Err("an encrypted value that's cut short".into());
    }
    let (iv, ct, mac) = (b64::decode(parts[0]), b64::decode(parts[1]), b64::decode(parts[2]));
    let mut m = <HmacSha256 as Mac>::new_from_slice(&k.mac).map_err(|e| e.to_string())?;
    m.update(&iv);
    m.update(&ct);
    m.verify_slice(&mac).map_err(|_| "Wrong master password (or the vault was changed by something else)".to_string())?;
    cbc::Decryptor::<aes::Aes256>::new_from_slices(&k.enc, &iv).map_err(|e| e.to_string())?.decrypt_padded_vec_mut::<Pkcs7>(&ct).map_err(|_| "it couldn't be opened".to_string())
}
pub fn decrypt_str(s: &str, k: &Keys) -> String {
    if s.is_empty() {
        return String::new();
    }
    decrypt(s, k).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default()
}

/// Encrypted with these keys, a fresh IV each time.
pub fn encrypt(plain: &[u8], k: &Keys) -> Result<String, String> {
    let mut iv = [0u8; 16];
    getrandom::getrandom(&mut iv).map_err(|e| e.to_string())?;
    let ct = cbc::Encryptor::<aes::Aes256>::new_from_slices(&k.enc, &iv).map_err(|e| e.to_string())?.encrypt_padded_vec_mut::<Pkcs7>(plain);
    let mut m = <HmacSha256 as Mac>::new_from_slice(&k.mac).map_err(|e| e.to_string())?;
    m.update(&iv);
    m.update(&ct);
    let mac = m.finalize().into_bytes();
    Ok(format!("2.{}|{}|{}", b64::encode(&iv), b64::encode(&ct), b64::encode(&mac)))
}

/// An organisation's key ("4.…", RSA-OAEP with SHA-1) opened with your private key (shared folders).
pub fn org_key(s: &str, private_der: &[u8]) -> Option<Keys> {
    use rsa::pkcs8::DecodePrivateKey;
    let body = s.split_once('.').map(|x| x.1).unwrap_or(s);
    let key = rsa::RsaPrivateKey::from_pkcs8_der(private_der).ok()?;
    let mut b = key.decrypt(rsa::Oaep::new::<sha1::Sha1>(), &b64::decode(body)).ok()?;
    let k = Keys::from64(&b);
    b.zeroize();
    k
}

// ------------------------------------------------------------------ the server

pub fn prelogin(base: &str, email: &str) -> Result<Kdf, String> {
    let r = http::send(&Req::new("POST", &format!("{base}/identity/accounts/prelogin")).json(&json!({"email": email.trim()})).timeout(15)).map_err(|e| format!("Your server doesn't answer: {e}"))?;
    if !r.ok() {
        return Err(format!("Your server answered {}", r.status));
    }
    Ok(kdf_of(&r.json().unwrap_or(Value::Null)))
}

/// Signed in: the answer (access_token, Key, PrivateKey…), or Err with a "two-step" hint when it asks for a code.
pub fn token(base: &str, email: &str, hash: &str, device: &str, code: &str) -> Result<Value, String> {
    let mut form: Vec<(&str, &str)> = vec![("grant_type", "password"), ("username", email.trim()), ("password", hash), ("scope", "api offline_access"), ("client_id", "desktop"), ("deviceType", "8"), ("deviceIdentifier", device), ("deviceName", "Hearth Passwords")];
    if !code.is_empty() {
        form.push(("twoFactorToken", code));
        form.push(("twoFactorProvider", "0"));
        form.push(("twoFactorRemember", "1"));
    }
    let r = http::send(&Req::new("POST", &format!("{base}/identity/connect/token")).form(&form).timeout(20)).map_err(|e| format!("Your server doesn't answer: {e}"))?;
    let j = r.json().unwrap_or(Value::Null);
    if r.ok() {
        return Ok(j);
    }
    if hearth_oss::ci(&j, &["TwoFactorProviders", "twoFactorProviders"]).is_some() {
        return Err("two-step".into());
    }
    let why = hearth_oss::ci_str(&j, &["error_description", "ErrorModel"]);
    Err(if why.to_lowercase().contains("username or password") || r.status == 400 { "Wrong email or master password".into() } else { format!("Your server said no: {why} ({})", r.status) })
}

pub fn get(base: &str, path: &str, token: &str) -> Result<Value, String> {
    let r = http::send(&Req::get(&format!("{base}{path}")).bearer(token).timeout(30)).map_err(|e| format!("Your server doesn't answer: {e}"))?;
    if !r.ok() {
        return Err(format!("Your server answered {}", r.status));
    }
    r.json().ok_or_else(|| "Your server's answer couldn't be read".into())
}

pub fn send(base: &str, method: &str, path: &str, token: &str, body: &Value) -> Result<Value, String> {
    let r = http::send(&Req::new(method, &format!("{base}{path}")).bearer(token).json(body).timeout(30)).map_err(|e| format!("Your server doesn't answer: {e}"))?;
    if !r.ok() {
        return Err(format!("Your server said no ({}): {}", r.status, hearth_oss::ci_str(&r.json().unwrap_or(Value::Null), &["message", "Message"])));
    }
    Ok(r.json().unwrap_or(Value::Null))
}

/// A JSON value with every key in camelCase (what the server takes back).
pub fn camel(v: &Value) -> Value {
    match v {
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, x)| {
                    let mut c = k.chars();
                    let key = match c.next() {
                        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
                        None => String::new(),
                    };
                    (key, camel(x))
                })
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(camel).collect()),
        x => x.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn keys_are_made_as_bitwarden_makes_them() {
        // vectors from Python's hashlib / hmac (PBKDF2-SHA256, HKDF-Expand)
        let kdf = Kdf { kind: 0, iterations: 5000, memory: 64, parallelism: 4 };
        let mk = master_key("hearth test password", "Owner@Example.com ", &kdf).unwrap();
        assert_eq!(hex(&mk), "5f7b22b7e383cb907672cf2b07280c0282686c5b199e91772d3f1cce45a4515e");
        assert_eq!(master_hash(&mk, "hearth test password"), "9y88BLI2UiZ/qR4z+9jhblPXzGwzbyGxHUxO3B4Avmo=");
        let s = stretch(&mk);
        assert_eq!(hex(&s.enc), "994a86d0d2986d6ee33eb780d033fa862ecd0b93e8f618f31b7655995b3b5a15");
        assert_eq!(hex(&s.mac), "776f79cc516e143cc248449a8c28e68f9a32ae62131f9441b3da62ef15f57eb1");
    }

    #[test]
    fn values_open_only_with_their_keys_and_unchanged() {
        let k = Keys::from64(&[7u8; 64]).unwrap();
        let e = encrypt("r7#Vq4mLp!8eZw2Tn%kHs9Yb".as_bytes(), &k).unwrap();
        assert!(e.starts_with("2."));
        assert_eq!(decrypt_str(&e, &k), "r7#Vq4mLp!8eZw2Tn%kHs9Yb");
        assert_ne!(encrypt(b"x", &k).unwrap(), encrypt(b"x", &k).unwrap(), "a fresh IV each time");
        let other = Keys::from64(&[8u8; 64]).unwrap();
        assert!(decrypt(&e, &other).is_err());
        let mut parts: Vec<String> = e.split('|').map(str::to_string).collect();
        parts[1] = b64::encode(b"tampered with the data");
        assert!(decrypt(&parts.join("|"), &k).is_err(), "a changed value is refused");
        assert!(format!("{k:?}") == "Keys(…)", "keys never show in a log");
    }

    #[test]
    fn answers_in_either_case_and_back_in_camel_case() {
        assert_eq!(kdf_of(&json!({"Kdf": 1, "KdfIterations": 3, "KdfMemory": 64, "KdfParallelism": 4})), Kdf { kind: 1, iterations: 3, memory: 64, parallelism: 4 });
        assert_eq!(camel(&json!({"Login": {"Password": "x", "Uris": [{"Uri": "u"}]}})), json!({"login": {"password": "x", "uris": [{"uri": "u"}]}}));
    }
}
