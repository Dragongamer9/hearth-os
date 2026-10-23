//! KDE Connect packets: one JSON object per line, {"id": <ms>, "type": "kdeconnect.…", "body": {...}}, plus
//! "payloadSize" and "payloadTransferInfo": {"port": N} when a file follows on a separate connection.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PROTOCOL_VERSION: i64 = 8;
pub const UDP_PORT: u16 = 1716;
pub const MIN_TCP_PORT: u16 = 1716;
pub const MAX_TCP_PORT: u16 = 1764;
pub const MIN_PAYLOAD_PORT: u16 = 1739;
pub const MAX_PAYLOAD_PORT: u16 = 1764;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Packet {
    #[serde(default, deserialize_with = "any_id")]
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub body: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_size: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_transfer_info: Option<Value>,
}

/// The packet id: a number (ms since 1970) from every current app; some older ones sent it as a string.
fn any_id<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    let v = Value::deserialize(d)?;
    Ok(v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)).or_else(|| v.as_str().and_then(|s| s.parse().ok())).unwrap_or(0))
}

impl Packet {
    pub fn new(kind: &str, body: Value) -> Packet {
        Packet { id: crate::now_ms(), kind: kind.into(), body, payload_size: None, payload_transfer_info: None }
    }
    /// The line that goes on the wire.
    pub fn line(&self) -> String { let mut s = serde_json::to_string(self).unwrap_or_default(); s.push('\n'); s }
    pub fn parse(line: &str) -> Option<Packet> {
        let p: Packet = serde_json::from_str(line.trim()).ok()?;
        if !p.body.is_object() && !p.body.is_null() { return None; }
        Some(p)
    }
    pub fn str(&self, k: &str) -> Option<&str> { self.body.get(k).and_then(|v| v.as_str()) }
    pub fn bool(&self, k: &str) -> bool { self.body.get(k).and_then(|v| v.as_bool()).unwrap_or(false) }
    pub fn int(&self, k: &str) -> Option<i64> { self.body.get(k).and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)).or_else(|| v.as_str().and_then(|s| s.parse().ok()))) }
    pub fn float(&self, k: &str) -> Option<f64> { self.body.get(k).and_then(|v| v.as_f64()) }
    pub fn has(&self, k: &str) -> bool { self.body.get(k).is_some() }
    pub fn payload_port(&self) -> Option<u16> {
        self.payload_transfer_info.as_ref().and_then(|i| i.get("port")).and_then(|p| p.as_i64()).and_then(|p| u16::try_from(p).ok())
    }
}

/// Packets Hearth Connect understands (what a phone may send it).
pub const INCOMING: &[&str] = &[
    "kdeconnect.ping", "kdeconnect.battery", "kdeconnect.share.request", "kdeconnect.share.request.update",
    "kdeconnect.notification", "kdeconnect.clipboard", "kdeconnect.clipboard.connect",
    "kdeconnect.mousepad.request", "kdeconnect.mousepad.keyboardstate", "kdeconnect.mpris", "kdeconnect.mpris.request",
    "kdeconnect.findmyphone.request",
];

/// Packets Hearth Connect sends.
pub const OUTGOING: &[&str] = &[
    "kdeconnect.ping", "kdeconnect.battery", "kdeconnect.share.request", "kdeconnect.notification.request",
    "kdeconnect.notification.reply", "kdeconnect.notification.action", "kdeconnect.clipboard", "kdeconnect.clipboard.connect",
    "kdeconnect.mpris", "kdeconnect.mpris.request", "kdeconnect.findmyphone.request", "kdeconnect.mousepad.echo",
    "kdeconnect.mousepad.keyboardstate",
];

pub struct Me<'a> {
    pub device_id: &'a str,
    pub name: &'a str,
    pub device_type: &'a str,
}

/// The identity packet. `tcp_port` for the UDP announcement; `target` (their id) for the first line on a TCP
/// connection we open (kdeconnect-kde 24.12+ checks it; older versions ignore it). The full set of fields is always
/// sent, which every version accepts.
pub fn identity(me: &Me, tcp_port: Option<u16>, target: Option<&str>) -> Packet {
    let mut body = json!({
        "deviceId": me.device_id, "deviceName": me.name, "deviceType": me.device_type, "protocolVersion": PROTOCOL_VERSION,
        "incomingCapabilities": INCOMING, "outgoingCapabilities": OUTGOING,
    });
    if let Some(p) = tcp_port { body["tcpPort"] = json!(p); }
    if let Some(t) = target { body["targetDeviceId"] = json!(t); body["targetProtocolVersion"] = json!(PROTOCOL_VERSION); }
    Packet::new("kdeconnect.identity", body)
}

/// A desktop or a laptop (it has a battery).
pub fn device_type() -> &'static str {
    let laptop = std::fs::read_dir("/sys/class/power_supply").map(|rd| rd.flatten().any(|e| {
        std::fs::read_to_string(e.path().join("type")).map(|t| t.trim() == "Battery").unwrap_or(false)
    })).unwrap_or(false);
    if laptop { "laptop" } else { "desktop" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_packets() {
        let me = Me { device_id: "0123456789abcdef0123456789abcdef", name: "Hearth", device_type: "desktop" };
        let p = identity(&me, Some(1716), None);
        assert_eq!(p.kind, "kdeconnect.identity");
        assert_eq!(p.body["protocolVersion"], 8);
        assert_eq!(p.body["tcpPort"], 1716);
        assert!(p.body.get("targetDeviceId").is_none());
        let line = p.line();
        assert!(line.ends_with('\n') && !line[..line.len() - 1].contains('\n'));
        let back = Packet::parse(&line).unwrap();
        assert_eq!(back.body["deviceId"], me.device_id);
        let c = identity(&me, None, Some("fedcba9876543210fedcba9876543210"));
        assert_eq!(c.body["targetDeviceId"], "fedcba9876543210fedcba9876543210");
        assert_eq!(c.body["targetProtocolVersion"], 8);
    }

    #[test]
    fn reads_phone_packets() {
        let p = Packet::parse(r#"{"id":1700000000000,"type":"kdeconnect.share.request","body":{"filename":"IMG_1.jpg","numberOfFiles":1,"totalPayloadSize":42},"payloadSize":42,"payloadTransferInfo":{"port":1739}}"#).unwrap();
        assert_eq!(p.payload_port(), Some(1739));
        assert_eq!(p.payload_size, Some(42));
        assert_eq!(p.str("filename"), Some("IMG_1.jpg"));
        let b = Packet::parse(r#"{"id":"1700000000000","type":"kdeconnect.battery","body":{"currentCharge":57,"isCharging":true,"thresholdEvent":0}}"#).unwrap();
        assert_eq!(b.id, 1700000000000);
        let b = Packet::parse(r#"{"id":5,"type":"kdeconnect.battery","body":{"currentCharge":57,"isCharging":true,"thresholdEvent":0}}"#).unwrap();
        assert_eq!(b.int("currentCharge"), Some(57));
        assert!(b.bool("isCharging"));
        let m = Packet::parse(r#"{"id":5,"type":"kdeconnect.mousepad.request","body":{"dx":3.5,"dy":-2.25}}"#).unwrap();
        assert_eq!(m.float("dx"), Some(3.5));
        assert!(Packet::parse("not json").is_none());
    }
}
