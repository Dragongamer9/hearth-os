//! This PC as a KDE Connect device: a device id (32 hex characters) and a self-signed certificate whose common name
//! is that id (organization "KDE", unit "Kde connect", as KDE Connect makes them; ECDSA P-256 like kdeconnect-kde's).
//! A paired phone is remembered by its certificate, and every later link must show the same one.

use sha2::{Digest, Sha256};
use std::path::Path;

pub struct Identity {
    pub device_id: String,
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>,
}

pub fn generate() -> Result<Identity, String> {
    let device_id = crate::random_hex(16);
    let key = rcgen::KeyPair::generate().map_err(|e| e.to_string())?;
    let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).map_err(|e| e.to_string())?;
    let mut dn = rcgen::DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, device_id.as_str());
    dn.push(rcgen::DnType::OrganizationName, "KDE");
    dn.push(rcgen::DnType::OrganizationalUnitName, "Kde connect");
    params.distinguished_name = dn;
    params.not_before = rcgen::date_time_ymd(2024, 1, 1);
    params.not_after = rcgen::date_time_ymd(2049, 12, 31);
    let cert = params.self_signed(&key).map_err(|e| e.to_string())?;
    Ok(Identity { device_id, cert_der: cert.der().to_vec(), key_der: key.serialize_der() })
}

/// `dir`/identity.json, made the first time (only this user can read it: the private key is in it).
pub fn load_or_create(dir: &Path) -> Result<Identity, String> {
    let path = dir.join("identity.json");
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let (Some(id), Some(c), Some(k)) = (v["deviceId"].as_str(), v["cert"].as_str().and_then(crate::unhex), v["key"].as_str().and_then(crate::unhex)) {
                return Ok(Identity { device_id: id.to_string(), cert_der: c, key_der: k });
            }
        }
    }
    let id = generate()?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let text = serde_json::json!({"deviceId": id.device_id, "cert": crate::hex(&id.cert_der), "key": crate::hex(&id.key_der)}).to_string();
    write_private(&path, text.as_bytes())?;
    Ok(id)
}

pub fn write_private(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
        f.write_all(data).map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// A certificate's public key (SubjectPublicKeyInfo, DER) and common name.
pub fn cert_info(der: &[u8]) -> Option<(Vec<u8>, String)> {
    let (_, c) = x509_parser::parse_x509_certificate(der).ok()?;
    let spki = c.public_key().raw.to_vec();
    let cn = c.subject().iter_common_name().next().and_then(|a| a.as_str().ok()).unwrap_or("").to_string();
    Some((spki, cn))
}

/// The 8 characters both devices show while pairing (KDE Connect's PairingHandler::verificationKey): SHA-256 of the
/// two public keys, the larger first, then the pairing request's timestamp; uppercase hex.
pub fn verification_key(our_cert: &[u8], their_cert: &[u8], timestamp: i64) -> Option<String> {
    let (mut a, _) = cert_info(our_cert)?;
    let (mut b, _) = cert_info(their_cert)?;
    if a < b { std::mem::swap(&mut a, &mut b); }
    let mut h = Sha256::new();
    h.update(&a);
    h.update(&b);
    h.update(timestamp.to_string().as_bytes());
    Some(crate::hex(&h.finalize())[..8].to_uppercase())
}

/// KDE Connect's rule for device ids.
pub fn valid_device_id(id: &str) -> bool {
    (32..=38).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// KDE Connect's rule for names shown to people: no quotes or punctuation it filters, at most 32 characters.
pub fn filter_name(s: &str) -> String {
    s.chars().filter(|c| !"\"',;:.!?()[]<>".contains(*c) && !c.is_control()).take(32).collect::<String>().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_is_what_kde_connect_expects() {
        let id = generate().unwrap();
        assert!(valid_device_id(&id.device_id));
        let (spki, cn) = cert_info(&id.cert_der).unwrap();
        assert_eq!(cn, id.device_id);
        assert!(!spki.is_empty());
        let other = generate().unwrap();
        let k1 = verification_key(&id.cert_der, &other.cert_der, 1_700_000_000).unwrap();
        let k2 = verification_key(&other.cert_der, &id.cert_der, 1_700_000_000).unwrap();
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 8);
        assert_ne!(k1, verification_key(&id.cert_der, &other.cert_der, 1_700_000_001).unwrap());
    }

    #[test]
    fn names_and_ids() {
        assert_eq!(filter_name("Ana's \"Pixel\" 9!"), "Anas Pixel 9");
        assert!(valid_device_id("_3bd8e4ef_4e4f_4eb1_92b8_1fe3c5a6f9d1"));
        assert!(!valid_device_id("short"));
        assert!(!valid_device_id("abcdefabcdefabcdefabcdefabcdef/x"));
    }

    #[test]
    fn keeps_its_identity() {
        let dir = std::env::temp_dir().join(format!("connect-id-{}", crate::random_hex(4)));
        let a = load_or_create(&dir).unwrap();
        let b = load_or_create(&dir).unwrap();
        assert_eq!(a.device_id, b.device_id);
        assert_eq!(a.cert_der, b.cert_der);
        let _ = std::fs::remove_dir_all(dir);
    }
}
