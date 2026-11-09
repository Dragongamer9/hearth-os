//! What Disks lets you do, and what it says first. Every action is checked here twice: when the page draws the
//! choices, and again against a fresh look at the drives right before anything changes (lib.rs `run`).
//! The rules:
//!  - The running system's own partitions (Hearth OS's root, start-up and EFI partitions, swap in use, and what
//!    it's built on through LVM, RAID or encryption; Windows' C:, EFI, reserved and recovery partitions) and every
//!    start-up or recovery partition are never changed, unmounted, resized or formatted. Disk Management offers
//!    only Open and Properties for them, and says why the rest is greyed out.
//!  - The system's drive can't be formatted as a whole, and its partitions aren't resized. Other changes on it
//!    need a word (or the volume's name) typed in.
//!  - Formatting or deleting a volume needs its name typed exactly. Internal drives and drives with another
//!    system on them need the word typed in before the whole drive is erased.
//!  - Before anything is erased the page shows exactly what will be lost (with the games on it, from what
//!    Storage knows: lib.rs).
//!  - The run is refused when the drive or partition isn't the one that was confirmed (`fingerprint`).

use crate::model::{human, Action, Drive, Partition, GIB, MIB};
use serde::{Deserialize, Serialize};

/// What the confirmation asks you to type for an erase that needs it.
pub const ERASE_WORD: &str = "ERASE";
/// ...and for a change on the system's drive that doesn't erase anything.
pub const CHANGE_WORD: &str = "CHANGE";
/// Why Disk Management greys out everything but Open and Properties for Hearth's own partitions.
pub const HEARTH_NOTE: &str = "Hearth runs from this partition, so it can't be changed or deleted here.";
/// ...for another system's start-up or recovery partition (on a drive Hearth doesn't run from).
pub const STARTUP_NOTE: &str = "A computer needs this partition to start up, so it can't be changed or deleted here. Format the whole drive if you don't need it anymore.";
/// Space a games drive keeps free after shrinking (Steam's updates and new downloads need room).
pub const GAMES_KEEP: u64 = 10 * GIB;
/// The smallest change Extend and Shrink make.
pub const MIN_RESIZE: u64 = 16 * MIB;

/// Options from the page for one action.
#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Opts {
    pub fs: String,
    pub label: String,
    /// Size in bytes for a new partition (0: all the free space); for Extend and Shrink, how much to add or take
    /// away.
    pub size: u64,
    /// What was typed in the confirmation.
    pub typed: String,
    /// The plan's `fingerprint` from when it was shown: the run is refused if the drive is no longer the same one
    /// (a USB stick swapped for another gets the same device path). Empty: not checked.
    pub expect: String,
    /// A drive letter (Hearth's letters, hearth_drive_letters: "F"): for Change drive letter, and for a new volume.
    pub letter: String,
    /// Not a quick format: every block is written with zeros first (slow).
    pub full: bool,
    /// Open the new or changed volume in Hearth Files when it's ready.
    pub open: bool,
}

/// What the page shows before an action runs.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub action: String,
    pub title: String,
    /// Why it can't be done (then nothing else matters).
    pub refuse: Option<String>,
    /// The word (or the volume's name) to type before it runs, if one is needed.
    pub typed: Option<String>,
    /// `typed` must match exactly, capitals and all (a volume's name); otherwise any case (ERASE, CHANGE).
    pub typed_exact: bool,
    /// Exactly what will be lost.
    pub lose: Vec<String>,
    pub warn: Vec<String>,
    /// The confirm button's label.
    pub confirm: String,
    pub danger: bool,
    /// File systems to choose from (formatting), the first is the suggestion: [{id, label, note}].
    pub fs_choices: Vec<FsChoice>,
    pub ask_label: bool,
    pub label_default: String,
    /// For a new partition: the most it can be.
    pub max_size: u64,
    /// Which drive and partition this plan is about (see `fingerprint`); the page sends it back with the run.
    pub fingerprint: String,
    /// The volume's name, as Disk Management shows it.
    pub volume: String,
    /// Games on what's erased (from what Storage knows; lib.rs fills these in on Linux).
    pub games: Option<u64>,
    pub games_bytes: Option<u64>,
    /// Extend and Shrink: the partition's size now, and the most that can be added or taken away.
    pub resize_size: u64,
    pub resize_max: u64,
    /// It's unmounted while it changes (and mounted again after).
    pub offline: bool,
    /// Change drive letter: its letter now, and the letters it can have (lib.rs fills them in).
    pub letter: String,
    pub letters: Vec<String>,
}

/// What identifies the drive and the partition (or free space) an action is for, beyond their paths: the drive's
/// serial, model and size, and the partitions' UUIDs, places and sizes. A different stick plugged in where the
/// last one was gets the same paths but not the same fingerprint.
pub fn fingerprint(d: &Drive, p: Option<&Partition>) -> String {
    let part = |p: &Partition| format!("{}|{}|{}|{}|{}", p.id, p.uuid, p.offset, p.size, p.fs);
    let mut s = format!("{}|{}|{}|{}|{}", d.id, d.serial, d.vendor, d.model, d.size);
    match p {
        Some(p) => { s.push_str("#"); s.push_str(&part(p)); }
        None => for p in d.partitions.iter().filter(|p| p.kind != "free") { s.push_str("#"); s.push_str(&part(p)); },
    }
    s
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FsChoice { pub id: String, pub label: String, pub note: String }

fn choice(id: &str, label: &str, note: &str) -> FsChoice { FsChoice { id: id.into(), label: label.into(), note: note.into() } }

/// The file systems Disks formats with, best first for the purpose. `games`: for a Steam library.
/// On Linux: exFAT (sharing), ext4, NTFS, btrfs. On Windows: exFAT, NTFS, FAT32.
pub fn fs_choices(host: &str, games: bool, size: u64) -> Vec<FsChoice> {
    if host == "windows" {
        let mut v = vec![
            choice("exfat", "exFAT", "Works with Windows, Mac, Linux, Steam Deck, consoles and cameras"),
            choice("ntfs", "NTFS", "Windows only; best for games on Windows"),
        ];
        if size <= 32 * crate::model::GIB { v.push(choice("fat32", "FAT32", "Older devices; no files over 4 GB")); }
        if games { v.swap(0, 1); }
        v
    } else {
        let mut v = vec![
            choice("exfat", "exFAT", "For sharing: works with Windows, Mac, Steam Deck, consoles and cameras"),
            choice("ext4", "ext4", "Best for games and files on Hearth OS"),
            choice("ntfs", "NTFS", "For Windows PCs"),
            choice("btrfs", "btrfs", "Linux, with snapshots and compression"),
        ];
        if games { v.retain(|c| c.id == "ext4" || c.id == "btrfs"); }
        v
    }
}

/// Disk Management's choices for a new volume or formatting one (New Simple Volume, Format…): ext4 first on
/// Hearth OS, then NTFS, exFAT, FAT32 (up to 32 GB, like Windows) and Btrfs. Windows: as `fs_choices`.
pub fn volume_fs_choices(host: &str, size: u64) -> Vec<FsChoice> {
    if host == "windows" { return fs_choices(host, false, size); }
    let mut v = vec![
        choice("ext4", "ext4", "Best for Hearth and games"),
        choice("ntfs", "NTFS", "Windows can use it too"),
        choice("exfat", "exFAT", "USB sticks, any computer"),
    ];
    if size <= 32 * GIB { v.push(choice("vfat", "FAT32", "Older devices; no files over 4 GB")); }
    v.push(choice("btrfs", "Btrfs", "Linux, with snapshots and compression"));
    v
}

/// File systems a Steam library works on without formatting.
pub fn games_ready(host: &str, fs: &str) -> bool {
    let fs = fs.to_lowercase();
    if host == "windows" { matches!(fs.as_str(), "ntfs" | "exfat" | "refs") } else { matches!(fs.as_str(), "ext4" | "btrfs" | "xfs" | "f2fs") }
}

/// The longest label each file system takes.
pub fn label_max(fs: &str) -> usize {
    match fs.to_lowercase().as_str() { "vfat" | "fat32" | "fat" => 11, "exfat" => 15, "ext4" | "ext3" | "ext2" => 16, "ntfs" => 32, "xfs" => 12, _ => 64 }
}

/// Checks a new label (empty is fine: no label).
pub fn check_label(fs: &str, label: &str) -> Result<(), String> {
    if label.chars().count() > label_max(fs) { return Err(format!("That name is too long. Up to {} letters.", label_max(fs))); }
    if label.chars().any(|c| c.is_control() || "\"*/:<>?\\|".contains(c)) {
        return Err("Names can't have these: \" * / : < > ? \\ |".into());
    }
    Ok(())
}

/// A file system's name as Disk Management shows it.
pub fn fs_name(fs: &str) -> String {
    match fs.to_lowercase().as_str() {
        "vfat" | "fat32" => "FAT32".into(), "exfat" => "exFAT".into(), "ntfs" => "NTFS".into(), "btrfs" => "Btrfs".into(),
        "crypto_luks" => "Encrypted".into(), "" => "Unknown".into(), other => other.to_string(),
    }
}

/// The name you type to format or delete a volume: its label, or its device's name when it has none ("sdb1").
pub fn volume_name(p: &Partition) -> String {
    let l = p.label.trim();
    if !l.is_empty() { return l.to_string(); }
    let dev = p.device.trim_end_matches(['\\', ':']).rsplit('/').next().unwrap_or("").to_string();
    if !dev.is_empty() { return dev; }
    if !p.letter.is_empty() { return p.letter.clone(); }
    p.what.clone()
}

/// How a file system can be resized: (grow while mounted, grow unmounted, shrink while mounted, shrink unmounted).
/// What udisks2 (libblockdev) does for each; anything else isn't resized.
pub fn resize_modes(fs: &str) -> (bool, bool, bool, bool) {
    match fs.to_lowercase().as_str() {
        "ext2" | "ext3" | "ext4" => (true, true, false, true),
        "btrfs" => (true, false, true, false),
        "xfs" => (true, false, false, false),
        "ntfs" => (false, true, false, true),
        _ => (false, false, false, false),
    }
}

/// The unallocated space right after a partition (where Extend grows it), if there is some.
pub fn free_after<'a>(d: &'a Drive, p: &Partition) -> Option<&'a Partition> {
    let end = p.offset + p.size;
    d.partitions.iter().find(|f| f.kind == "free" && f.offset >= end && f.offset - end < 2 * MIB)
}

/// The partition right before some unallocated space ("Extend <it> into this space…").
pub fn before_free<'a>(d: &'a Drive, free: &Partition) -> Option<&'a Partition> {
    d.partitions.iter().find(|p| p.kind != "free" && p.kind != "extended" && p.offset + p.size <= free.offset && free.offset - (p.offset + p.size) < 2 * MIB)
}

/// What a volume keeps free when it's shrunk: 10 GB on a games drive, otherwise a twentieth of it (16 MB to 1 GB).
pub fn keep_free(p: &Partition) -> u64 {
    if p.steam_library { GAMES_KEEP } else { (p.size / 20).clamp(MIN_RESIZE, GIB) }
}

/// The most Shrink can take away (whole MBs), when it's known how full the volume is.
pub fn shrink_max(p: &Partition) -> Option<u64> {
    p.free.map(|f| f.saturating_sub(keep_free(p)) / MIB * MIB)
}

/// Why Extend (`grow`) or Shrink can't be done to this partition now; None: it can.
pub fn resize_block(d: &Drive, p: &Partition, grow: bool) -> Option<String> {
    if d.system { return Some("Hearth runs from this drive, so Hearth Disks doesn't resize partitions on it.".into()); }
    if d.read_only { return Some("This drive is read-only.".into()); }
    if p.kind == "extended" || crate::model::is_logical(p) { return Some("Partitions inside an extended partition can't be resized here.".into()); }
    if p.number == 0 { return Some("It fills the whole drive without a partition table, so it can't be resized.".into()); }
    if p.kind == "encrypted" { return Some("It's locked (encrypted), so it can't be resized here.".into()); }
    let (og, fg, os, fs_) = resize_modes(&p.fs);
    let name = fs_name(&p.fs);
    if grow && !(og || fg) { return Some(format!("{name} can't be made bigger here.")); }
    if !grow && !(os || fs_) {
        return Some(if og { format!("{name} can only be made bigger, not smaller.") } else { format!("{name} can't be made smaller here.") });
    }
    if !p.resize_tool.is_empty() {
        return Some(format!("Resizing {name} needs {}, which isn't in this Hearth image.", p.resize_tool));
    }
    if grow {
        if free_after(d, p).is_none() { return Some("There's no unallocated space right after it to grow into.".into()); }
        if !fg && !p.mounted { return Some(format!("Mount it first: {name} grows while it's mounted.")); }
        None
    } else {
        let Some(max) = shrink_max(p) else { return Some("Mount it first, so Hearth can see how much of it is free.".into()) };
        if max < MIN_RESIZE {
            return Some(if p.steam_library { "A games drive keeps 10 GB free for Steam's updates, so there's nothing to take away.".into() } else { "It's too full to shrink.".into() });
        }
        None
    }
}

fn act(id: &str, label: &str, danger: bool) -> Action { Action { id: id.into(), label: label.into(), danger, why: String::new() } }
fn act_why(id: &str, label: &str, danger: bool, why: &str) -> Action { Action { id: id.into(), label: label.into(), danger, why: why.into() } }

/// Another system lives on this drive (erasing it would remove that system).
fn has_other_os(d: &Drive) -> bool { d.partitions.iter().any(|p| matches!(p.kind.as_str(), "windows" | "steamos" | "hearth")) }

/// What can be done with a whole drive.
pub fn drive_actions(d: &Drive, host: &str) -> Vec<Action> {
    let mut v = vec![];
    // A full check reads the drive; it changes nothing, so the system's drive has it too.
    let check = d.health.smart && host != "windows";
    if d.system {
        if check { v.push(act("selftest", "Run a full check", false)); }
        return v;
    }
    if d.removable || d.can_power_off || d.can_eject { v.push(act("eject", "Safely remove", false)); }
    if !d.read_only && d.kind != "optical" {
        v.push(act("games", "Use for games", true));
        v.push(act("format_drive", if d.kind == "sd" { "Format SD card" } else { "Format drive" }, true));
    }
    if check { v.push(act("selftest", "Run a full check", false)); }
    v
}

/// Everything Disk Management's menu has for one partition or stretch of free space: what can be done (empty
/// `why`) and what can't, with why. Windows: Hearth's letters, Extend and Shrink aren't there at all.
pub fn partition_choices(d: &Drive, p: &Partition, host: &str) -> Vec<Action> {
    let win = host == "windows";
    let mut v = vec![];
    let ro = if d.read_only { "This drive is read-only." } else { "" };
    if p.kind == "free" {
        v.push(act_why("create", "New Simple Volume…", false, ro));
        v.push(act_why("games", "New volume for games…", false, ro));
        return v;
    }
    let locked = p.system || p.protected;
    let can_open = p.mounted && (!p.mount_points.is_empty() || !p.letter.is_empty());
    // On Hearth OS, Open mounts it first when it isn't (never a protected one).
    let open_why = if can_open || (p.mountable && !locked && !win) { "" } else if !p.mountable { "There are no files on it to open." } else { "It isn't mounted." };
    v.push(act_why("open", "Open", false, open_why));
    if locked {
        let why = if p.system || d.system { HEARTH_NOTE } else { STARTUP_NOTE };
        let mut ids: Vec<(&str, &str)> = vec![];
        if !win { ids.push(("letter", "Change drive letter…")); }
        ids.extend([("format", "Format…")]);
        if !win { ids.extend([("extend", "Extend Volume…"), ("shrink", "Shrink Volume…")]); }
        ids.extend([("delete", "Delete Volume…"), ("games", "Use for games"), ("unmount", "Unmount")]);
        for (id, label) in ids { v.push(act_why(id, label, matches!(id, "format" | "delete"), why)); }
        return v;
    }
    if !win {
        let why = if !p.mountable { "It has no file system to give a letter to." } else if p.uuid.is_empty() { "It has no file system ID yet, so it can't keep a letter." } else { "" };
        v.push(act_why("letter", "Change drive letter…", false, why));
    }
    if p.mountable && !p.mounted { v.push(act("mount", if win { "Give it a drive letter" } else { "Mount" }, false)); }
    if p.mountable && p.mounted { v.push(act("unmount", if win { "Remove its drive letter" } else { "Unmount" }, false)); }
    if p.mountable {
        if p.steam_library { v.push(act_why("games", "Used for games", false, "It's already used for games.")); }
        else { v.push(act_why("games", "Use for games", !games_ready(host, &p.fs), ro)); }
        v.push(act_why("rename", "Rename", false, ro));
        v.push(act_why("check", "Check for problems", false, ro));
        v.push(act_why("repair", "Repair", false, ro));
    }
    if p.kind != "extended" {
        v.push(act_why("format", "Format…", true, ro));
        if !win {
            v.push(act_why("extend", "Extend Volume…", false, &resize_block(d, p, true).unwrap_or_default()));
            v.push(act_why("shrink", "Shrink Volume…", false, &resize_block(d, p, false).unwrap_or_default()));
        }
        v.push(act_why("delete", "Delete Volume…", true, ro));
    }
    v
}

/// What can be done with one partition or stretch of free space, in the order the page offers them.
pub fn partition_actions(d: &Drive, p: &Partition, host: &str) -> Vec<Action> {
    partition_choices(d, p, host).into_iter().filter(|a| a.why.is_empty()).collect()
}

/// Fills in every drive's and partition's actions (and what's greyed out, with why).
pub fn annotate(drives: &mut [Drive], host: &str) {
    for d in drives.iter_mut() {
        d.actions = drive_actions(d, host);
        let snapshot = d.clone();
        for p in d.partitions.iter_mut() {
            let (ok, no): (Vec<Action>, Vec<Action>) = partition_choices(&snapshot, p, host).into_iter().partition(|a| a.why.is_empty());
            p.actions = ok;
            p.blocked = no;
        }
    }
}

fn what_line(p: &Partition) -> String {
    let mut s = if p.label.is_empty() { format!("{} ({})", p.what, human(p.size)) } else { format!("\"{}\" ({}, {})", p.label, p.what, human(p.size)) };
    if let Some(u) = p.used { if u > 0 { s.push_str(&format!(": {} of files", human(u))); } }
    if p.steam_library { s.push_str(", with the games on it"); }
    s
}

/// What will be lost, partition by partition.
fn losing(parts: &[&Partition]) -> Vec<String> {
    parts.iter().filter(|p| p.kind != "free").map(|p| format!("Everything on {}", what_line(p))).collect()
}

/// Megabytes, Windows' way: "307,200 MB".
pub fn mb(bytes: u64) -> String {
    let n = (bytes / MIB).to_string();
    let mut out = String::new();
    for (i, c) in n.chars().enumerate() {
        if i > 0 && (n.len() - i) % 3 == 0 { out.push(','); }
        out.push(c);
    }
    format!("{out} MB")
}

/// What an action will do, or why it can't. `p`: the partition or free space (None: the whole drive).
pub fn plan(action: &str, d: &Drive, p: Option<&Partition>, host: &str) -> Plan {
    let mut plan = Plan { action: action.into(), confirm: "OK".into(), fingerprint: fingerprint(d, p), ..Default::default() };
    if let Some(p) = p { plan.volume = volume_name(p); }
    let choices = match p { Some(p) => partition_choices(d, p, host), None => drive_actions(d, host) };
    let refuse = |plan: &mut Plan, why: &str| { plan.refuse = Some(why.into()); };
    let sys_name = if host == "windows" { "Windows" } else { "Hearth OS" };
    if let Some(p) = p {
        if p.system && action != "open" {
            refuse(&mut plan, &format!("{sys_name} is running from this partition, so Disks won't change it."));
            return plan;
        }
        if p.protected && action != "open" {
            refuse(&mut plan, if d.system { "Your PC needs this partition to start up, so Disks won't change it." } else { STARTUP_NOTE });
            return plan;
        }
    } else if d.system && !matches!(action, "open" | "selftest") {
        refuse(&mut plan, &format!("{sys_name} is running from this drive, so Disks won't erase it or remove it."));
        return plan;
    }
    match choices.iter().find(|a| a.id == action) {
        None => { refuse(&mut plan, "That can't be done here."); return plan; }
        Some(a) if !a.why.is_empty() => { refuse(&mut plan, &a.why); return plan; }
        _ => {}
    }
    let drive_name = d.name();
    let system_drive_note = format!("This is the drive {sys_name} runs from. Only {} is changed; {sys_name} itself isn't touched.",
        p.map(|p| if p.kind == "free" { "the free space".to_string() } else { what_line(p) }).unwrap_or_default());
    match (action, p) {
        ("open", _) => { plan.title = "Open files".into(); }
        ("mount", Some(p)) => { plan.title = if host == "windows" { format!("Give {} a drive letter", what_line(p)) } else { format!("Mount {}", what_line(p)) }; plan.confirm = "Mount".into(); }
        ("unmount", Some(p)) => {
            plan.title = if host == "windows" { "Remove its drive letter".into() } else { format!("Unmount {}", what_line(p)) };
            plan.confirm = "Unmount".into();
            if p.steam_library { plan.warn.push("Steam can't see the games on it until it's mounted again.".into()); }
        }
        ("eject", None) => {
            plan.title = format!("Safely remove {drive_name}");
            plan.confirm = "Remove".into();
            plan.warn.push("Apps using files on it are asked to let go first. Unplug it once it says it's safe.".into());
        }
        ("selftest", None) => {
            plan.title = format!("Run a full check of {drive_name}");
            plan.confirm = "Start the check".into();
            plan.warn.push("The drive reads every part of itself and notes anything it can't read. Nothing on it is changed.".into());
            plan.warn.push("It can take hours on a big hard drive. You can keep using it meanwhile (a little slower) and close Disks; the result shows in Properties › Health.".into());
        }
        ("letter", Some(p)) => {
            plan.title = format!("Change the drive letter of {}", what_line(p));
            plan.letter = p.letter.clone();
            plan.confirm = "OK".into();
            plan.warn.push("C: is always Hearth. Hearth Files and Windows apps use the new letter; nothing on the drive changes.".into());
        }
        ("rename", Some(p)) => {
            plan.title = format!("Rename {}", what_line(p));
            plan.ask_label = true;
            plan.label_default = p.label.clone();
            plan.confirm = "Rename".into();
            if p.mounted && host != "windows" && matches!(p.fs.as_str(), "vfat" | "exfat" | "ntfs") { plan.warn.push("It's unmounted while it's renamed.".into()); }
        }
        ("check", Some(p)) => {
            plan.title = format!("Check {} for problems", what_line(p));
            plan.confirm = "Check".into();
            if p.mounted && host != "windows" { plan.warn.push("It's unmounted while it's checked.".into()); }
        }
        ("repair", Some(p)) => {
            plan.title = format!("Repair {}", what_line(p));
            plan.confirm = "Repair".into();
            plan.warn.push("Repairing fixes the file system's records. Damaged files can't always be saved; copy what matters first if you can.".into());
            if p.mounted && host != "windows" { plan.warn.push("It's unmounted while it's repaired.".into()); }
            if d.system { plan.typed = Some(CHANGE_WORD.into()); plan.warn.insert(0, system_drive_note.clone()); }
        }
        ("format", Some(p)) | ("delete", Some(p)) => {
            let fmt = action == "format";
            plan.title = if fmt { format!("Format {}", what_line(p)) } else { format!("Delete {}", what_line(p)) };
            plan.lose = losing(&[p]);
            plan.danger = true;
            plan.confirm = if fmt { "Format".into() } else { "Delete".into() };
            if fmt {
                plan.fs_choices = volume_fs_choices(host, p.size);
                plan.ask_label = true;
                plan.label_default = p.label.clone();
            }
            // Its name, typed exactly: the same for every volume, so it's never a habit to press through.
            plan.typed = Some(volume_name(p));
            plan.typed_exact = true;
            if d.system { plan.warn.insert(0, system_drive_note.clone()); }
            if p.kind == "windows" { plan.warn.push("This looks like Windows. Erasing it removes that Windows and everything in it.".into()); }
            if p.kind == "encrypted" { plan.warn.push("It's locked, so Disks can't tell what's in it. All of it will be lost.".into()); }
        }
        ("extend", Some(p)) | ("shrink", Some(p)) => {
            let grow = action == "extend";
            let (og, _, os, _) = resize_modes(&p.fs);
            plan.title = format!("{} {}", if grow { "Extend" } else { "Shrink" }, volume_name(p));
            plan.confirm = if grow { "Extend".into() } else { "Shrink".into() };
            plan.resize_size = p.size;
            plan.resize_max = if grow { free_after(d, p).map(|f| f.size / MIB * MIB).unwrap_or(0) } else { shrink_max(p).unwrap_or(0) };
            plan.offline = p.mounted && !(if grow { og } else { os });
            if grow { plan.warn.push("Only the unallocated space right after it is used. Nothing on it is erased.".into()); }
            else {
                plan.warn.push(format!("Your files stay. Hearth checks the file system first and stops if it isn't clean{}.",
                    if p.steam_library { ", and keeps 10 GB free for Steam's updates" } else { "" }));
            }
            if plan.offline { plan.warn.push("It's unmounted while it changes, then mounted again. Close apps using files on it first.".into()); }
            if p.steam_library { plan.warn.push("Close Steam and any game running from it before you start.".into()); }
        }
        ("create", Some(p)) => {
            plan.title = format!("New Simple Volume in {} of unallocated space", human(p.size));
            plan.fs_choices = volume_fs_choices(host, p.size);
            plan.ask_label = true;
            plan.label_default = "New Volume".into();
            plan.max_size = p.size;
            plan.confirm = "Finish".into();
            if d.system { plan.typed = Some(CHANGE_WORD.into()); plan.warn.push(system_drive_note.clone()); }
        }
        ("games", target) => {
            plan.title = format!("Use {} for games", target.map(what_line).unwrap_or_else(|| drive_name.clone()));
            plan.confirm = "Set up for games".into();
            plan.label_default = "Games".into();
            let needs_format = match target { None => true, Some(p) => p.kind != "free" && !games_ready(host, &p.fs) };
            if target.map(|p| p.kind == "free").unwrap_or(false) {
                let p = target.unwrap();
                plan.fs_choices = fs_choices(host, true, p.size);
                plan.ask_label = true;
                plan.max_size = p.size;
                if d.system { plan.typed = Some(CHANGE_WORD.into()); plan.warn.push(system_drive_note.clone()); }
                plan.warn.push("A new partition is made in the free space and added to Steam as a library.".into());
            } else if needs_format {
                let parts: Vec<&Partition> = match target { Some(p) => vec![p], None => d.partitions.iter().collect() };
                plan.lose = losing(&parts);
                if plan.lose.is_empty() { plan.lose.push(format!("Everything on {drive_name}")); }
                plan.danger = true;
                plan.fs_choices = fs_choices(host, true, target.map(|p| p.size).unwrap_or(d.size));
                plan.ask_label = true;
                plan.confirm = "Erase and set up".into();
                let why = if host == "windows" { "Steam games on Windows need NTFS or exFAT" } else { "Steam games on Hearth OS need ext4 or btrfs (Windows formats can't run them)" };
                if target.is_some() { plan.warn.push(format!("{why}, so it's formatted first.")); }
                else { plan.warn.push(format!("The whole drive is erased and made one partition for Steam games. {why}.")); }
                if d.system || !d.removable || has_other_os(d) || target.map(|p| p.protected).unwrap_or(false) { plan.typed = Some(ERASE_WORD.into()); }
                if d.system { plan.warn.insert(0, system_drive_note.clone()); }
            } else {
                plan.warn.push("A SteamLibrary folder is made on it and added to Steam. Nothing on it is erased.".into());
                if host != "windows" { plan.warn.push("It's set to mount by itself whenever it's there, so Steam always finds it.".into()); }
            }
            plan.warn.push("If Steam is open it closes for a moment and opens again.".into());
        }
        ("format_drive", None) => {
            plan.title = format!("Format {drive_name} ({})", human(d.size));
            let parts: Vec<&Partition> = d.partitions.iter().collect();
            plan.lose = losing(&parts);
            if plan.lose.is_empty() { plan.lose.push(format!("Everything on {drive_name}")); }
            plan.danger = true;
            plan.fs_choices = fs_choices(host, false, d.size);
            plan.ask_label = true;
            plan.confirm = "Erase and format".into();
            if !d.removable || has_other_os(d) { plan.typed = Some(ERASE_WORD.into()); }
            if has_other_os(d) { plan.warn.push("Another system is on this drive. Formatting removes it.".into()); }
            plan.warn.push("The whole drive becomes one partition.".into());
        }
        _ => { refuse(&mut plan, "That can't be done here."); }
    }
    plan
}

/// Is this one of Hearth's drive letters (D: to Y:; C: is Hearth, Z: is Windows apps' view of the system)?
pub fn letter_ok(l: &str) -> bool {
    let l = l.trim().trim_end_matches(':');
    l.len() == 1 && "DEFGHIJKLMNOPQRSTUVWXY".contains(&l.to_uppercase())
}

/// The final check before running: the plan again (from a fresh look at the drives), the typed word, the
/// options. Returns the plan that's going to run.
pub fn approve(action: &str, d: &Drive, p: Option<&Partition>, host: &str, o: &Opts) -> Result<Plan, String> {
    let plan = plan(action, d, p, host);
    if let Some(why) = &plan.refuse { return Err(why.clone()); }
    if !o.expect.is_empty() && o.expect != plan.fingerprint {
        return Err("This isn't the same drive or partition you confirmed (it may have been swapped or changed). Nothing was changed. Look at it again and choose what to do.".into());
    }
    if let Some(word) = &plan.typed {
        let ok = if plan.typed_exact { o.typed.trim() == word.trim() } else { o.typed.trim().to_uppercase() == *word };
        if !ok { return Err(format!("Type {word} to confirm.")); }
    }
    if !plan.fs_choices.is_empty() && !plan.fs_choices.iter().any(|c| c.id == o.fs) {
        return Err("Choose a format from the list.".into());
    }
    if plan.ask_label {
        let fs = if o.fs.is_empty() { p.map(|p| p.fs.clone()).unwrap_or_default() } else { o.fs.clone() };
        check_label(&fs, &o.label)?;
    }
    if plan.max_size > 0 && o.size > plan.max_size { return Err("That's more than the free space.".into()); }
    if plan.max_size > 0 && o.size != 0 && o.size < 32 * MIB { return Err("Make it at least 32 MB.".into()); }
    if !o.letter.is_empty() && !letter_ok(&o.letter) { return Err("Pick a letter from D: to Y:.".into()); }
    if action == "letter" && o.letter.is_empty() { return Err("Pick a letter.".into()); }
    if matches!(action, "extend" | "shrink") {
        if o.size < MIN_RESIZE { return Err("Choose at least 16 MB.".into()); }
        if o.size > plan.resize_max { return Err(format!("That's more than it can {} ({}).", if action == "extend" { "grow" } else { "give up" }, mb(plan.resize_max))); }
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Drive, Partition, GIB};

    fn part(id: &str, kind: &str, fs: &str) -> Partition {
        Partition { id: id.into(), kind: kind.into(), fs: fs.into(), size: 10 * GIB, mountable: !fs.is_empty() && fs != "swap",
                    what: if kind == "data" { "Files".into() } else { kind.into() }, ..Default::default() }
    }

    fn system_drive() -> Drive {
        let mut root = part("root", "hearth", "btrfs");
        root.system = true;
        root.mounted = true;
        root.mount_points = vec!["/sysroot".into()];
        let mut efi = part("efi", "efi", "vfat");
        efi.protected = true;
        efi.system = true;
        let mut win_efi = part("efi2", "efi", "vfat");
        win_efi.protected = true;
        let data = part("data", "data", "exfat");
        let free = Partition { id: "free:d:1".into(), kind: "free".into(), size: 50 * GIB, ..Default::default() };
        Drive { id: "d".into(), kind: "nvme".into(), system: true, size: 500 * GIB, table: "gpt".into(), partitions: vec![efi, win_efi, root, data, free], ..Default::default() }
    }

    fn usb() -> Drive {
        let mut p = part("u1", "data", "exfat");
        p.used = Some(GIB);
        p.label = "STICK".into();
        Drive { id: "u".into(), kind: "usb".into(), removable: true, can_power_off: true, size: 64 * GIB, table: "dos".into(), partitions: vec![p], ..Default::default() }
    }

    #[test]
    fn never_touches_the_running_system() {
        let d = system_drive();
        let root = &d.partitions[2];
        for a in ["format", "delete", "unmount", "rename", "check", "repair", "games", "mount"] {
            let pl = plan(a, &d, Some(root), "linux");
            assert!(pl.refuse.is_some(), "{a} on the root must be refused");
            assert!(approve(a, &d, Some(root), "linux", &Opts { typed: "ERASE".into(), fs: "ext4".into(), ..Default::default() }).is_err());
        }
        // The running system's EFI partition: nothing that changes it.
        for a in ["format", "delete", "rename", "repair"] { assert!(plan(a, &d, Some(&d.partitions[0]), "linux").refuse.is_some(), "{a}"); }
        // Another EFI partition on the same drive (another system's): also refused here.
        assert!(plan("format", &d, Some(&d.partitions[1]), "linux").refuse.is_some());
        // The drive as a whole: no format, no eject, no "use for games".
        for a in ["format_drive", "eject", "games"] { assert!(plan(a, &d, None, "linux").refuse.is_some(), "{a}"); }
        assert!(drive_actions(&d, "linux").is_empty());
        // The root partition offers nothing but opening it.
        let acts: Vec<String> = partition_actions(&d, root, "linux").into_iter().map(|a| a.id).collect();
        assert_eq!(acts, vec!["open"]);
    }

    #[test]
    fn other_partitions_on_the_system_drive_need_the_word() {
        let d = system_drive();
        let data = &d.partitions[3];
        let pl = plan("format", &d, Some(data), "linux");
        assert!(pl.refuse.is_none());
        assert_eq!(pl.typed.as_deref(), Some("Files"), "its name, typed exactly");
        assert!(pl.typed_exact);
        assert!(!pl.lose.is_empty());
        let o = Opts { fs: "ext4".into(), ..Default::default() };
        assert!(approve("format", &d, Some(data), "linux", &o).is_err());
        assert!(approve("format", &d, Some(data), "linux", &Opts { typed: "files".into(), ..o.clone() }).is_err(), "capitals count");
        assert!(approve("format", &d, Some(data), "linux", &Opts { typed: "ERASE".into(), ..o.clone() }).is_err());
        assert!(approve("format", &d, Some(data), "linux", &Opts { typed: "Files".into(), ..o.clone() }).is_ok());
        // Free space on the system drive: making a partition asks for CHANGE.
        let free = &d.partitions[4];
        assert_eq!(plan("create", &d, Some(free), "linux").typed.as_deref(), Some(CHANGE_WORD));
    }

    #[test]
    fn usb_drives_are_simple() {
        let d = usb();
        let pl = plan("format_drive", &d, None, "linux");
        assert!(pl.refuse.is_none());
        assert!(pl.typed.is_none(), "a USB stick with only files needs no typed word");
        assert_eq!(pl.lose, vec!["Everything on \"STICK\" (Files, 10.7 GB): 1.1 GB of files".to_string()]);
        assert_eq!(pl.fs_choices[0].id, "exfat");
        assert!(approve("format_drive", &d, None, "linux", &Opts { fs: "exfat".into(), label: "MY STICK".into(), ..Default::default() }).is_ok());
        assert!(approve("format_drive", &d, None, "linux", &Opts { fs: "zfs".into(), ..Default::default() }).is_err());
        assert!(approve("format_drive", &d, None, "linux", &Opts { fs: "exfat".into(), label: "a/b".into(), ..Default::default() }).is_err());
        assert!(approve("format_drive", &d, None, "linux", &Opts { fs: "exfat".into(), label: "much too long a label".into(), ..Default::default() }).is_err());
        let acts: Vec<String> = drive_actions(&d, "linux").into_iter().map(|a| a.id).collect();
        assert_eq!(acts, vec!["eject", "games", "format_drive"]);
        // Use for games on exFAT (Linux): it has to be formatted as ext4 or btrfs.
        let g = plan("games", &d, Some(&d.partitions[0]), "linux");
        assert!(g.danger && !g.lose.is_empty());
        assert!(g.fs_choices.iter().all(|c| c.id == "ext4" || c.id == "btrfs"));
        // On Windows exFAT is fine for games: nothing is erased.
        let g = plan("games", &d, Some(&d.partitions[0]), "windows");
        assert!(!g.danger && g.lose.is_empty() && g.fs_choices.is_empty());
    }

    #[test]
    fn internal_drives_and_other_systems_need_the_word() {
        let mut d = usb();
        d.removable = false;
        d.kind = "ssd".into();
        assert_eq!(plan("format_drive", &d, None, "linux").typed.as_deref(), Some(ERASE_WORD));
        let mut d = usb();
        d.partitions.push(part("w", "windows", "ntfs"));
        let pl = plan("format_drive", &d, None, "linux");
        assert_eq!(pl.typed.as_deref(), Some(ERASE_WORD));
        assert_eq!(pl.lose.len(), 2);
        assert_eq!(plan("delete", &d, Some(&d.partitions[1]), "linux").typed.as_deref(), Some("windows"));
    }

    #[test]
    fn windows_choices() {
        let small = fs_choices("windows", false, 16 * GIB);
        assert_eq!(small.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), vec!["exfat", "ntfs", "fat32"]);
        assert!(!fs_choices("windows", false, 64 * GIB).iter().any(|c| c.id == "fat32"));
        assert_eq!(fs_choices("windows", true, 64 * GIB)[0].id, "ntfs");
        assert_eq!(fs_choices("linux", false, 0).iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), vec!["exfat", "ext4", "ntfs", "btrfs"]);
        assert!(games_ready("linux", "ext4") && !games_ready("linux", "exfat") && games_ready("windows", "NTFS"));
    }

    #[test]
    fn refuses_a_swapped_drive() {
        // The plan is shown for one USB stick...
        let d = usb();
        let pl = plan("format", &d, Some(&d.partitions[0]), "linux");
        let o = Opts { fs: "exfat".into(), typed: "STICK".into(), expect: pl.fingerprint.clone(), ..Default::default() };
        assert!(approve("format", &d, Some(&d.partitions[0]), "linux", &o).is_ok());
        // ...then it's swapped for another at the same path: same ids, different UUID, serial or size.
        let mut other = usb();
        other.partitions[0].uuid = "1234-ABCD".into();
        assert!(approve("format", &other, Some(&other.partitions[0]), "linux", &o).is_err());
        let mut other = usb();
        other.serial = "OTHER".into();
        assert!(approve("format", &other, Some(&other.partitions[0]), "linux", &o).is_err());
        let mut other = usb();
        other.partitions[0].size = 7 * GIB;
        assert!(approve("format", &other, Some(&other.partitions[0]), "linux", &o).is_err());
        // The whole drive too.
        let pl = plan("format_drive", &d, None, "linux");
        let o = Opts { fs: "exfat".into(), expect: pl.fingerprint.clone(), ..Default::default() };
        assert!(approve("format_drive", &d, None, "linux", &o).is_ok());
        let mut other = usb();
        other.size = 32 * GIB;
        assert!(approve("format_drive", &other, None, "linux", &o).is_err());
        let mut other = usb();
        other.partitions[0].uuid = "5678-EF01".into();
        assert!(approve("format_drive", &other, None, "linux", &o).is_err());
        // Mounting or files being added don't count as a change.
        let mut same = usb();
        same.partitions[0].mounted = true;
        same.partitions[0].used = Some(2 * GIB);
        assert!(approve("format_drive", &same, None, "linux", &o).is_ok());
    }

    #[test]
    fn sizes_for_new_partitions() {
        let d = usb();
        let mut d2 = d.clone();
        d2.partitions.push(Partition { id: "free:u:9".into(), kind: "free".into(), size: 8 * GIB, ..Default::default() });
        let free = d2.partitions[1].clone();
        let o = |size| Opts { fs: "exfat".into(), size, ..Default::default() };
        assert!(approve("create", &d2, Some(&free), "linux", &o(0)).is_ok());
        assert!(approve("create", &d2, Some(&free), "linux", &o(4 * GIB)).is_ok());
        assert!(approve("create", &d2, Some(&free), "linux", &o(9 * GIB)).is_err());
        assert!(approve("create", &d2, Some(&free), "linux", &o(1024)).is_err());
    }

    // ------------------------------------------------------------------ Disk Management

    fn ids(v: &[Action]) -> Vec<&str> { v.iter().map(|a| a.id.as_str()).collect() }
    fn why<'a>(d: &'a Drive, p: &Partition, id: &str) -> String {
        partition_choices(d, p, "linux").into_iter().find(|a| a.id == id).map(|a| a.why).unwrap_or_else(|| format!("({id} not listed)"))
    }

    /// A 100 GiB internal data drive: an ext4 volume (10 GiB, 5 GiB free, mounted) and unallocated space after it.
    fn data_drive(fs: &str) -> Drive {
        let mut p = part("v", "linux", fs);
        p.number = 1; p.offset = MIB; p.size = 10 * GIB; p.uuid = "u-1".into(); p.label = "Media".into(); p.device = "/dev/sdb1".into();
        p.mounted = true; p.mount_points = vec!["/run/media/you/Media".into()]; p.used = Some(5 * GIB); p.free = Some(5 * GIB);
        let free = Partition { id: "free:d2:x".into(), kind: "free".into(), offset: MIB + 10 * GIB, size: 40 * GIB, ..Default::default() };
        Drive { id: "d2".into(), kind: "hdd".into(), size: 100 * GIB, table: "gpt".into(), partitions: vec![p, free], ..Default::default() }
    }

    #[test]
    fn protected_partitions_offer_only_open() {
        let d = system_drive();
        // Hearth's root: Open; everything else greyed out with the same note.
        let root = &d.partitions[2];
        assert_eq!(ids(&partition_actions(&d, root, "linux")), vec!["open"]);
        for id in ["letter", "format", "extend", "shrink", "delete", "games", "unmount"] {
            assert_eq!(why(&d, root, id), HEARTH_NOTE, "{id}");
        }
        // Its EFI partition (not mounted): nothing at all, not even Open, and the note on the rest.
        let efi = &d.partitions[0];
        assert!(partition_actions(&d, efi, "linux").is_empty());
        assert_eq!(why(&d, efi, "format"), HEARTH_NOTE);
        // Another system's EFI partition on Hearth's drive: also only Open and Properties.
        assert!(partition_actions(&d, &d.partitions[1], "linux").is_empty());
        // Every action is refused for them, also when asked for directly.
        for p in [root, efi, &d.partitions[1]] {
            for a in ["format", "delete", "extend", "shrink", "letter", "games", "unmount", "mount", "rename", "repair", "check"] {
                assert!(plan(a, &d, Some(p), "linux").refuse.is_some(), "{a} on {}", p.id);
                assert!(approve(a, &d, Some(p), "linux", &Opts { typed: volume_name(p), fs: "ext4".into(), size: GIB, letter: "F".into(), ..Default::default() }).is_err());
            }
        }
        // A start-up partition on a USB stick (an installer's EFI): protected too, with the start-up note.
        let mut u = usb();
        let mut e = part("ue", "efi", "vfat");
        e.protected = true;
        u.partitions.push(e.clone());
        assert!(partition_actions(&u, &e, "linux").is_empty());
        assert_eq!(why(&u, &e, "delete"), STARTUP_NOTE);
        assert!(plan("format", &u, Some(&e), "linux").refuse.is_some());
        // The whole stick can still be formatted.
        assert!(plan("format_drive", &u, None, "linux").refuse.is_none());
    }

    #[test]
    fn system_kinds_are_never_offered() {
        // What Hearth runs from, however it's built: plain, LVM, RAID, encrypted, start-up files, swap in use.
        let d = system_drive();
        for (kind, fs) in [("hearth", "btrfs"), ("linux", "LVM2_member"), ("unknown", "linux_raid_member"), ("encrypted", "crypto_LUKS"),
                           ("boot", "ext4"), ("swap", "swap"), ("efi", "vfat")] {
            let mut p = part("s", kind, fs);
            p.system = true;
            assert!(partition_actions(&d, &p, "linux").iter().all(|a| a.id == "open"), "{kind}/{fs}");
            for a in ["format", "delete", "extend", "shrink", "letter", "games", "unmount"] {
                assert!(plan(a, &d, Some(&p), "linux").refuse.is_some(), "{a} on {kind}/{fs}");
            }
            // On another drive too (a second device of Hearth's file system).
            let mut other = data_drive("ext4");
            other.partitions[0] = p.clone();
            assert!(partition_actions(&other, &p, "linux").iter().all(|a| a.id == "open"), "{kind}/{fs} elsewhere");
        }
    }

    #[test]
    fn normal_volume_menu() {
        let d = data_drive("ext4");
        let p = &d.partitions[0];
        assert_eq!(ids(&partition_actions(&d, p, "linux")),
                   vec!["open", "letter", "unmount", "games", "rename", "check", "repair", "format", "extend", "shrink", "delete"]);
        assert!(partition_choices(&d, p, "linux").iter().all(|a| a.why.is_empty()));
        // Unallocated space: New Simple Volume and New volume for games.
        assert_eq!(ids(&partition_actions(&d, &d.partitions[1], "linux")), vec!["create", "games"]);
        assert_eq!(before_free(&d, &d.partitions[1]).map(|x| x.id.as_str()), Some("v"));
        assert_eq!(free_after(&d, p).map(|x| x.size), Some(40 * GIB));
        // Windows: no Hearth letters, no Extend or Shrink in the menu at all.
        let w = ids(&partition_choices(&d, p, "windows")).into_iter().map(str::to_string).collect::<Vec<_>>();
        assert!(!w.iter().any(|a| a == "letter" || a == "extend" || a == "shrink"), "{w:?}");
        // A read-only drive: greyed out with why.
        let mut ro = d.clone();
        ro.read_only = true;
        assert_eq!(why(&ro, &ro.partitions[0], "format"), "This drive is read-only.");
        assert_eq!(why(&ro, &ro.partitions[1], "create"), "This drive is read-only.");
    }

    #[test]
    fn what_can_be_resized() {
        let d = data_drive("ext4");
        assert!(resize_block(&d, &d.partitions[0], true).is_none());
        assert!(resize_block(&d, &d.partitions[0], false).is_none());
        // exFAT and FAT32 can't be resized; xfs only grows.
        for fs in ["exfat", "vfat"] {
            let d = data_drive(fs);
            assert!(why(&d, &d.partitions[0], "extend").contains("can't be made bigger"), "{fs}");
            assert!(why(&d, &d.partitions[0], "shrink").contains("can't be made smaller"), "{fs}");
        }
        let d = data_drive("xfs");
        assert!(why(&d, &d.partitions[0], "extend").is_empty());
        assert!(why(&d, &d.partitions[0], "shrink").contains("only be made bigger"));
        // NTFS needs ntfsresize: greyed out with why when it isn't there.
        let mut d = data_drive("ntfs");
        d.partitions[0].resize_tool = "ntfsresize".into();
        assert!(why(&d, &d.partitions[0], "shrink").contains("ntfsresize"));
        assert!(plan("shrink", &d, Some(&d.partitions[0]), "linux").refuse.is_some());
        // No unallocated space right after it: Extend is greyed out.
        let mut d = data_drive("ext4");
        d.partitions.pop();
        assert!(why(&d, &d.partitions[0], "extend").contains("no unallocated space"));
        // Unmounted: how full it is isn't known, so Shrink waits; btrfs grows only while mounted.
        let mut d = data_drive("ext4");
        d.partitions[0].mounted = false; d.partitions[0].mount_points.clear(); d.partitions[0].free = None;
        assert!(why(&d, &d.partitions[0], "shrink").contains("Mount it first"));
        assert!(why(&d, &d.partitions[0], "extend").is_empty(), "ext4 grows unmounted");
        let mut b = data_drive("btrfs");
        b.partitions[0].mounted = false; b.partitions[0].mount_points.clear();
        assert!(why(&b, &b.partitions[0], "extend").contains("Mount it first"));
        // Never on the drive Hearth runs from, never a logical partition or a whole-drive file system.
        let s = system_drive();
        let mut data = s.partitions[3].clone();
        data.number = 4; data.free = Some(5 * GIB); data.mounted = true;
        assert!(why(&s, &data, "shrink").contains("Hearth runs from this drive"));
        assert!(plan("extend", &s, Some(&data), "linux").refuse.is_some());
        let mut d = data_drive("ext4");
        d.table = "dos".into(); d.partitions[0].number = 5; d.partitions[0].part_type = "0x83".into();
        assert!(why(&d, &d.partitions[0], "shrink").contains("extended"));
        let mut d = data_drive("ext4");
        d.partitions[0].number = 0;
        assert!(why(&d, &d.partitions[0], "shrink").contains("whole drive"));
    }

    #[test]
    fn games_drives_keep_10_gb() {
        let mut d = data_drive("ext4");
        let p = &mut d.partitions[0];
        p.steam_library = true; p.size = 100 * GIB; p.free = Some(12 * GIB);
        assert_eq!(shrink_max(p), Some(2 * GIB));
        p.free = Some(9 * GIB);
        assert_eq!(shrink_max(p), Some(0));
        let p = d.partitions[0].clone();
        assert!(why(&d, &p, "shrink").contains("10 GB"));
        // An ordinary volume keeps a twentieth (16 MB to 1 GB).
        let d = data_drive("ext4");
        assert_eq!(keep_free(&d.partitions[0]), (10 * GIB / 20).min(GIB));
        assert_eq!(shrink_max(&d.partitions[0]), Some((5 * GIB - 512 * MIB) / MIB * MIB));
    }

    #[test]
    fn extend_and_shrink_plans() {
        let d = data_drive("ext4");
        let p = &d.partitions[0];
        let e = plan("extend", &d, Some(p), "linux");
        assert!(e.refuse.is_none());
        assert_eq!((e.resize_size, e.resize_max, e.offline), (10 * GIB, 40 * GIB, false), "ext4 grows while mounted");
        let s = plan("shrink", &d, Some(p), "linux");
        assert!(s.refuse.is_none());
        assert!(s.offline, "ext4 shrinks unmounted");
        assert_eq!(s.resize_max, shrink_max(p).unwrap());
        let o = |size| Opts { size, expect: e.fingerprint.clone(), ..Default::default() };
        assert!(approve("extend", &d, Some(p), "linux", &o(GIB)).is_ok());
        assert!(approve("extend", &d, Some(p), "linux", &o(41 * GIB)).is_err(), "more than the free space after it");
        assert!(approve("extend", &d, Some(p), "linux", &o(MIB)).is_err(), "too small");
        assert!(approve("shrink", &d, Some(p), "linux", &Opts { size: 6 * GIB, ..Default::default() }).is_err(), "more than it can give up");
        assert!(approve("shrink", &d, Some(p), "linux", &Opts { size: GIB, ..Default::default() }).is_ok());
        let b = data_drive("btrfs");
        assert!(!plan("shrink", &b, Some(&b.partitions[0]), "linux").offline, "btrfs shrinks while mounted");
        // A swapped drive is refused for a resize too.
        let mut other = data_drive("ext4");
        other.serial = "OTHER".into();
        assert!(approve("extend", &other, Some(&other.partitions[0]), "linux", &o(GIB)).is_err());
    }

    #[test]
    fn names_and_letters() {
        let d = data_drive("ext4");
        let mut p = d.partitions[0].clone();
        assert_eq!(volume_name(&p), "Media");
        p.label.clear();
        assert_eq!(volume_name(&p), "sdb1");
        let del = plan("delete", &d, Some(&d.partitions[0]), "linux");
        assert_eq!((del.typed.as_deref(), del.typed_exact, del.danger), (Some("Media"), true, true));
        assert!(approve("delete", &d, Some(&d.partitions[0]), "linux", &Opts { typed: "media".into(), ..Default::default() }).is_err());
        assert!(approve("delete", &d, Some(&d.partitions[0]), "linux", &Opts { typed: " Media ".into(), ..Default::default() }).is_ok());
        // Change drive letter: D: to Y: only (C: is Hearth, Z: is Windows apps' view of the system).
        let o = |l: &str| Opts { letter: l.into(), ..Default::default() };
        assert!(approve("letter", &d, Some(&d.partitions[0]), "linux", &o("F")).is_ok());
        for l in ["C", "Z", "", "FF", "1"] { assert!(approve("letter", &d, Some(&d.partitions[0]), "linux", &o(l)).is_err(), "{l}"); }
        assert!(letter_ok("e:") && !letter_ok("c:"));
        // New volume choices: ext4 first, FAT32 only up to 32 GB.
        let v = volume_fs_choices("linux", 40 * GIB);
        assert_eq!(v.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), vec!["ext4", "ntfs", "exfat", "btrfs"]);
        assert!(volume_fs_choices("linux", 16 * GIB).iter().any(|c| c.id == "vfat"));
        assert_eq!(mb(307_200 * MIB), "307,200 MB");
        assert_eq!(mb(512 * MIB), "512 MB");
        assert_eq!(fs_name("vfat"), "FAT32");
    }

    #[test]
    fn full_check_on_any_drive() {
        let mut d = system_drive();
        d.health.smart = true;
        assert_eq!(ids(&drive_actions(&d, "linux")), vec!["selftest"]);
        assert!(plan("selftest", &d, None, "linux").refuse.is_none());
        assert!(plan("format_drive", &d, None, "linux").refuse.is_some());
        assert!(drive_actions(&d, "windows").is_empty());
    }
}
