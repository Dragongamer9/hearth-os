//! Controller layouts across your devices (Hearth Input): a device lists its own layouts (each game's custom layout
//! and the global one) to the devices it's paired with, and the UI asks every paired device for theirs, to use one
//! here. Pull only: nothing is ever written on another device, and what comes in is checked again here
//! (hearth-core input_map::validate, strict: keys, clicks and scrolling only) before Hearth saves it.
//!
//! Between devices: input.layouts. From the UI: input.peers → {devices: [{peer, name, layouts: [{game, name, layout}]}]}.

use crate::node::Node;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

/// This device's own layouts, as another device sees them (only ones that pass the checks).
pub fn own() -> Vec<Value> {
    hearth_core::input_map::own_layouts_in(&hearth_core::input_map::dir()).into_iter()
        .filter(|l| hearth_core::input_map::validate(&l["layout"]).is_ok()).collect()
}

/// What another device sent: its layouts that pass the checks here too, at most 200.
pub fn checked(list: &Value) -> Vec<Value> {
    list.as_array().into_iter().flatten().take(200).filter_map(|l| {
        let game = l["game"].as_str().filter(|g| !g.is_empty() && g.len() <= 200)?;
        hearth_core::input_map::validate(&l["layout"]).ok()?;
        let layout = hearth_core::input_map::migrate(&l["layout"]).ok()?;
        let name: String = l["name"].as_str().unwrap_or("Layout").chars().take(60).collect();
        Some(json!({"game": game, "name": name, "layout": layout}))
    }).collect()
}

impl Node {
    pub async fn inputs_answer(self: Arc<Self>, _peer: &str, op: &str, _req: &Value) -> Result<Value, String> {
        match op {
            "input.layouts" => Ok(json!({"name": self.name(), "layouts": own()})),
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    pub async fn inputs_command(self: &Arc<Self>, cmd: &str, _a: &Value) -> Result<Value, String> {
        match cmd {
            "input.peers" => {
                let peers: Vec<(String, String)> = crate::lock(&self.peers).list.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
                let mut set = tokio::task::JoinSet::new();
                for (id, name) in peers {
                    let n = self.clone();
                    set.spawn(async move { let r = tokio::time::timeout(Duration::from_secs(6), n.request(&id, &json!({"op": "input.layouts"}))).await; (id, name, r) });
                }
                let mut devices = Vec::new();
                while let Some(r) = set.join_next().await {
                    let Ok((id, name, r)) = r else { continue };
                    if let Ok(Ok(v)) = r { devices.push(json!({"peer": id, "name": name, "layouts": checked(&v["layouts"])})); }
                }
                Ok(json!({"devices": devices}))
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_layouts_that_pass_the_checks_come_in() {
        let list = json!([
            {"game": "steam:1", "name": "Mine", "layout": {"buttons": {"a": "key:SPACE"}}},
            {"game": "steam:2", "name": "Evil", "layout": {"buttons": {"a": "run:/bin/sh"}}},
            {"game": "", "layout": {}},
            {"game": "steam:3", "name": "Old", "layout": {"version": 1, "buttons": {"lb": "shift"}}}
        ]);
        let got = checked(&list);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0]["game"], "steam:1");
        assert_eq!(got[1]["layout"]["version"], 2);   // migrated
        assert!(checked(&json!("nope")).is_empty());
    }
}
