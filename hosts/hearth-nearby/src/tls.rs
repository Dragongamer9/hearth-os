//! Nearby Share's identity: a self-signed certificate (ECDSA P-256), made once and kept in config_dir(). Its
//! fingerprint (SHA-256 of the certificate, uppercase hex: what LocalSend uses) is how other devices tell Hearth
//! apart and how they remember it. The same certificate is the HTTPS server's and the client certificate Hearth shows
//! when it sends (LocalSend 1.18+ only believes a sender's fingerprint when the sender proves it that way).
//!
//! Certificates aren't checked against any authority (there is none on a home network): the server accepts any
//! client certificate and only notes its fingerprint, and the client pins the receiver's announced fingerprint when
//! it knows it.

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{DigitallySignedStruct, DistinguishedName, SignatureScheme};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;

pub struct Identity {
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>,
    pub fingerprint: String,
}

pub fn provider() -> Arc<CryptoProvider> { Arc::new(rustls::crypto::ring::default_provider()) }

/// SHA-256 of a certificate (DER), uppercase hex: LocalSend's fingerprint.
pub fn fingerprint(cert_der: &[u8]) -> String { crate::hex(&Sha256::digest(cert_der)).to_uppercase() }

/// A new self-signed certificate (`cn` as its name).
pub fn generate(cn: &str) -> Result<Identity, String> {
    let key = rcgen::KeyPair::generate().map_err(|e| e.to_string())?;
    let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).map_err(|e| e.to_string())?;
    let mut dn = rcgen::DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, cn);
    params.distinguished_name = dn;
    params.not_before = rcgen::date_time_ymd(2024, 1, 1);
    params.not_after = rcgen::date_time_ymd(2049, 12, 31);
    let cert = params.self_signed(&key).map_err(|e| e.to_string())?;
    let cert_der = cert.der().to_vec();
    Ok(Identity { fingerprint: fingerprint(&cert_der), cert_der, key_der: key.serialize_der() })
}

/// The certificate in `dir`/identity.json, made the first time.
pub fn load_or_create(dir: &Path) -> Result<Identity, String> {
    let path = dir.join("identity.json");
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let (Some(c), Some(k)) = (v["cert"].as_str().and_then(crate::unhex), v["key"].as_str().and_then(crate::unhex)) {
                return Ok(Identity { fingerprint: fingerprint(&c), cert_der: c, key_der: k });
            }
        }
    }
    let id = generate("Hearth Nearby Share")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let text = serde_json::json!({"cert": crate::hex(&id.cert_der), "key": crate::hex(&id.key_der)}).to_string();
    write_private(&path, text.as_bytes())?;
    Ok(id)
}

/// A file only this user can read (the private key is in it).
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

fn key(id: &Identity) -> PrivateKeyDer<'static> { PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(id.key_der.clone())) }

/// Accepts any client certificate (or none): the server notes its fingerprint, nothing more.
#[derive(Debug)]
struct AnyClient(Arc<CryptoProvider>);

impl ClientCertVerifier for AnyClient {
    fn offer_client_auth(&self) -> bool { true }
    fn client_auth_mandatory(&self) -> bool { false }
    fn root_hint_subjects(&self) -> &[DistinguishedName] { &[] }
    fn verify_client_cert(&self, _: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: UnixTime) -> Result<ClientCertVerified, rustls::Error> {
        Ok(ClientCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(m, c, d, &self.0.signature_verification_algorithms)
    }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(m, c, d, &self.0.signature_verification_algorithms)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> { self.0.signature_verification_algorithms.supported_schemes() }
}

/// Accepts the receiver's certificate if it has the fingerprint the receiver announced (when known).
#[derive(Debug)]
struct Pinned { provider: Arc<CryptoProvider>, expect: Option<String> }

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(&self, end: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: &ServerName<'_>, _: &[u8], _: UnixTime) -> Result<ServerCertVerified, rustls::Error> {
        if let Some(e) = &self.expect {
            if !e.is_empty() && !fingerprint(end.as_ref()).eq_ignore_ascii_case(e) {
                return Err(rustls::Error::General("that device's certificate isn't the one it announced".into()));
            }
        }
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(m, c, d, &self.provider.signature_verification_algorithms)
    }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(m, c, d, &self.provider.signature_verification_algorithms)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> { self.provider.signature_verification_algorithms.supported_schemes() }
}

pub fn server_config(id: &Identity) -> Result<Arc<rustls::ServerConfig>, String> {
    let p = provider();
    let mut cfg = rustls::ServerConfig::builder_with_provider(p.clone())
        .with_safe_default_protocol_versions().map_err(|e| e.to_string())?
        .with_client_cert_verifier(Arc::new(AnyClient(p)))
        .with_single_cert(vec![CertificateDer::from(id.cert_der.clone())], key(id)).map_err(|e| e.to_string())?;
    cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(cfg))
}

/// For sending: shows our certificate, and pins the receiver's when `expect` is its announced fingerprint.
pub fn client_config(id: &Identity, expect: Option<String>) -> Result<rustls::ClientConfig, String> {
    let p = provider();
    let mut cfg = rustls::ClientConfig::builder_with_provider(p.clone())
        .with_safe_default_protocol_versions().map_err(|e| e.to_string())?
        .dangerous().with_custom_certificate_verifier(Arc::new(Pinned { provider: p, expect }))
        .with_client_auth_cert(vec![CertificateDer::from(id.cert_der.clone())], key(id)).map_err(|e| e.to_string())?;
    cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_and_keeps_one_identity() {
        let dir = std::env::temp_dir().join(format!("nearby-id-{}", crate::random_id(4)));
        let a = load_or_create(&dir).unwrap();
        let b = load_or_create(&dir).unwrap();
        assert_eq!(a.fingerprint, b.fingerprint);
        assert_eq!(a.fingerprint.len(), 64);
        assert_eq!(a.fingerprint, a.fingerprint.to_uppercase());
        assert!(server_config(&a).is_ok());
        assert!(client_config(&a, Some(a.fingerprint.clone())).is_ok());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(dir.join("identity.json")).unwrap().permissions().mode() & 0o777, 0o600);
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
