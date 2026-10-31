//! The Hearth Mod Organizer driven through its command interface, on fake games with small synthesized mods.

use super::*;
use std::collections::BTreeMap;
use std::io::Write;

fn put(p: &Path, b: &[u8]) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, b).unwrap(); }
fn read(p: &Path) -> String { std::fs::read_to_string(p).unwrap_or_else(|_| "<missing>".into()) }
fn esp(flags: u32, masters: &[&str]) -> Vec<u8> { plugins::tests::esp(flags, masters, "test") }

/// Every file (with its bytes) and folder under `dir`: equal trees are byte-identical.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    fn walk(d: &Path, root: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().to_string();
            let md = std::fs::symlink_metadata(&p).unwrap();
            if md.file_type().is_symlink() { out.insert(format!("{rel} ->"), std::fs::read_link(&p).unwrap().to_string_lossy().as_bytes().to_vec()); }
            else if md.is_dir() { out.insert(format!("{rel}/"), Vec::new()); walk(&p, root, out); }
            else { out.insert(rel, std::fs::read(&p).unwrap()); }
        }
    }
    walk(dir, dir, &mut out);
    out
}

fn make_zip(p: &Path, files: &[(&str, &[u8])]) {
    let mut z = zip::ZipWriter::new(std::fs::File::create(p).unwrap());
    for (n, b) in files { z.start_file(n.to_string(), zip::write::SimpleFileOptions::default()).unwrap(); z.write_all(b).unwrap(); }
    z.finish().unwrap();
}

struct Fx { t: tempfile::TempDir, ctx: Ctx, game: PathBuf, pfx: PathBuf }

/// A fake Skyrim Special Edition: its folder, its Proton folder, and Hearth's data folder.
fn skyrim() -> Fx {
    let t = tempfile::tempdir().unwrap();
    let game = t.path().join("Steam/steamapps/common/Skyrim Special Edition");
    put(&game.join("Data/Skyrim.esm"), &esp(1, &[]));
    put(&game.join("Data/Update.esm"), &esp(1, &["Skyrim.esm"]));
    put(&game.join("Data/Textures/sky.dds"), b"game sky");
    put(&game.join("SkyrimSE.exe"), b"exe");
    put(&game.join("skse64_loader.exe"), b"skse");
    let pfx = t.path().join("Steam/steamapps/compatdata/489830/pfx");
    let user = pfx.join("drive_c/users/steamuser");
    put(&user.join("AppData/Local/Skyrim Special Edition/Plugins.txt"), b"the game's own plugins");
    put(&user.join("Documents/My Games/Skyrim Special Edition/Skyrim.ini"), b"[General]\r\ngame=1\r\n");
    put(&user.join("Documents/My Games/Skyrim Special Edition/Saves/game.ess"), b"game save");
    let g = GameEntry { id: "steam:489830".into(), title: "Skyrim Special Edition".into(), source: "steam".into(), store: "steam".into(), install: Some(game.clone()), prefix: Some(pfx.clone()),
                        steam_app: Some(489830), kind: "game".into(), ..Default::default() };
    let ctx = Ctx { data: t.path().join("data"), games: vec![g], exe: PathBuf::from("/usr/bin/hearth-mods") };
    Fx { t, ctx, game, pfx }
}

fn cmd(ctx: &Ctx, a: &[&str]) -> Value {
    let args: Vec<String> = a.iter().map(|s| s.to_string()).collect();
    command(ctx, &args).unwrap_or_else(|e| panic!("{a:?}: {e}"))
}
fn names(v: &Value, key: &str) -> Vec<String> { v[key].as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap().to_string()).collect() }

#[test]
fn instance_mods_conflicts_plugins_and_a_clean_link_deploy() {
    let fx = skyrim();
    let ctx = &fx.ctx;
    let v = cmd(ctx, &["create", "steam:489830"]);
    assert_eq!(v["created"], "skyrimse");
    let dir = ctx.base().join("skyrimse");
    let ini = Ini::read(&dir.join("ModOrganizer.ini"));
    assert_eq!(ini.get("General", "gameName").as_deref(), Some("Skyrim Special Edition"));
    assert_eq!(mo2::win_to_linux(&ini.get("General", "gamePath").unwrap(), None).as_deref(), Some(fx.game.as_path()));
    assert!(dir.join("profiles/Default/modlist.txt").is_file() && dir.join("overwrite").is_dir() && dir.join("downloads").is_dir());
    let entry = util::read_json(&ctx.modlists().join("org-skyrimse.json"));
    assert!(read(Path::new(entry["launch"].as_str().unwrap())).contains("--organizer \"skyrimse\" run"));
    assert_eq!(cmd(ctx, &["create", "steam:489830"])["created"], "skyrimse", "one instance per game");

    // A zip with a wrapping folder and a Data folder; its texture's case differs from the game's.
    let z = fx.t.path().join("Cool Mod.zip");
    make_zip(&z, &[("Cool Mod 1.0/Data/textures/SKY.dds", b"cool sky"), ("Cool Mod 1.0/Data/Cool.esp", &esp(0, &["Skyrim.esm"])), ("Cool Mod 1.0/readme.txt", b"read me")]);
    let v = cmd(ctx, &["skyrimse", "install", z.to_str().unwrap()]);
    assert_eq!(v["installed"], "Cool Mod");
    assert!(dir.join("mods/Cool Mod/Cool.esp").is_file() && dir.join("mods/Cool Mod/textures/SKY.dds").is_file());
    assert!(Ini::read(&dir.join("mods/Cool Mod/meta.ini")).get("General", "installationFile").unwrap() == "Cool Mod.zip");
    // A folder with a plugin that needs a master nobody has.
    let f = fx.t.path().join("Patch");
    put(&f.join("textures/sky.dds"), b"patch sky");
    put(&f.join("Patch.esp"), &esp(0, &["Cool.esp", "Missing.esm"]));
    let v = cmd(ctx, &["skyrimse", "install", f.to_str().unwrap(), "--name", "Patch Mod"]);
    assert_eq!(v["installed"], "Patch Mod");
    let s = &v["summary"];
    assert_eq!(names(s, "mods"), vec!["Cool Mod", "Patch Mod", "Overwrite"]);
    assert_eq!(s["mods"][0]["conflicts"]["losesTo"], json!(["Patch Mod"]));
    assert_eq!(s["mods"][1]["conflicts"]["winsOver"], json!(["Cool Mod"]));
    assert_eq!(read(&dir.join("profiles/Default/modlist.txt")), format!("{}\r\n+Patch Mod\r\n+Cool Mod\r\n", mo2::HEADER));
    // Plugins: the game's masters always on; new ones off until turned on.
    let pl = names(s, "plugins");
    assert_eq!(pl, vec!["Skyrim.esm", "Update.esm", "Cool.esp", "Patch.esp"]);
    cmd(ctx, &["skyrimse", "plugin-enable", "Cool.esp"]);
    let v = cmd(ctx, &["skyrimse", "plugin-enable", "patch.esp"]);
    let p = &v["summary"]["plugins"][3];
    assert_eq!((p["active"].as_bool(), p["mod"].as_str(), p["loadIndex"].as_str()), (Some(true), Some("Patch Mod"), Some("03")));
    assert_eq!(p["missing"], json!([{"master": "Missing.esm", "why": "missing"}]));
    assert!(v["summary"]["warnings"].as_array().unwrap().iter().any(|w| w.as_str().unwrap().contains("masters")));
    assert_eq!(read(&dir.join("profiles/Default/plugins.txt")), format!("{}\r\n*Cool.esp\r\n*Patch.esp\r\n", mo2::HEADER));
    // Per-mod conflict files.
    let c = cmd(ctx, &["skyrimse", "conflicts", "Patch Mod"]);
    assert!(c["files"].as_array().unwrap().iter().any(|x| x["path"] == "Textures/sky.dds" && x["status"] == "wins"));
    // Priority: Cool Mod to the top wins the texture; off and on again.
    cmd(ctx, &["skyrimse", "move", "Patch Mod", "0"]);
    let v = cmd(ctx, &["skyrimse", "disable", "Cool Mod"]);
    assert_eq!(v["summary"]["mods"][1]["enabled"], false);
    let v = cmd(ctx, &["skyrimse", "enable", "Cool Mod"]);
    assert_eq!(names(&v["summary"], "mods"), vec!["Patch Mod", "Cool Mod", "Overwrite"]);
    // Local inis and saves for the profile.
    cmd(ctx, &["skyrimse", "profile-settings", "--local-inis", "on", "--local-saves", "on"]);
    assert_eq!(read(&dir.join("profiles/Default/Skyrim.ini")), "[General]\r\ngame=1\r\n", "the profile starts from the game's ini");
    put(&dir.join("profiles/Default/Skyrim.ini"), b"[General]\r\nprofile=1\r\n");
    put(&dir.join("profiles/Default/saves/profile.ess"), b"profile save");

    // Deploy with links: the game folder gets the winners in its own spelling.
    let (game0, pfx0) = (tree(&fx.game), tree(&fx.pfx));
    let v = cmd(ctx, &["skyrimse", "deploy", "--links"]);
    assert_eq!(v["deployed"]["mode"], "hardlink", "{v}");
    assert_eq!(read(&fx.game.join("Data/Textures/sky.dds")), "cool sky");
    assert!(fx.game.join("Data/Patch.esp").is_file() && !fx.game.join("Data/textures").exists());
    let user = fx.pfx.join("drive_c/users/steamuser");
    assert!(read(&user.join("AppData/Local/Skyrim Special Edition/Plugins.txt")).contains("*Patch.esp"));
    assert_eq!(read(&user.join("Documents/My Games/Skyrim Special Edition/Skyrim.ini")), "[General]\r\nprofile=1\r\n");
    assert_eq!(read(&user.join("Documents/My Games/Skyrim Special Edition/Saves/profile.ess")), "profile save");
    // While it's deployed the game writes: a new file in Data, its ini, a save.
    put(&fx.game.join("Data/SKSE/Plugins/new.log"), b"log");
    put(&user.join("Documents/My Games/Skyrim Special Edition/Skyrim.ini"), b"[General]\r\nprofile=2\r\n");
    put(&user.join("Documents/My Games/Skyrim Special Edition/Saves/new.ess"), b"new save");
    let v = cmd(ctx, &["skyrimse", "undeploy"]);
    assert_eq!(v["result"]["undeployed"], true, "{v}");
    assert_eq!(tree(&fx.game), game0, "the game folder is byte-for-byte as it was");
    assert_eq!(tree(&fx.pfx), pfx0, "the Proton folder too");
    assert_eq!(read(&dir.join("overwrite/SKSE/Plugins/new.log")), "log", "what the game wrote went to overwrite");
    assert_eq!(read(&dir.join("profiles/Default/Skyrim.ini")), "[General]\r\nprofile=2\r\n", "the profile keeps the game's ini changes");
    assert_eq!(read(&dir.join("profiles/Default/saves/new.ess")), "new save");
    assert!(cmd(ctx, &["skyrimse", "status"])["status"].is_null());
    // Overwrite now has a file: it's a source, shown last.
    let s = cmd(ctx, &["skyrimse"]);
    assert_eq!(s["mods"][2]["files"], 1);

    // Deployed by hand and left (a crash): put back at sign-in.
    cmd(ctx, &["skyrimse", "deploy", "--links"]);
    assert_eq!(read(&fx.game.join("Data/Textures/sky.dds")), "cool sky");
    let r = cmd(ctx, &["recover"]);
    assert_eq!(r["recovered"].as_array().unwrap().len(), 1, "{r}");
    assert_eq!(tree(&fx.game), game0);
    assert_eq!(tree(&fx.pfx), pfx0);

    // Profiles: a new empty one, switched to (written to ModOrganizer.ini as MO2 does).
    let v = cmd(ctx, &["skyrimse", "profile-create", "Survival"]);
    assert_eq!(v["created"], "Survival");
    let v = cmd(ctx, &["skyrimse", "profile", "Survival"]);
    assert_eq!(v["summary"]["instance"]["profile"], "Survival");
    assert!(v["summary"]["mods"].as_array().unwrap().iter().all(|m| m["kind"] == "overwrite" || m["enabled"] == false));
    assert_eq!(Ini::read(&dir.join("ModOrganizer.ini")).get("General", "selected_profile").as_deref(), Some("Survival"));
    let v = cmd(ctx, &["skyrimse", "profile-create", "Copy", "--from", "Default"]);
    assert!(v["summary"]["profiles"].as_array().unwrap().len() == 3);
    assert_eq!(read(&dir.join("profiles/Copy/modlist.txt")), read(&dir.join("profiles/Default/modlist.txt")));
    // Hiding a file and removing a mod.
    cmd(ctx, &["skyrimse", "profile", "Default"]);
    cmd(ctx, &["skyrimse", "hide", "Patch Mod", "textures/sky.dds"]);
    assert!(dir.join("mods/Patch Mod/textures/sky.dds.mohidden").is_file());
    let c = cmd(ctx, &["skyrimse", "conflicts"]);
    assert_eq!(c["conflicts"][0]["hidden"], 1);
    cmd(ctx, &["skyrimse", "unhide", "Patch Mod", "textures/sky.dds.mohidden"]);
    assert!(dir.join("mods/Patch Mod/textures/sky.dds").is_file());
    let v = cmd(ctx, &["skyrimse", "remove", "Patch Mod"]);
    assert_eq!(names(&v["summary"], "mods"), vec!["Cool Mod", "Overwrite"]);
    assert!(!dir.join("mods/Patch Mod").exists() && !read(&dir.join("profiles/Copy/modlist.txt")).contains("Patch Mod"));
    // Executables: the script extender first.
    let ex = cmd(ctx, &["skyrimse", "executables"]);
    assert_eq!(ex["executables"][0]["title"], "SKSE");
    assert_eq!(ex["executables"][0]["extender"], true);
    assert!(command(ctx, &["skyrimse".into(), "nonsense".into()]).is_err());
}

#[test]
fn fomod_and_bain_installs() {
    let fx = skyrim();
    let ctx = &fx.ctx;
    cmd(ctx, &["create", "steam:489830"]);
    let w = fx.t.path().join("Weathers");
    for (p, b) in [("00 Core/Textures/a.dds", "core"), ("readme.txt", "r"), ("opt/Bright.esp", "bright"), ("opt/Dark.esp", "dark"), ("opt/a.esp", "a"), ("opt/z.esp", "z"), ("opt/bright.ini", "ini"), ("opt/stars/s1.dds", "s1")] { put(&w.join(p), b.as_bytes()); }
    put(&w.join("fomod/ModuleConfig.xml"), fomod::tests::CONFIG.as_bytes());
    let v = cmd(ctx, &["skyrimse", "install", w.to_str().unwrap()]);
    assert_eq!(v["needs"], "fomod", "{v}");
    assert_eq!(v["name"], "Test Weathers");
    assert_eq!(v["fomod"]["steps"].as_array().unwrap().len(), 2);
    assert_eq!(v["evaluation"]["choices"], json!([[[1], []], [[0]]]));
    let token = v["token"].as_str().unwrap().to_string();
    // The choices evaluated without installing (what the screens do on each change).
    let e = cmd(ctx, &["skyrimse", "fomod-eval", &token, "--choices", "[[[0],[]]]"]);
    assert_eq!(e["evaluation"]["steps"][1]["visible"], false);
    // A wrong choice is refused; the right ones install.
    assert!(command(ctx, &["skyrimse".into(), "install".into(), format!("token:{token}"), "--choices".into(), "[[[],[]]]".into()]).is_err());
    let v = cmd(ctx, &["skyrimse", "install", &format!("token:{token}"), "--choices", "[[[0],[1]],[]]"]);
    assert_eq!(v["installed"], "Test Weathers", "{v}");
    let m = ctx.base().join("skyrimse/mods/Test Weathers");
    assert_eq!(read(&m.join("TestWeathers.esp")), "bright");
    assert_eq!(read(&m.join("TestWeathers.ini")), "ini");
    assert_eq!(read(&m.join("Textures/a.dds")), "core");
    assert!(!m.join("z.esp").exists() && !m.join("opt").exists());
    assert!(!ctx.base().join("skyrimse/.hearth/staging").join(&token).exists(), "the unpacked copy is cleaned up");

    // BAIN: numbered parts, 00 by default.
    let b = fx.t.path().join("Bain Pack");
    put(&b.join("00 Core/meshes/a.nif"), b"a");
    put(&b.join("01 Option/textures/b.dds"), b"b");
    put(&b.join("02 Other/textures/c.dds"), b"c");
    put(&b.join("readme.txt"), b"r");
    let v = cmd(ctx, &["skyrimse", "install", b.to_str().unwrap()]);
    assert_eq!(v["needs"], "bain");
    assert_eq!(v["packages"], json!([{"name": "00 Core", "default": true}, {"name": "01 Option", "default": false}, {"name": "02 Other", "default": false}]));
    let t = v["token"].as_str().unwrap().to_string();
    let v = cmd(ctx, &["skyrimse", "install", &t, "--choices", "[\"00 Core\", 2]", "--off"]);
    assert_eq!((v["installed"].as_str(), v["enabled"].as_bool()), (Some("Bain Pack"), Some(false)));
    let m = ctx.base().join("skyrimse/mods/Bain Pack");
    assert!(m.join("meshes/a.nif").is_file() && m.join("textures/c.dds").is_file() && !m.join("textures/b.dds").exists());
    // The same name again gets a number.
    let again = fx.t.path().join("again.zip");
    make_zip(&again, &[("meshes/x.nif", b"x")]);
    assert_eq!(cmd(ctx, &["skyrimse", "install", again.to_str().unwrap(), "--name", "Bain Pack"])["installed"], "Bain Pack (2)");
}

#[test]
fn data_folders_in_archives() {
    let t = tempfile::tempdir().unwrap();
    let k = Kind::by_id("skyrimse");
    let r = t.path().join("a");
    put(&r.join("skse64_2_02/skse64_loader.exe"), b"x");
    put(&r.join("skse64_2_02/skse64.dll"), b"x");
    put(&r.join("skse64_2_02/readme.txt"), b"x");
    put(&r.join("skse64_2_02/Data/Scripts/x.pex"), b"x");
    let (d, notes) = data_root(&r, &k);
    assert_eq!(d, r.join("skse64_2_02/Data"));
    assert!(notes[0].contains("skse64_loader.exe") && !notes[0].contains("readme"), "{notes:?}");
    let r2 = t.path().join("b");
    put(&r2.join("Wrapper/Inner/meshes/m.nif"), b"x");
    assert_eq!(data_root(&r2, &k).0, r2.join("Wrapper/Inner"));
    let r3 = t.path().join("c");
    put(&r3.join("Mod.esp"), b"x");
    assert_eq!(data_root(&r3, &k), (r3.clone(), Vec::new()));
    let r4 = t.path().join("d");
    put(&r4.join("Docs/x.txt"), b"x");
    put(&r4.join("Other/y.txt"), b"x");
    assert!(!data_root(&r4, &k).1.is_empty(), "not a Data folder: said so");
    assert!(bain_packages(&r4).is_none());
}

#[test]
fn an_mo2_instance_is_imported_and_its_launcher_chosen() {
    let fx = skyrim();
    let ctx = &fx.ctx;
    let list = fx.t.path().join("Modlists/My List");
    let ini = format!("[General]\r\ngameName=Skyrim Special Edition\r\ngamePath={}\r\nselected_profile=@ByteArray(Main)\r\n[customExecutables]\r\nsize=2\r\n1\\title=SKSE\r\n1\\binary={}\r\n1\\arguments=\r\n2\\title=xEdit\r\n2\\binary={}\r\n2\\arguments=-quickautoclean\r\n",
        mo2::qt_bytearray(&mo2::linux_to_win(&fx.game)), mo2::linux_to_win(&fx.game.join("skse64_loader.exe")).replace('\\', "/"), mo2::linux_to_win(&list.join("tools/SSEEdit.exe")).replace('\\', "/"));
    put(&list.join("ModOrganizer.ini"), ini.as_bytes());
    put(&list.join("tools/SSEEdit.exe"), b"xedit");
    put(&list.join("mods/A/textures/a.dds"), b"a");
    put(&list.join("mods/B/b.esp"), &esp(0, &["Skyrim.esm"]));
    put(&list.join("mods/Things_separator/meta.ini"), b"[General]\r\n");
    put(&list.join("mods/Gone Later/x.txt"), b"x");
    put(&list.join("profiles/Main/modlist.txt"), b"# This file was automatically generated by Mod Organizer.\r\n+B\r\n-Things_separator\r\n+A\r\n*DLC: Dawnguard\r\n+Not Here\r\n");
    put(&list.join("profiles/Main/plugins.txt"), b"*b.esp\r\n");
    put(&list.join("profiles/Main/settings.ini"), b"[General]\r\nLocalSettings=true\r\nLocalSaves=false\r\n");
    put(&list.join("profiles/Other/modlist.txt"), b"");
    // The list's own library entry, as wabbajack.rs writes it.
    let ml = ctx.modlists();
    put(&ml.join("w1.desktop"), b"[Desktop Entry]\nExec=umu-run ModOrganizer.exe\n");
    util::write_json(&ml.join("w1.json"), &json!({"id": "w1", "title": "My List", "launch": ml.join("w1.desktop"), "folder": list})).unwrap();
    let v = cmd(ctx, &["import", list.to_str().unwrap(), "--wabbajack", "w1"]);
    let id = v["imported"].as_str().unwrap().to_string();
    assert_eq!(id, "my-list");
    let s = &v["summary"];
    assert_eq!(s["instance"]["profile"], "Main");
    assert_eq!(s["instance"]["game"], "skyrimse");
    assert_eq!(names(s, "mods"), vec!["DLC: Dawnguard", "A", "Things", "B", "Gone Later", "Overwrite"], "lowest first; unlisted mods last; lines for missing mods dropped");
    let kinds: Vec<&str> = s["mods"].as_array().unwrap().iter().map(|m| m["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, vec!["unmanaged", "mod", "separator", "mod", "mod", "overwrite"]);
    assert_eq!(s["mods"][4]["enabled"], false);
    assert!(s["plugins"].as_array().unwrap().iter().any(|p| p["name"] == "b.esp" && p["active"] == true && p["mod"] == "B"));
    let ex = s["executables"].as_array().unwrap();
    assert_eq!(ex.len(), 3, "{ex:?}");
    assert_eq!((ex[0]["title"].as_str(), ex[0]["from"].as_str()), (Some("SKSE"), Some("game")), "MO2's SKSE entry is the same program");
    assert!(ex.iter().any(|e| e["title"] == "xEdit" && e["args"] == "-quickautoclean" && e["present"] == true));
    assert_eq!(s["profiles"].as_array().unwrap().iter().find(|p| p["name"] == "Main").unwrap()["localInis"], true);
    // Play runs it natively; Mod Organizer 2 is the fallback, per list.
    let launch = || util::read_json(&ml.join("w1.json"))["launch"].as_str().unwrap().to_string();
    assert!(launch().ends_with("w1-hearth.desktop"));
    assert!(read(Path::new(&launch())).contains("--organizer \"my-list\" run"));
    assert_eq!(cmd(ctx, &["my-list", "launcher", "mo2"])["launcher"], "mo2");
    assert!(launch().ends_with("w1.desktop"));
    cmd(ctx, &["my-list", "launcher", "hearth"]);
    assert!(launch().ends_with("w1-hearth.desktop"));
    // Importing again keeps the same instance (and the choice).
    cmd(ctx, &["my-list", "launcher", "mo2"]);
    let v = cmd(ctx, &["import", list.to_str().unwrap(), "--wabbajack", "w1"]);
    assert_eq!(v["imported"], "my-list");
    assert_eq!(v["summary"]["instance"]["launcher"], "mo2");
    assert_eq!(cmd(ctx, &["instances"])["instances"].as_array().unwrap().len(), 1);
    // An instance made by Hearth can't fall back on MO2.
    let other = fx.t.path().join("Plain");
    put(&other.join("ModOrganizer.ini"), format!("[General]\r\ngameName=Skyrim Special Edition\r\ngamePath={}\r\n", mo2::qt_bytearray(&mo2::linux_to_win(&fx.game))).as_bytes());
    let v = cmd(ctx, &["import", other.to_str().unwrap()]);
    assert!(command(ctx, &[v["imported"].as_str().unwrap().into(), "launcher".into(), "mo2".into()]).is_err());
    assert!(other.join("profiles/Default/modlist.txt").is_file(), "an instance without profiles gets Default");
}

/// A game Hearth doesn't know: mods go into its folder; a mod's folders join the game's own whatever their case.
#[test]
fn other_games_and_case_insensitive_links() {
    let t = tempfile::tempdir().unwrap();
    let game = t.path().join("Some Game");
    put(&game.join("BepInEx/config/x.cfg"), b"cfg");
    put(&game.join("game.exe"), b"exe");
    let g = GameEntry { id: "manual:some".into(), title: "Some Game".into(), install: Some(game.clone()), kind: "game".into(), ..Default::default() };
    let ctx = Ctx { data: t.path().join("data"), games: vec![g], exe: PathBuf::from("hearth-mods") };
    let v = cmd(&ctx, &["create", "manual:some"]);
    assert_eq!(v["summary"]["instance"]["game"], "generic");
    let m = t.path().join("m");
    put(&m.join("bepinex/plugins/Mod.dll"), b"dll");
    put(&m.join("BEPINEX/CONFIG/X.CFG"), b"mod cfg");
    cmd(&ctx, &["some-game", "install", m.to_str().unwrap(), "--name", "Mod"]);
    let before = tree(&game);
    let v = cmd(&ctx, &["some-game", "deploy", "--links"]);
    assert_eq!(v["deployed"]["mode"], "hardlink");
    assert_eq!(read(&game.join("BepInEx/plugins/Mod.dll")), "dll");
    assert_eq!(read(&game.join("BepInEx/config/x.cfg")), "mod cfg", "the game's own file, replaced in its own spelling");
    assert!(!game.join("bepinex").exists() && !game.join("BEPINEX").exists());
    // A change while deployed by hand deploys again.
    let v = cmd(&ctx, &["some-game", "disable", "Mod"]);
    assert!(v.get("deployProblem").is_none(), "{v}");
    assert_eq!(read(&game.join("BepInEx/config/x.cfg")), "cfg");
    cmd(&ctx, &["some-game", "undeploy"]);
    assert_eq!(tree(&game), before);
}

/// fuse-overlayfs for real (where it's installed): the game folder isn't touched at all, and what the game writes
/// lands in overwrite.
#[test]
#[cfg_attr(not(has_fuse_overlayfs), ignore)]
fn overlay_end_to_end() {
    let fx = skyrim();
    let ctx = &fx.ctx;
    cmd(ctx, &["create", "steam:489830"]);
    let f = fx.t.path().join("M");
    put(&f.join("textures/SKY.dds"), b"mod sky");
    put(&f.join("meshes/new.nif"), b"nif");
    cmd(ctx, &["skyrimse", "install", f.to_str().unwrap(), "--name", "M"]);
    let game0 = tree(&fx.game);
    let v = cmd(ctx, &["skyrimse", "deploy"]);
    assert_eq!(v["deployed"]["mode"], "overlay", "the overlay didn't mount: {v}");
    assert_eq!(v["deployed"]["mounted"], true);
    let data = fx.game.join("Data");
    assert_eq!(read(&data.join("Textures/sky.dds")), "mod sky");
    assert_eq!(read(&data.join("meshes/new.nif")), "nif");
    assert!(data.join("Skyrim.esm").is_file());
    put(&data.join("SKSE/new.log"), b"log");
    let v = cmd(ctx, &["skyrimse", "undeploy"]);
    assert_eq!(v["result"]["undeployed"], true, "{v}");
    assert!(!crate::mods::overlay::mounted(Some(&data)));
    assert_eq!(tree(&fx.game), game0, "the game folder was never changed");
    assert_eq!(read(&ctx.base().join("skyrimse/overwrite/SKSE/new.log")), "log");
}
