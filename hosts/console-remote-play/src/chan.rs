//! The stream's `message` and `control` channels (JSON text), as xbox-xcloud-player speaks them (MIT).

use serde_json::{json, Value};

/// The data channels the offer opens: name, ordered, protocol.
pub const CHANNELS: [(&str, bool, &str); 4] = [("input", true, "1.0"), ("chat", true, "chatV1"), ("control", true, "controlV1"), ("message", true, "messageV1")];

/// `message`, when it opens.
pub fn handshake() -> String {
    json!({"type": "Handshake", "version": "messageV1", "id": crate::uuid(), "cv": ""}).to_string()
}

fn message(target: &str, content: Value) -> String {
    json!({"type": "Message", "content": content.to_string(), "id": crate::uuid(), "target": target, "cv": ""}).to_string()
}

/// What follows the Xbox's HandshakeAck: the system UI it may show, this screen, no touch.
pub fn after_ack(w: u32, h: u32) -> Vec<String> {
    vec![
        message("/streaming/systemUi/configuration", json!({"version": [0, 1, 0], "systemUis": [10, 19, 31, 27, 32, -41]})),
        message("/streaming/properties/clientappinstallidchanged", json!({"clientAppInstallId": crate::uuid()})),
        message("/streaming/characteristics/orientationchanged", json!({"orientation": 0})),
        message("/streaming/characteristics/touchinputenabledchanged", json!({"touchInputEnabled": false})),
        message("/streaming/characteristics/clientdevicecapabilities", json!({})),
        message("/streaming/characteristics/dimensionschanged", json!({
            "horizontal": w, "vertical": h, "preferredWidth": w, "preferredHeight": h,
            "safeAreaLeft": 0, "safeAreaTop": 0, "safeAreaRight": w, "safeAreaBottom": h, "supportsCustomResolution": true
        })),
    ]
}

/// A message from the Xbox: is it the HandshakeAck? (And a transaction to answer, by id.)
pub fn is_ack(text: &str) -> bool {
    serde_json::from_str::<Value>(text).map(|v| v["type"] == "HandshakeAck").unwrap_or(false)
}

/// `control`, after the HandshakeAck: authorization, then gamepad 0 is here.
pub fn control_start() -> Vec<String> {
    vec![
        json!({"message": "authorizationRequest", "accessKey": "4BDB3609-C1F1-4195-9B37-FEFF45DA8B8E"}).to_string(),
        json!({"message": "gamepadChanged", "gamepadIndex": 0, "wasAdded": true}).to_string(),
    ]
}

/// `control`: ask for a whole picture again (after the picture broke up).
pub fn keyframe() -> String {
    json!({"message": "videoKeyframeRequested", "ifrRequested": true}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_handshake_and_after() {
        let h: Value = serde_json::from_str(&handshake()).unwrap();
        assert_eq!(h["version"], "messageV1");
        assert!(is_ack("{\"type\":\"HandshakeAck\"}"));
        assert!(!is_ack("{\"type\":\"Message\"}"));
        let a = after_ack(1280, 720);
        assert_eq!(a.len(), 6);
        let dim: Value = serde_json::from_str(&a[5]).unwrap();
        let c: Value = serde_json::from_str(dim["content"].as_str().unwrap()).unwrap();
        assert_eq!(c["preferredWidth"], 1280);
        assert!(control_start()[1].contains("gamepadChanged"));
    }
}
