//! LocalSend protocol v2's messages (https://github.com/localsend/protocol). Unknown fields are ignored and missing
//! optional ones get defaults, as the official apps do: older apps (v2.0) and newer ones (v2.2) both work.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Who a device is: the multicast announcement, /register's body and answer, and prepare-upload's "info".
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    #[serde(default)]
    pub alias: String,
    #[serde(default)]
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_type: Option<String>,
    #[serde(default)]
    pub fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(default)]
    pub download: bool,
    /// v2.0/2.1 multicast: true for an announcement, false for an answer. v2.2 leaves it out (always an announcement).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announce: Option<bool>,
    /// v1's name for the same.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announcement: Option<bool>,
}

impl Info {
    /// Is this multicast message an announcement (to be answered) rather than an answer?
    pub fn is_announcement(&self) -> bool { self.announce != Some(false) && self.announcement != Some(false) }
    pub fn https(&self) -> bool { self.protocol.as_deref() != Some("http") }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileMeta {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub file_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl FileMeta {
    /// LocalSend sends a text message as one text "file" with the text in its preview.
    pub fn is_message(&self) -> bool { self.file_type.starts_with("text/") && self.preview.is_some() }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PrepareUpload {
    pub info: Info,
    pub files: HashMap<String, FileMeta>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrepareResponse {
    pub session_id: String,
    #[serde(default)]
    pub files: HashMap<String, String>,
}

/// What a device type looks like to people.
pub fn type_label(t: Option<&str>) -> &'static str {
    match t.unwrap_or("desktop") {
        "mobile" => "Phone or tablet",
        "web" => "Web browser",
        "headless" => "Command line",
        "server" => "Server",
        _ => "Computer",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_protocol_examples() {
        // From LocalSend's protocol README (v2.2 and v2.0 announcements, register, prepare-upload).
        let a: Info = serde_json::from_str(r#"{"alias":"Nice Orange","version":"2.0","deviceModel":"Samsung","deviceType":"mobile","fingerprint":"random string","port":53317,"protocol":"https","download":true,"announce":true}"#).unwrap();
        assert!(a.is_announcement() && a.https());
        assert_eq!(a.port, Some(53317));
        let b: Info = serde_json::from_str(r#"{"alias":"Secret Banana","version":"2.2","fingerprint":"x","port":53317,"protocol":"http"}"#).unwrap();
        assert!(b.is_announcement() && !b.https());
        let c: Info = serde_json::from_str(r#"{"alias":"Secret Banana","version":"2.0","deviceModel":"Windows","deviceType":"desktop","fingerprint":"x","port":53317,"protocol":"https","download":true,"announce":false}"#).unwrap();
        assert!(!c.is_announcement());
        let p: PrepareUpload = serde_json::from_str(r#"{"info":{"alias":"Nice Orange","version":"2.0","deviceModel":"Samsung","deviceType":"mobile","fingerprint":"random string","port":53317,"protocol":"https","download":true},
            "files":{"some file id":{"id":"some file id","fileName":"my image.png","size":324242,"fileType":"image/jpeg","sha256":"*sha256 hash*","preview":"*preview data*","metadata":{"modified":"2021-01-01T12:34:56Z","accessed":"2021-01-01T12:34:56Z"}},
            "another file id":{"id":"another file id","fileName":"another image.jpg","size":1234,"fileType":"image/jpeg","sha256":"*sha256 hash*","preview":"*preview data*"}}}"#).unwrap();
        assert_eq!(p.files.len(), 2);
        assert_eq!(p.files["some file id"].size, 324242);
        let r: PrepareResponse = serde_json::from_str(r#"{"sessionId":"mySessionId","files":{"someFileId":"someFileToken"}}"#).unwrap();
        assert_eq!(r.session_id, "mySessionId");
    }

    #[test]
    fn writes_what_localsend_reads() {
        let i = Info { alias: "Hearth".into(), version: "2.1".into(), device_model: Some("Hearth OS".into()), device_type: Some("desktop".into()),
            fingerprint: "AB".into(), port: Some(53317), protocol: Some("https".into()), download: false, announce: Some(true), announcement: None };
        let v: serde_json::Value = serde_json::to_value(&i).unwrap();
        assert_eq!(v["deviceModel"], "Hearth OS");
        assert_eq!(v["deviceType"], "desktop");
        assert_eq!(v["announce"], true);
        assert!(v.get("announcement").is_none());
        let m = FileMeta { id: "1".into(), file_name: "a.txt".into(), size: 3, file_type: "text/plain".into(), preview: Some("hey".into()), ..Default::default() };
        assert!(m.is_message());
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["fileName"], "a.txt");
        assert!(v.get("sha256").is_none());
    }
}
