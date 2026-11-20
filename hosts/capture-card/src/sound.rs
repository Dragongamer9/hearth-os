//! The card's sound: it shows up as a USB microphone (a PipeWire source). Picked by itself: the source on the same
//! USB device as the card's picture (its /sys path), else one named like the card. Asked through pactl (PipeWire's
//! pulse side) with a time limit, only when the card is found or Settings opens.

use crate::card::Card;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    /// PipeWire's node name (what pipewiresrc's target-object takes).
    pub name: String,
    pub description: String,
    pub sysfs: String,
    pub bus: String,
    pub product: String,
}

/// Every microphone-like source (not the speakers' monitors).
pub fn sources() -> Vec<Source> {
    if let Ok(v) = std::env::var("HEARTH_CAPTURE_FAKE_SOURCES") {
        // CI: "name|description;name|description".
        return v.split(';').filter(|s| !s.is_empty()).map(|s| {
            let mut p = s.splitn(2, '|');
            let name = p.next().unwrap_or("").to_string();
            Source { description: p.next().unwrap_or(&name).to_string(), name, sysfs: String::new(), bus: "usb".into(), product: String::new() }
        }).collect();
    }
    let out = crate::output(&["pactl", "--format=json", "list", "sources"], 4).unwrap_or_default();
    parse(&out)
}

pub fn parse(json: &str) -> Vec<Source> {
    let v: Value = serde_json::from_str(json).unwrap_or(Value::Null);
    v.as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|s| {
            let name = s["name"].as_str()?.to_string();
            if name.ends_with(".monitor") || s["monitor_source"].as_str().map(|m| !m.is_empty() && m == name).unwrap_or(false) {
                return None;
            }
            let p = &s["properties"];
            Some(Source {
                description: s["description"].as_str().unwrap_or(&name).to_string(),
                sysfs: p["sysfs.path"].as_str().or(p["device.bus_path"].as_str()).unwrap_or("").to_string(),
                bus: p["device.bus"].as_str().unwrap_or("").to_string(),
                product: p["device.product.name"].as_str().unwrap_or("").to_string(),
                name,
            })
        })
        .collect()
}

/// The card's own microphone among the sources.
pub fn for_card<'a>(card: &Card, list: &'a [Source]) -> Option<&'a Source> {
    if let Some(f) = &card.fake_sound {
        return list.iter().find(|s| s.name == *f);
    }
    if !card.usb_port.is_empty() {
        let a = format!("/{}/", card.usb_port);
        let b = format!("/{}:", card.usb_port);
        if let Some(s) = list.iter().find(|s| s.sysfs.contains(&a) || s.sysfs.contains(&b)) {
            return Some(s);
        }
    }
    let base = card.chip.split(" (").next().unwrap_or("").to_lowercase();
    list.iter().find(|s| s.bus == "usb" && !base.is_empty() && (s.product.to_lowercase() == base || s.description.to_lowercase().contains(&base)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_cards_microphone() {
        let j = r#"[
          {"name":"alsa_output.pci.monitor","description":"Monitor of Speakers","properties":{}},
          {"name":"alsa_input.pci-0000_00_1f.3.analog-stereo","description":"Built-in Audio","properties":{"device.bus":"pci","sysfs.path":"/devices/pci0000:00/0000:00:1f.3/sound/card0"}},
          {"name":"alsa_input.usb-MACROSILICON_USB3_Video-02.analog-stereo","description":"USB3 Video Analog Stereo","properties":{"device.bus":"usb","device.product.name":"USB3 Video","sysfs.path":"/devices/pci0000:00/0000:00:14.0/usb4/4-1/4-1:1.2/sound/card2"}}
        ]"#;
        let l = parse(j);
        assert_eq!(l.len(), 2, "the monitor isn't a microphone");
        let card = crate::card::Card {
            dev: "/dev/video0".into(),
            id: String::new(),
            name: "Monster 4K".into(),
            chip: "USB3 Video (MS2130)".into(),
            speed: 5000,
            usb_port: "4-1".into(),
            modes: vec![],
            fake_picture: None,
            fake_sound: None,
        };
        assert_eq!(for_card(&card, &l).map(|s| s.name.as_str()), Some("alsa_input.usb-MACROSILICON_USB3_Video-02.analog-stereo"));
        // Another port: found by its name instead.
        let c2 = crate::card::Card { usb_port: "9-9".into(), ..card };
        assert!(for_card(&c2, &l).is_some());
        assert!(parse("not json").is_empty());
    }
}
