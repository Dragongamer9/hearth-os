//! Linux: everything through udisks2 on the system bus (zbus). udisks asks polkit, and polkit asks for your
//! password when an action needs it (internal drives; USB drives and SD cards usually don't). Disks never runs
//! mkfs, parted or mount itself.

use crate::model::{Drive, Partition, MIB};
use crate::safety::Opts;
use crate::udisks;
use crate::Progress;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value as V};

const DEST: &str = "org.freedesktop.UDisks2";
const ROOT: &str = "/org/freedesktop/UDisks2";
const MANAGER: &str = "/org/freedesktop/UDisks2/Manager";

fn bus() -> Result<Connection, String> {
    Connection::system().map_err(|e| format!("Couldn't reach the system's disk service (udisks2): {e}"))
}

/// A udisks error in plain words.
fn plain(e: zbus::Error) -> String {
    let text = e.to_string();
    let low = text.to_lowercase();
    if low.contains("notauthorized") || low.contains("not authorized") {
        if low.contains("dismissed") || low.contains("cancel") { return "Cancelled. Nothing was changed.".into(); }
        return "This needs your password, and no password prompt could be shown. Open Disks in Desktop Mode and try again.".into();
    }
    if low.contains("cancelled") || low.contains("dismissed") { return "Cancelled. Nothing was changed.".into(); }
    if low.contains("busy") { return "It's in use. Close apps using files on it (and Steam, if games run from it), then try again.".into(); }
    if low.contains("serviceunknown") || low.contains("was not provided") { return "The system's disk service (udisks2) isn't installed or didn't start.".into(); }
    // udisks' own message after its error name: "GDBus.Error:org.freedesktop.UDisks2.Error.Failed: Error ..."
    let msg = text.rsplit_once(": ").map(|(_, m)| m.to_string()).unwrap_or(text);
    format!("It didn't work: {}", msg.trim())
}

fn call<B, R>(c: &Connection, path: &str, iface: &str, method: &str, body: &B) -> Result<R, String>
where
    B: serde::Serialize + zbus::zvariant::DynamicType,
    R: serde::de::DeserializeOwned + zbus::zvariant::Type,
{
    let msg = c.call_method(Some(DEST), path, Some(iface), method, body).map_err(plain)?;
    msg.body().deserialize::<R>().map_err(|e| format!("The disk service answered strangely: {e}"))
}

fn call0<B>(c: &Connection, path: &str, iface: &str, method: &str, body: &B) -> Result<(), String>
where
    B: serde::Serialize + zbus::zvariant::DynamicType,
{
    c.call_method(Some(DEST), path, Some(iface), method, body).map(|_| ()).map_err(plain)
}

/// Options for a udisks call: password prompts allowed.
fn opts() -> HashMap<&'static str, V<'static>> {
    let mut m = HashMap::new();
    m.insert("auth.no_user_interaction", V::from(false));
    m
}

/// A D-Bus value as JSON: byte strings (ay, like Block.Device) become text, arrays of them lists of text.
pub fn to_json(v: &V) -> Value {
    match v {
        V::U8(x) => json!(x),
        V::Bool(x) => json!(x),
        V::I16(x) => json!(x),
        V::U16(x) => json!(x),
        V::I32(x) => json!(x),
        V::U32(x) => json!(x),
        V::I64(x) => json!(x),
        V::U64(x) => json!(x),
        V::F64(x) => json!(x),
        V::Str(s) => json!(s.as_str()),
        V::Signature(s) => json!(s.as_str()),
        V::ObjectPath(p) => json!(p.as_str()),
        V::Value(b) => to_json(b),
        V::Array(a) => {
            if a.element_signature().as_str() == "y" {
                let bytes: Vec<u8> = a.iter().filter_map(|x| if let V::U8(b) = x { Some(*b) } else { None }).collect();
                json!(String::from_utf8_lossy(&bytes).trim_end_matches('\0'))
            } else {
                Value::Array(a.iter().map(to_json).collect())
            }
        }
        V::Dict(d) => {
            let mut m = Map::new();
            for (k, v) in d.iter() {
                let key = match k { V::Str(s) => s.as_str().to_string(), other => to_json(other).to_string() };
                m.insert(key, to_json(v));
            }
            Value::Object(m)
        }
        V::Structure(s) => Value::Array(s.fields().iter().map(to_json).collect()),
        _ => Value::Null,
    }
}

type Managed = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

/// All of udisks2's objects as JSON (udisks.rs's input).
pub fn objects(c: &Connection) -> Result<Value, String> {
    let m: Managed = call(c, ROOT, "org.freedesktop.DBus.ObjectManager", "GetManagedObjects", &())?;
    let mut out = Map::new();
    for (path, ifaces) in m {
        let mut io = Map::new();
        for (iface, props) in ifaces {
            let mut po = Map::new();
            for (k, v) in props { po.insert(k, to_json(&v)); }
            io.insert(iface, Value::Object(po));
        }
        out.insert(path.as_str().to_string(), Value::Object(io));
    }
    Ok(Value::Object(out))
}

/// (used, free) of what's mounted at `path`.
pub fn usage(path: &str) -> Option<(u64, u64)> {
    let c = std::ffi::CString::new(path).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 { return None; }
    let f = st.f_frsize as u64;
    let total = st.f_blocks as u64 * f;
    Some((total.saturating_sub(st.f_bfree as u64 * f), st.f_bavail as u64 * f))
}

fn has_library(mount: &str, libs: &[std::path::PathBuf]) -> bool {
    let m = std::path::Path::new(mount);
    m.join("steamapps").is_dir() || m.join("SteamLibrary/steamapps").is_dir() || m.join("libraryfolder.vdf").is_file()
        || libs.iter().any(|l| l.starts_with(m))
}

pub fn list() -> Result<Vec<Drive>, String> {
    let c = bus()?;
    let objs = objects(&c)?;
    let home = hearth_core::paths::home();
    let libs: Vec<std::path::PathBuf> = hearth_core::steam::steam_path(&home).map(|s| hearth_core::steam::libraries(&s)).unwrap_or_default();
    // The system's own mounts (/var/home holds Steam's own library) aren't "a Steam library" partition.
    let steam = move |mounts: &[String]| mounts.iter().filter(|m| !crate::model::is_system_mount(m)).any(|m| has_library(m, &libs));
    let ctx = udisks::Ctx { uid: unsafe { libc::getuid() }, steam: &steam, usage: &usage };
    Ok(udisks::parse(&objs, &ctx))
}

/// One object's property, freshly read.
fn prop(c: &Connection, path: &str, iface: &str, name: &str) -> Value {
    objects(c).ok().and_then(|o| o.get(path).and_then(|x| x.get(iface)).and_then(|x| x.get(name)).cloned()).unwrap_or(Value::Null)
}

fn unmount(c: &Connection, p: &Partition) -> Result<(), String> {
    if p.mounted && p.mountable { call0(c, &p.id, udisks::FILESYSTEM, "Unmount", &(opts(),))?; }
    Ok(())
}

fn mount(c: &Connection, path: &str) -> Result<String, String> {
    call::<_, String>(c, path, udisks::FILESYSTEM, "Mount", &(opts(),))
}

/// Unmounts everything on a drive and locks what's unlocked, so it can be formatted or removed.
fn tear_down(c: &Connection, d: &Drive) -> Result<(), String> {
    for p in d.partitions.iter().filter(|p| p.kind != "free") {
        unmount(c, p)?;
        if p.fs == "crypto_LUKS" {
            let clear = prop(c, &p.id, udisks::ENCRYPTED, "CleartextDevice");
            if clear.as_str().map(|s| s != "/").unwrap_or(false) {
                if let Some(cl) = clear.as_str() {
                    if !prop(c, cl, udisks::FILESYSTEM, "MountPoints").as_array().map(|a| a.is_empty()).unwrap_or(true) {
                        call0(c, cl, udisks::FILESYSTEM, "Unmount", &(opts(),))?;
                    }
                }
                call0(c, &p.id, udisks::ENCRYPTED, "Lock", &(opts(),))?;
            }
        }
    }
    Ok(())
}

/// The partition type for a new partition holding `fs` on a `table` (gpt or dos) table.
pub fn part_type(table: &str, fs: &str) -> &'static str {
    let linux = matches!(fs, "ext4" | "btrfs" | "xfs" | "f2fs");
    match (table, linux) {
        ("gpt", true) => crate::model::LINUX_TYPE,
        ("gpt", false) => crate::model::BASIC_DATA_TYPE,
        (_, true) => "0x83",
        (_, false) => if fs == "vfat" { "0x0c" } else { "0x07" },
    }
}

/// Options for making a file system: its label, and the new file system belongs to you (ext4, btrfs).
fn fs_opts(fs: &str, label: &str) -> HashMap<&'static str, V<'static>> {
    let mut m = opts();
    if !label.is_empty() { m.insert("label", V::from(label.to_string())); }
    if matches!(fs, "ext4" | "btrfs" | "xfs") { m.insert("take-ownership", V::from(true)); }
    m.insert("update-partition-type", V::from(true));
    m
}

/// Makes a partition with a file system in free space; returns its block object.
fn create(c: &Connection, d: &Drive, offset: u64, size: u64, fs: &str, label: &str) -> Result<String, String> {
    let table = if d.table.is_empty() { "gpt" } else { d.table.as_str() };
    let mut popts = opts();
    if table == "dos" { popts.insert("partition-type", V::from("primary".to_string())); }
    let path: OwnedObjectPath = call(c, &d.block, udisks::TABLE, "CreatePartitionAndFormat",
        &(offset, size, part_type(table, fs), "", popts, fs, fs_opts(fs, label)))?;
    Ok(path.as_str().to_string())
}

/// Erases the whole drive: a new partition table and one partition with a file system. Returns its block object.
/// MBR (dos) for exFAT, FAT and NTFS, which cameras, consoles and older PCs read; GPT otherwise and for drives
/// over 2 TB.
fn format_drive(c: &Connection, d: &Drive, fs: &str, label: &str, step: &dyn Fn(&str)) -> Result<String, String> {
    tear_down(c, d)?;
    let table = if matches!(fs, "exfat" | "vfat" | "ntfs") && d.size <= 2 * 1024 * 1024 * MIB { "dos" } else { "gpt" };
    step("Erasing the drive");
    let mut o = opts();
    o.insert("tear-down", V::from(true));
    call0(c, &d.block, udisks::BLOCK, "Format", &(table, o))?;
    step("Making the new partition");
    let fresh = Drive { table: table.into(), ..d.clone() };
    create(c, &fresh, MIB, 0, fs, label)
}

fn format_part(c: &Connection, p: &Partition, fs: &str, label: &str) -> Result<(), String> {
    unmount(c, p)?;
    let mut o = fs_opts(fs, label);
    o.insert("tear-down", V::from(true));
    // A file system on the whole drive (no partition table) has no partition type to update.
    if p.number == 0 { o.remove("update-partition-type"); }
    call0(c, &p.id, udisks::BLOCK, "Format", &(fs, o))
}

/// Mounts it, or mounts it again after a change, where it was (udisks picks /run/media/<you>/<label>).
fn remount_if(c: &Connection, path: &str, was: bool) { if was { let _ = mount(c, path); } }

/// "Use for games": the file system (on block object `path`) mounts by itself at /var/mnt/games-<name> on every
/// start (an fstab entry, added through udisks: x-systemd.automount, so an SD card that isn't there doesn't
/// hold up starting), then it's mounted and Hearth's Steam code (hearth-core) adds <mount>/SteamLibrary to Steam.
fn setup_games(c: &Connection, path: &str, step: &dyn Fn(&str)) -> Result<String, String> {
    let objs = objects(c)?;
    let o = objs.get(path).cloned().unwrap_or(Value::Null);
    let block = &o[udisks::BLOCK];
    let uuid = block["IdUUID"].as_str().unwrap_or("").to_string();
    let fs = block["IdType"].as_str().unwrap_or("").to_string();
    let label = block["IdLabel"].as_str().unwrap_or("").to_string();
    if uuid.is_empty() { return Err("It has no file system to use yet.".into()); }
    let configured = block["Configuration"].as_array().map(|a| a.iter().any(|i| i[0] == "fstab")).unwrap_or(false);
    let mounts: Vec<String> = o[udisks::FILESYSTEM]["MountPoints"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
    if !configured {
        step("Setting it to mount by itself");
        let dir = format!("/var/mnt/games-{}", crate::safe_name(&label, &uuid));
        let mut mo = vec!["nofail", "x-systemd.automount", "x-systemd.device-timeout=5s", "x-mount.mkdir", "x-gvfs-show", "x-udisks-auth"].join(",");
        // Windows formats have no owners: say who owns the files.
        if matches!(fs.as_str(), "exfat" | "vfat" | "ntfs") {
            let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
            mo.push_str(&format!(",uid={uid},gid={gid}"));
        }
        let nul = |s: &str| { let mut b = s.as_bytes().to_vec(); b.push(0); V::from(b) };
        let mut item: HashMap<&str, V> = HashMap::new();
        item.insert("fsname", nul(&format!("UUID={uuid}")));
        item.insert("dir", nul(&dir));
        item.insert("type", nul("auto"));
        item.insert("opts", nul(&mo));
        item.insert("freq", V::from(0i32));
        item.insert("passno", V::from(0i32));
        if !mounts.is_empty() { call0(c, path, udisks::FILESYSTEM, "Unmount", &(opts(),))?; }
        call0(c, path, udisks::BLOCK, "AddConfigurationItem", &(("fstab", item), opts()))?;
    }
    step("Mounting it");
    let now: Vec<String> = prop(c, path, udisks::FILESYSTEM, "MountPoints").as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let at = match now.first() { Some(m) => m.clone(), None => mount(c, path)? };
    step("Adding it to Steam");
    let folder = std::path::Path::new(&at).join("SteamLibrary");
    match hearth_core::steam::add_library(&hearth_core::paths::home(), &folder) {
        Ok(restarted) => Ok(if restarted { "Ready for games. Steam was restarted and lists it under Storage.".into() }
                            else { "Ready for games. Steam lists it under Storage; pick it when you install a game.".into() }),
        Err(e) if e.contains("make the games folder") => Err(format!("{e} It belongs to another user: format it for games to use it.")),
        Err(e) => Err(e),
    }
}

pub fn execute(action: &str, d: &Drive, p: Option<&Partition>, o: &Opts, progress: Progress) -> Result<String, String> {
    let c = bus()?;
    let step_text = std::sync::Mutex::new(String::from("Working"));
    let done = AtomicBool::new(false);
    // While it runs, udisks' jobs are watched for a progress bar (when udisks knows how far it is).
    std::thread::scope(|s| {
        s.spawn(|| {
            let Ok(c2) = bus() else { return };
            while !done.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(700));
                if done.load(Ordering::SeqCst) { break; }
                if let Ok(objs) = objects(&c2) {
                    let pct = objs.as_object().into_iter().flatten().filter_map(|(_, o)| o.get("org.freedesktop.UDisks2.Job"))
                        .filter(|j| j["ProgressValid"] == true).filter_map(|j| j["Progress"].as_f64()).next();
                    if let Some(pct) = pct { progress(&step_text.lock().unwrap().clone(), Some(pct)); }
                }
            }
        });
        let step = |t: &str| { *step_text.lock().unwrap() = t.to_string(); progress(t, None); };
        let r = work(&c, action, d, p, o, &step);
        done.store(true, Ordering::SeqCst);
        r
    })
}

fn work(c: &Connection, action: &str, d: &Drive, p: Option<&Partition>, o: &Opts, step: &dyn Fn(&str)) -> Result<String, String> {
    let c = c.clone();
    let r = match (action, p) {
        ("open", Some(p)) => {
            let at = p.mount_points.first().cloned().ok_or("It isn't mounted.")?;
            std::process::Command::new("xdg-open").arg(&at).spawn().map(|_| String::new()).map_err(|e| format!("Couldn't open it: {e}"))
        }
        ("mount", Some(p)) => { step("Mounting"); mount(&c, &p.id).map(|at| format!("Mounted at {at}.")) }
        ("unmount", Some(p)) => { step("Unmounting"); unmount(&c, p).map(|_| "Unmounted.".to_string()) }
        ("eject", None) => {
            step("Letting go of it");
            tear_down(&c, d)?;
            if d.kind == "image" {
                call0(&c, &d.block, udisks::LOOP, "Delete", &(opts(),))?;
            } else if d.can_power_off {
                step("Powering it off");
                call0(&c, &d.id, udisks::DRIVE, "PowerOff", &(opts(),))?;
            } else if d.can_eject {
                call0(&c, &d.id, udisks::DRIVE, "Eject", &(opts(),))?;
            }
            Ok(format!("You can remove {} now.", d.name()))
        }
        ("rename", Some(p)) => {
            step("Renaming");
            let was = p.mounted && matches!(p.fs.as_str(), "vfat" | "exfat" | "ntfs");
            if was { unmount(&c, p)?; }
            let r = call0(&c, &p.id, udisks::FILESYSTEM, "SetLabel", &(o.label.as_str(), opts()));
            remount_if(&c, &p.id, was);
            r.map(|_| "Renamed.".to_string())
        }
        ("check", Some(p)) | ("repair", Some(p)) => {
            let repair = action == "repair";
            let was = p.mounted;
            unmount(&c, p)?;
            step(if repair { "Repairing" } else { "Checking" });
            let ok: Result<bool, String> = call(&c, &p.id, udisks::FILESYSTEM, if repair { "Repair" } else { "Check" }, &(opts(),));
            remount_if(&c, &p.id, was);
            match (ok, repair) {
                (Ok(true), false) => Ok("No problems found.".into()),
                (Ok(false), false) => Ok("It has problems. Choose Repair to fix them.".into()),
                (Ok(true), true) => Ok("Repaired.".into()),
                (Ok(false), true) => Err("It couldn't be fully repaired. Copy off what you can, then format it.".into()),
                (Err(e), _) => Err(e),
            }
        }
        ("format", Some(p)) => {
            step("Formatting");
            format_part(&c, p, &o.fs, &o.label)?;
            if d.removable { let _ = mount(&c, &p.id); }
            Ok("Formatted.".into())
        }
        ("delete", Some(p)) => {
            step("Deleting the partition");
            unmount(&c, p)?;
            let mut po = opts();
            po.insert("tear-down", V::from(true));
            call0(&c, &p.id, udisks::PARTITION, "Delete", &(po,)).map(|_| "Deleted.".to_string())
        }
        ("create", Some(p)) => {
            step("Making the partition");
            let size = if o.size == 0 { 0 } else { o.size };
            let path = create(&c, d, p.offset, size, &o.fs, &o.label)?;
            if d.removable { let _ = mount(&c, &path); }
            Ok("Partition made.".into())
        }
        ("format_drive", None) => {
            let path = format_drive(&c, d, &o.fs, &o.label, step)?;
            if d.removable { step("Mounting it"); let _ = mount(&c, &path); }
            Ok(format!("{} is formatted.", d.name()))
        }
        ("games", target) => {
            let label = if o.label.is_empty() { "Games".to_string() } else { o.label.clone() };
            let path = match target {
                None => format_drive(&c, d, &o.fs, &label, step)?,
                Some(p) if p.kind == "free" => { step("Making the partition"); create(&c, d, p.offset, o.size, &o.fs, &label)? }
                Some(p) if !crate::safety::games_ready("linux", &p.fs) => { step("Formatting"); format_part(&c, p, &o.fs, &label)?; p.id.clone() }
                Some(p) => p.id.clone(),
            };
            setup_games(&c, &path, step)
        }
        _ => Err("That can't be done here.".into()),
    };
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn converts_dbus_values() {
        let dev = V::from(b"/dev/sda1\0".to_vec());
        assert_eq!(to_json(&dev), json!("/dev/sda1"));
        let mounts = V::from(vec![b"/run/media/sam/SD\0".to_vec(), b"/mnt\0".to_vec()]);
        assert_eq!(to_json(&mounts), json!(["/run/media/sam/SD", "/mnt"]));
        assert_eq!(to_json(&V::from(42u64)), json!(42));
        assert_eq!(to_json(&V::from("gpt")), json!("gpt"));
    }
    #[test]
    fn partition_types() {
        assert_eq!(part_type("gpt", "ext4"), crate::model::LINUX_TYPE);
        assert_eq!(part_type("gpt", "exfat"), crate::model::BASIC_DATA_TYPE);
        assert_eq!(part_type("dos", "exfat"), "0x07");
        assert_eq!(part_type("dos", "btrfs"), "0x83");
    }
}
