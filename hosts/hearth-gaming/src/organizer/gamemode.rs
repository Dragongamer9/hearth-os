//! The organizer for Game Mode's mod screens (ui/index.html `HMA`, through Hearth Mods' worker: service.rs and
//! hearth-core hearth_mods.rs CALLS). Game Mode knows games, not instances: a library game with an organizer instance
//! (made with `create`, or a Wabbajack list's own game) is answered from its instance, in the shapes the screens
//! already draw:
//! - `organizer_game {id}` → `{profile, profiles: [{id, name}], plugins: [{name, enabled, master, light, mod, loadIndex,
//!   missing}] | null, instance}`, or null when the game has no instance (the screens then show Hearth Mods' own list);
//! - `organizer_set_profile {id, profile}`, `organizer_plugin_toggle {id, plugin, on}`, `organizer_plugin_move {id,
//!   plugin, by}`;
//! - a download installed into the game (`install_download`) that has a FOMOD installer answers `{fomod: {token, name,
//!   steps: [{name, groups: [{name, type, options: [{name, description, type, image}]}]}]}}`, and
//!   `organizer_fomod_finish {token, choices}` installs it with the choices (`[[[option indexes] per group] per step]`).
//! - `mods_of`, `toggle`, `move_mod` and `remove_mod` for such a game answer from the instance's mod list, in Hearth
//!   Mods' summary shape (so the Installed tab shows the organizer's mods).

use super::{vfs, Ctx, Instance};
use serde_json::{json, Value};
use std::path::Path;

/// The game's organizer instance, if it has one.
pub fn for_game(ctx: &Ctx, game_id: &str) -> Option<Instance> {
    if game_id.is_empty() { return None; }
    let r = ctx.recs().into_iter().find(|r| r.library_id == game_id)?;
    Instance::load(r).ok()
}

fn s(a: &Value, k: &str) -> String { a[k].as_str().unwrap_or("").to_string() }

/// The instance's mods in Hearth Mods' summary shape (GameMods::summary), for the Installed tab.
pub fn simple_summary(inst: &Instance, game_id: &str) -> Value {
    let sum = inst.summary();
    let mods: Vec<Value> = sum["mods"].as_array().cloned().unwrap_or_default().into_iter().filter(|m| m["kind"] == "mod").enumerate().map(|(i, m)| {
        let c = &m["conflicts"];
        json!({"id": m["id"], "name": m["name"], "enabled": m["enabled"], "order": i, "target": "game", "sub": inst.kind.data, "root": inst.data_target(),
               "files": m["files"].as_u64().unwrap_or(0), "size": 0, "added": 0, "source": m["installedFrom"], "overriddenBy": c["losesTo"].as_array().cloned().unwrap_or_default(),
               "winning": c["winning"].as_u64().unwrap_or(0), "targetLabel": "Hearth Mod Organizer"})
    }).collect();
    json!({"game": game_id, "title": inst.rec.title, "mods": mods, "targets": [], "deployed": 0, "originals": 0, "folder": inst.dir, "deploy": "organizer",
           "modes": [{"id": "organizer", "label": "Hearth Mod Organizer (laid over the game while it runs)"}], "nexus": "", "overlay": Value::Null,
           "organizer": {"id": inst.rec.id, "profile": inst.profile, "deployed": sum["deployed"], "warnings": sum["warnings"]}})
}

fn after_change(inst: &Instance, game_id: &str) -> Result<Value, String> {
    let inst = Instance::load(inst.rec.clone())?;
    let problem = inst.refresh_deploy().err();
    Ok(json!({"report": {"placed": 0, "removed": 0, "restored": 0, "keptChanged": [], "problems": problem.into_iter().collect::<Vec<_>>()}, "summary": simple_summary(&inst, game_id)}))
}

/// Hearth Mods' own calls for a game with an instance: answered by the organizer (None: not such a game or call).
pub fn simple_call(ctx: &Ctx, name: &str, a: &Value) -> Option<Result<Value, String>> {
    if !matches!(name, "mods_of" | "toggle" | "move_mod" | "remove_mod") { return None; }
    let id = s(a, "id");
    let inst = for_game(ctx, &id)?;
    Some((|| -> Result<Value, String> {
        if name == "mods_of" { return Ok(simple_summary(&inst, &id)); }
        inst.guard()?;
        let m = s(a, "m");
        match name {
            "toggle" => inst.set_enabled(&m, a["on"].as_bool().unwrap_or(false))?,
            "move_mod" => {
                let l = inst.modlist();
                let cur = l.iter().position(|x| x.name == m).ok_or("That mod isn't in the list anymore.")? as i64;
                inst.move_mod(&m, cur + a["by"].as_i64().unwrap_or(0))?;
            }
            _ => inst.remove_mod(&m)?,
        }
        after_change(&inst, &id)
    })())
}

/// A FOMOD answer in the shape Game Mode's installer screen draws.
fn fomod_for_gamemode(inst: &Instance, r: &Value) -> Value {
    let f = &r["fomod"];
    let ev = &r["evaluation"];
    let steps: Vec<Value> = f["steps"].as_array().cloned().unwrap_or_default().iter().enumerate().map(|(si, st)| {
        let groups: Vec<Value> = st["groups"].as_array().cloned().unwrap_or_default().iter().enumerate().map(|(gi, g)| {
            let types = &ev["steps"][si]["groups"][gi]["types"];
            let options: Vec<Value> = g["plugins"].as_array().cloned().unwrap_or_default().iter().enumerate().map(|(pi, p)| {
                let img = p["imagePath"].as_str().filter(|x| !x.is_empty()).map(|x| format!("file://{x}"));
                json!({"name": p["name"], "description": p["description"], "type": types[pi].as_str().or(p["defaultType"].as_str()).unwrap_or("Optional"), "image": img})
            }).collect();
            json!({"name": g["name"], "type": g["type"], "options": options})
        }).collect();
        json!({"name": st["name"], "groups": groups})
    }).collect();
    json!({"fomod": {"token": format!("{}:{}", inst.rec.id, r["token"].as_str().unwrap_or("")), "name": r["name"], "steps": steps, "info": f["info"]}})
}

/// A finished download into a game with an instance: installed into the instance (or a FOMOD answer).
pub fn install_file(ctx: &Ctx, game_id: &str, file: &Path, name: &str) -> Option<Result<Value, String>> {
    let inst = for_game(ctx, game_id)?;
    Some((|| -> Result<Value, String> {
        inst.guard()?;
        let r = inst.install(&file.to_string_lossy(), Some(name).filter(|n| !n.is_empty()), None, true)?;
        match r["needs"].as_str() {
            Some("fomod") => Ok(fomod_for_gamemode(&inst, &r)),
            Some("bain") => {
                // BAIN in Game Mode: the default parts (choosing them is in the Mods window).
                let parts: Vec<Value> = r["packages"].as_array().cloned().unwrap_or_default().into_iter().filter(|p| p["default"] == true).map(|p| p["name"].clone()).collect();
                let tok = format!("token:{}", r["token"].as_str().unwrap_or(""));
                inst.install(&tok, r["name"].as_str(), Some(&json!(parts)), true)?;
                after_change(&inst, game_id)
            }
            _ => after_change(&inst, game_id),
        }
    })())
}

/// The organizer calls Game Mode makes.
pub fn call(ctx: &Ctx, name: &str, a: &Value) -> Result<Value, String> {
    match name {
        "organizer_game" => {
            let Some(inst) = for_game(ctx, &s(a, "id")) else { return Ok(Value::Null) };
            let sum = inst.summary();
            let plugins = if inst.kind.has_plugins() {
                json!(sum["plugins"].as_array().cloned().unwrap_or_default().iter().map(|p| json!({"name": p["name"], "enabled": p["active"], "master": p["master"], "light": p["light"], "implicit": p["implicit"],
                    "mod": p["mod"], "loadIndex": p["loadIndex"], "missing": p["missing"]})).collect::<Vec<_>>())
            } else { Value::Null };
            Ok(json!({"profile": inst.profile, "profiles": inst.profiles().into_iter().map(|p| json!({"id": p, "name": p})).collect::<Vec<_>>(), "plugins": plugins,
                      "instance": inst.public(), "deployed": vfs::status(&inst), "warnings": sum["warnings"]}))
        }
        "organizer_set_profile" | "organizer_plugin_toggle" | "organizer_plugin_move" => {
            let id = s(a, "id");
            let mut inst = for_game(ctx, &id).ok_or("This game has no Hearth Mod Organizer instance.")?;
            inst.guard()?;
            match name {
                "organizer_set_profile" => inst.select_profile(&s(a, "profile"))?,
                "organizer_plugin_toggle" => inst.set_plugin(&s(a, "plugin"), a["on"].as_bool().unwrap_or(false))?,
                _ => {
                    let plugin = s(a, "plugin");
                    let s0 = inst.current_plugins();
                    let cur = s0.iter().position(|p| p.name.eq_ignore_ascii_case(&plugin)).ok_or("That plugin isn't there anymore.")? as i64;
                    inst.move_plugin(&plugin, cur + a["by"].as_i64().unwrap_or(0))?;
                }
            }
            after_change(&inst, &id)
        }
        "organizer_fomod_finish" => {
            let t = s(a, "token");
            let (iid, tok) = t.rsplit_once(':').ok_or("That installer was put away: install the mod again.")?;
            let inst = ctx.open(iid)?;
            inst.guard()?;
            let choices = a.get("choices").cloned().unwrap_or(Value::Null);
            inst.install(&format!("token:{tok}"), None, Some(&choices), true)?;
            let game_id = inst.rec.library_id.clone();
            after_change(&inst, &game_id)
        }
        _ => Err(format!("Hearth Mods doesn't know \"{name}\".")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::GameEntry;
    use std::path::PathBuf;

    #[test]
    fn game_mode_calls() {
        let t = tempfile::tempdir().unwrap();
        let game = t.path().join("Skyrim Special Edition");
        let put = |p: &Path, b: &[u8]| { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, b).unwrap(); };
        put(&game.join("Data/Skyrim.esm"), &super::super::plugins::tests::esp(1, &[], "x"));
        put(&game.join("SkyrimSE.exe"), b"x");
        let g = GameEntry { id: "steam:489830".into(), title: "Skyrim Special Edition".into(), install: Some(game.clone()), steam_app: Some(489830), kind: "game".into(), ..Default::default() };
        let ctx = Ctx { data: t.path().join("data"), games: vec![g.clone()], exe: PathBuf::from("hearth-mods") };
        assert!(call(&ctx, "organizer_game", &json!({"id": "steam:489830"})).unwrap().is_null(), "no instance yet");
        assert!(simple_call(&ctx, "mods_of", &json!({"id": "steam:489830"})).is_none());
        super::super::create(&ctx, &g, None, None).unwrap();
        let m = t.path().join("A.esp folder");
        put(&m.join("A.esp"), &super::super::plugins::tests::esp(0, &["Skyrim.esm"], "x"));
        let r = install_file(&ctx, "steam:489830", &m, "Mod A").unwrap().unwrap();
        assert_eq!(r["summary"]["mods"][0]["name"], "Mod A");
        assert_eq!(r["summary"]["mods"][0]["enabled"], true);
        let o = call(&ctx, "organizer_game", &json!({"id": "steam:489830"})).unwrap();
        assert_eq!(o["profile"], "Default");
        assert_eq!(o["plugins"][1]["name"], "A.esp");
        assert_eq!(o["plugins"][1]["enabled"], false);
        call(&ctx, "organizer_plugin_toggle", &json!({"id": "steam:489830", "plugin": "A.esp", "on": true})).unwrap();
        assert_eq!(call(&ctx, "organizer_game", &json!({"id": "steam:489830"})).unwrap()["plugins"][1]["enabled"], true);
        let r = simple_call(&ctx, "toggle", &json!({"id": "steam:489830", "m": "Mod A", "on": false})).unwrap().unwrap();
        assert_eq!(r["summary"]["mods"][0]["enabled"], false);
        // A FOMOD from a download: Game Mode's shape, then finished with its choices.
        let w = t.path().join("Weathers");
        for (p, b) in [("00 Core/Textures/a.dds", "core"), ("readme.txt", "r"), ("opt/Bright.esp", "bright"), ("opt/Dark.esp", "dark"), ("opt/a.esp", "a"), ("opt/z.esp", "z"), ("opt/bright.ini", "ini"), ("opt/stars/s1.dds", "s1")] { put(&w.join(p), b.as_bytes()); }
        put(&w.join("fomod/ModuleConfig.xml"), super::super::fomod::tests::CONFIG.as_bytes());
        let r = install_file(&ctx, "steam:489830", &w, "").unwrap().unwrap();
        let f = &r["fomod"];
        assert!(f["token"].as_str().unwrap().starts_with("skyrimse:"));
        assert_eq!(f["steps"][0]["groups"][0]["options"][1]["type"], "Recommended");
        let r = call(&ctx, "organizer_fomod_finish", &json!({"token": f["token"], "choices": [[[1], []], [[0]]]})).unwrap();
        assert!(r["summary"]["mods"].as_array().unwrap().iter().any(|m| m["name"] == "Test Weathers"));
        assert_eq!(std::fs::read_to_string(ctx.base().join("skyrimse/mods/Test Weathers/TestWeathers.esp")).unwrap(), "dark");
    }
}
