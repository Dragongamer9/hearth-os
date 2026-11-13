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
    let slaves = |k: &str| -> Vec<String> {
        if k.contains('/') || k.starts_with('.') { return vec![]; }
        std::fs::read_dir(format!("/sys/class/block/{k}/slaves")).into_iter().flatten().flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned()).collect()
    };
    let ctx = udisks::Ctx { uid: unsafe { libc::getuid() }, steam: &steam, usage: &usage, slaves: &slaves };
    let mut drives = udisks::parse(&objs, &ctx);
    // Hearth's drive letters (read only: opening Disks never hands one out), and whether each file system's resize
    // tool is installed.
    let letters = crate::letters::load();
    for d in drives.iter_mut() {
        for p in d.partitions.iter_mut().filter(|p| p.kind != "free") {
            if p.system && p.kind == "hearth" { p.letter = "C".into(); }
            else if let Some(l) = letters.get(&crate::letters::key(&p.uuid)) { p.letter = l.clone(); }
            p.resize_tool = resize_tool(&c, &p.fs);
        }
    }
    Ok(drives)
}

static RESIZE_TOOLS: std::sync::Mutex<Option<HashMap<String, String>>> = std::sync::Mutex::new(None);

/// The tool resizing `fs` needs when udisks2 says it's missing (Manager.CanResize), else "". Asked once per file
/// system while Disks is open.
fn resize_tool(c: &Connection, fs: &str) -> String {
    let (a, b, x, y) = crate::safety::resize_modes(fs);
    if !(a || b || x || y) { return String::new(); }
    let key = fs.to_lowercase();
    let mut g = RESIZE_TOOLS.lock().unwrap_or_else(|p| p.into_inner());
    let cache = g.get_or_insert_with(HashMap::new);
    if let Some(t) = cache.get(&key) { return t.clone(); }
    let t = match call::<_, (bool, u64, String)>(c, MANAGER, "org.freedesktop.UDisks2.Manager", "CanResize", &(key.as_str(),)) {
        Ok((true, _, _)) => String::new(),
        Ok((false, _, util)) => if util.is_empty() { format!("a {key} resize tool") } else { util },
        // An older udisks2 without CanResize: try, and say what went wrong if it doesn't work.
        Err(_) => String::new(),
    };
    cache.insert(key, t.clone());
    t
}

/// What's using a volume right now (the names of the programs with files or folders open on it), and "Steam"
/// when Steam is running and the volume holds games. Only this user's programs can be seen, which is who uses
/// a games drive.
pub fn busy_with(p: &Partition) -> Vec<String> {
    let mounts: Vec<&String> = p.mount_points.iter().filter(|m| !crate::model::is_system_mount(m)).collect();
    let under = |t: &std::path::Path| mounts.iter().any(|m| t.starts_with(m.as_str()));
    let me = std::process::id().to_string();
    let mut names = std::collections::BTreeSet::new();
    let Ok(rd) = std::fs::read_dir("/proc") else { return vec![] };
    for e in rd.flatten() {
        let pid = e.file_name().to_string_lossy().to_string();
        if pid == me || pid.is_empty() || !pid.chars().all(|c| c.is_ascii_digit()) { continue; }
        let base = e.path();
        let comm = std::fs::read_to_string(base.join("comm")).unwrap_or_default().trim().to_string();
        if p.steam_library && (comm == "steam" || comm == "steamwebhelper" || comm == "reaper") { names.insert("Steam".to_string()); continue; }
        if mounts.is_empty() { continue; }
        let mut hit = ["cwd", "exe"].iter().any(|l| std::fs::read_link(base.join(l)).map(|t| under(&t)).unwrap_or(false));
        if !hit {
            if let Ok(fds) = std::fs::read_dir(base.join("fd")) {
                hit = fds.flatten().any(|f| std::fs::read_link(f.path()).map(|t| under(&t)).unwrap_or(false));
            }
        }
        if hit { names.insert(if comm.is_empty() { format!("program {pid}") } else { comm }); }
    }
    names.into_iter().collect()
}

/// Properties › Health's "What the drive reports": its SMART attributes (SATA) or NVMe health log, in plain words.
pub fn health_lines(d: &Drive) -> Vec<Value> {
    let Ok(c) = bus() else { return vec![] };
    let objs = objects(&c).unwrap_or(Value::Null);
    let o = &objs[d.id.as_str()];
    if o.get(udisks::ATA).is_some() {
        type Attr = (u8, String, u16, i32, i32, i32, i64, i32, HashMap<String, OwnedValue>);
        match call::<_, Vec<Attr>>(&c, &d.id, udisks::ATA, "SmartGetAttributes", &(opts(),)) {
            Ok(attrs) => udisks::ata_lines(&attrs.into_iter().map(|a| (a.1, a.3, a.5, a.6, a.7)).collect::<Vec<_>>()),
            Err(_) => vec![],
        }
    } else if o.get(udisks::NVME).is_some() {
        match call::<_, HashMap<String, OwnedValue>>(&c, &d.id, udisks::NVME, "SmartGetAttributes", &(opts(),)) {
            Ok(m) => { let j: Map<String, Value> = m.iter().map(|(k, v)| (k.clone(), to_json(v))).collect(); udisks::nvme_lines(&j) }
            Err(_) => vec![],
        }
    } else { vec![] }
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

/// Mounts a file system that was just made, once udisks has caught up with it (its Filesystem part can show up
/// a moment after the call returns). Returns a note for the message when it can't be mounted.
fn mount_new(c: &Connection, path: &str) -> String {
    for _ in 0..40 {
        if objects(c).ok().and_then(|o| o.get(path).and_then(|x| x.get(udisks::FILESYSTEM)).cloned()).is_some() { break; }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    match mount(c, path) {
        Ok(_) => String::new(),
        Err(e) => format!(" It couldn't be mounted yet ({e})."),
    }
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

/// Lets go of a whole drive before Make a bootable USB writes to it: everything on it unmounted, encrypted
/// partitions locked.
pub fn release(d: &Drive) -> Result<(), String> {
    let c = bus()?;
    tear_down(&c, d)
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

fn format_part(c: &Connection, p: &Partition, fs: &str, label: &str, full: bool) -> Result<(), String> {
    unmount(c, p)?;
    let mut o = fs_opts(fs, label);
    o.insert("tear-down", V::from(true));
    // Not a quick format: zeros over all of it first.
    if full { o.insert("erase", V::from("zero".to_string())); }
    // A file system on the whole drive (no partition table) has no partition type to update.
    if p.number == 0 { o.remove("update-partition-type"); }
    call0(c, &p.id, udisks::BLOCK, "Format", &(fs, o))
}

/// Opens a folder in Hearth Files (the desktop's file manager).
fn open_folder(at: &str) -> Result<String, String> {
    std::process::Command::new("xdg-open").arg(at).spawn().map(|_| String::new()).map_err(|e| format!("Couldn't open it: {e}"))
}

/// After a volume is made or formatted: its drive letter (when one was picked) and opening it in Files (when
/// asked). Returns notes for the message; neither stops what was done.
fn finish_new(c: &Connection, path: &str, o: &Opts) -> String {
    let mut note = String::new();
    if !o.letter.is_empty() {
        let mut uuid = String::new();
        for _ in 0..40 {
            uuid = prop(c, path, udisks::BLOCK, "IdUUID").as_str().unwrap_or("").to_string();
            if !uuid.is_empty() { break; }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        match crate::letters::set(&uuid, &o.letter) {
            Ok(l) => note.push_str(&format!(" It's {l}:.")),
            Err(e) => note.push_str(&format!(" It didn't get its letter: {e}")),
        }
    }
    if o.open {
        let at: Vec<String> = prop(c, path, udisks::FILESYSTEM, "MountPoints").as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
        match at.first() {
            Some(m) => { let _ = open_folder(m); }
            None => note.push_str(" It isn't mounted, so it couldn't be opened in Files yet."),
        }
    }
    note
}

/// Extend (into the unallocated space right after it) or Shrink a volume by `amount` bytes, through udisks2:
/// the partition and its file system are changed in the order that never leaves the file system bigger than its
/// partition. A file system that can only change unmounted is unmounted (after checking nothing uses it), checked
/// first and mounted again after. Refused while Steam or another program uses it, or if it isn't clean.
fn resize(c: &Connection, d: &Drive, p: &Partition, grow: bool, amount: u64, step: &dyn Fn(&str)) -> Result<String, String> {
    let users = busy_with(p);
    if !users.is_empty() { return Err(crate::busy_text(&users)); }
    let (og, fg, os, fs_) = crate::safety::resize_modes(&p.fs);
    let (online, offline) = if grow { (og, fg) } else { (os, fs_) };
    let was = p.mounted;
    let unmount_it = was && !online;
    let mount_it = !was && !offline;
    let name = crate::safety::volume_name(p);
    let room_end = if grow {
        let f = crate::safety::free_after(d, p).ok_or("There's no unallocated space right after it anymore. Nothing was changed.")?;
        f.offset + f.size
    } else { 0 };
    if mount_it { step("Mounting"); mount(c, &p.id)?; }
    if unmount_it { step("Unmounting"); unmount(c, p)?; }
    let back = |c: &Connection| { if unmount_it { let _ = mount(c, &p.id); } };
    let mounted_during = (was && !unmount_it) || mount_it;
    if !mounted_during {
        // It's unmounted now: it must be clean before it changes (resize2fs and ntfsresize want that too).
        step("Checking the file system");
        match call::<_, bool>(c, &p.id, udisks::FILESYSTEM, "Check", &(opts(),)) {
            Ok(true) => {}
            Ok(false) => { back(c); return Err(format!("{name}'s file system has problems, so it wasn't resized. Choose Properties › Repair first. Nothing was changed.")); }
            Err(e) => { back(c); return Err(format!("{name} couldn't be checked, so it wasn't resized ({e}). Nothing was changed.")); }
        }
        // ext's resize2fs only resizes an unmounted file system checked since it was last mounted: a check that
        // writes the "checked" mark (on a clean file system it changes nothing else).
        if p.fs.starts_with("ext") {
            if let Ok(false) | Err(_) = call::<_, bool>(c, &p.id, udisks::FILESYSTEM, "Repair", &(opts(),)) {
                back(c);
                return Err(format!("{name} couldn't be made ready to resize. Nothing was changed."));
            }
        }
    }
    let r = if grow {
        let new = (p.size + amount).min(room_end.saturating_sub(p.offset)) / MIB * MIB;
        step("Making the partition bigger");
        match call0(c, &p.id, udisks::PARTITION, "Resize", &(new, opts())) {
            Err(e) => Err(format!("{e} Nothing was changed.")),
            Ok(()) => {
                step("Making the file system bigger");
                call0(c, &p.id, udisks::FILESYSTEM, "Resize", &(0u64, opts()))
                    .map(|_| format!("{name} is now {}.", crate::model::human(new)))
                    .map_err(|e| format!("The partition was made bigger, but its file system couldn't grow into it yet ({e}). Your files are fine: choose Extend again to finish."))
            }
        }
    } else {
        let new = p.size.saturating_sub(amount) / MIB * MIB;
        if new < 32 * MIB { back(c); return Err("That would leave it too small.".into()); }
        // The file system a little smaller than the partition will be, then the partition, then the file system
        // grown to fill it exactly (the partition can come out a little bigger than asked, never smaller).
        step("Making the file system smaller");
        match call0(c, &p.id, udisks::FILESYSTEM, "Resize", &(new - 4 * MIB, opts())) {
            Err(e) => Err(format!("{e} Nothing was changed.")),
            Ok(()) => {
                step("Making the partition smaller");
                match call0(c, &p.id, udisks::PARTITION, "Resize", &(new, opts())) {
                    Err(e) => Err(format!("Its file system was made smaller, but the partition wasn't ({e}). Your files are fine; the space isn't free yet: choose Shrink again.")),
                    Ok(()) => {
                        step("Fitting the file system to the partition");
                        let _ = call0(c, &p.id, udisks::FILESYSTEM, "Resize", &(0u64, opts()));
                        Ok(format!("{name} is now {}. The space after it is unallocated.", crate::model::human(new)))
                    }
                }
            }
        }
    };
    back(c);
    r
}

/// Mounts it, or mounts it again after a change, where it was (udisks picks /run/media/<you>/<label>).
fn remount_if(c: &Connection, path: &str, was: bool) { if was { let _ = mount(c, path); } }

/// "Use for games": the file system (on block object `path`) mounts by itself at /var/mnt/games-<name> on every
/// start (an fstab entry, added through udisks: x-systemd.automount, so an SD card that isn't there doesn't
/// hold up starting), then it's mounted and Hearth's Steam code (hearth-core) adds <mount>/SteamLibrary to Steam.
fn setup_games(c: &Connection, path: &str, step: &dyn Fn(&str)) -> Result<String, String> {
    // A file system that was just made can take a moment to show up with its UUID.
    let mut o = Value::Null;
    for _ in 0..40 {
        o = objects(c)?.get(path).cloned().unwrap_or(Value::Null);
        if o[udisks::BLOCK]["IdUUID"].as_str().map(|u| !u.is_empty()).unwrap_or(false) && o.get(udisks::FILESYSTEM).is_some() { break; }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
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
            let at = match p.mount_points.first() {
                Some(m) => m.clone(),
                // Open mounts it first (never Hearth's own or a start-up partition: safety doesn't offer that).
                None if p.mountable && !p.system && !p.protected => { step("Mounting"); mount(&c, &p.id)? }
                None => return Err("It isn't mounted.".into()),
            };
            open_folder(&at)
        }
        ("letter", Some(p)) => {
            step("Changing the letter");
            crate::letters::set(&p.uuid, &o.letter).map(|l| format!("It's {l}: now. Hearth Files and Windows apps use the new letter."))
        }
        ("extend", Some(p)) | ("shrink", Some(p)) => resize(&c, d, p, action == "extend", o.size, step),
        ("selftest", None) => {
            let objs = objects(&c)?;
            let me = &objs[d.id.as_str()];
            step("Starting the check");
            if me.get(udisks::ATA).is_some() { call0(&c, &d.id, udisks::ATA, "SmartSelftestStart", &("extended", opts()))?; }
            else if me.get(udisks::NVME).is_some() { call0(&c, &d.id, udisks::NVME, "SmartSelftestStart", &("extended", opts()))?; }
            else { return Err("This drive can't test itself.".into()); }
            Ok("The full check has started. It runs inside the drive; Properties › Health shows how far it is and the result.".into())
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
            step(if o.full { "Formatting (writing zeros over all of it)" } else { "Formatting" });
            format_part(&c, p, &o.fs, &o.label, o.full)?;
            let note = if d.removable || o.open { mount_new(&c, &p.id) } else { String::new() };
            Ok(format!("Formatted.{note}{}", finish_new(&c, &p.id, o)))
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
            let note = if d.removable || o.open { mount_new(&c, &path) } else { String::new() };
            Ok(format!("The new volume is ready.{note}{}", finish_new(&c, &path, o)))
        }
        ("format_drive", None) => {
            let path = format_drive(&c, d, &o.fs, &o.label, step)?;
            let note = if d.removable { step("Mounting it"); mount_new(&c, &path) } else { String::new() };
            Ok(format!("{} is formatted.{note}", d.name()))
        }
        ("games", target) => {
            let label = if o.label.is_empty() { "Games".to_string() } else { o.label.clone() };
            let path = match target {
                None => format_drive(&c, d, &o.fs, &label, step)?,
                Some(p) if p.kind == "free" => { step("Making the partition"); create(&c, d, p.offset, o.size, &o.fs, &label)? }
                Some(p) if !crate::safety::games_ready("linux", &p.fs) => { step("Formatting"); format_part(&c, p, &o.fs, &label, o.full)?; p.id.clone() }
                Some(p) => p.id.clone(),
            };
            let msg = setup_games(&c, &path, step)?;
            Ok(format!("{msg}{}", finish_new(&c, &path, o)))
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
