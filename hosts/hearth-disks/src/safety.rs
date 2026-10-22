//! What Disks lets you do, and what it says first. Every action is checked here twice: when the page draws the
//! choices, and again against a fresh look at the drives right before anything changes (lib.rs `prepare`).
//! The rules:
//!  - The running system's own partitions (Hearth OS's root, start-up and EFI partitions, swap in use; Windows'
//!    C:, EFI, reserved and recovery partitions) are never changed, unmounted or formatted.
//!  - The system's drive can't be formatted as a whole. Other changes on it need the word typed in.
//!  - Start-up and recovery partitions on other drives, internal drives, and drives with another system on them
//!    also need the word typed in before they're erased.
//!  - Before anything is erased the page shows exactly what will be lost.

use crate::model::{human, Action, Drive, Partition};
use serde::{Deserialize, Serialize};

/// What the confirmation asks you to type for an erase that needs it.
pub const ERASE_WORD: &str = "ERASE";
/// ...and for a change on the system's drive that doesn't erase anything.
pub const CHANGE_WORD: &str = "CHANGE";

/// Options from the page for one action.
#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Opts {
    pub fs: String,
    pub label: String,
    /// Size in bytes for a new partition (0: all the free space).
    pub size: u64,
    /// What was typed in the confirmation.
    pub typed: String,
    /// The plan's `fingerprint` from when it was shown: the run is refused if the drive is no longer the same one
    /// (a USB stick swapped for another gets the same device path). Empty: not checked.
    pub expect: String,
}

/// What the page shows before an action runs.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub action: String,
    pub title: String,
    /// Why it can't be done (then nothing else matters).
    pub refuse: Option<String>,
    /// The word to type before it runs, if one is needed.
    pub typed: Option<String>,
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

fn act(id: &str, label: &str, danger: bool) -> Action { Action { id: id.into(), label: label.into(), danger } }

/// Another system lives on this drive (erasing it would remove that system).
fn has_other_os(d: &Drive) -> bool { d.partitions.iter().any(|p| matches!(p.kind.as_str(), "windows" | "steamos" | "hearth")) }

/// What can be done with a whole drive.
pub fn drive_actions(d: &Drive, host: &str) -> Vec<Action> {
    let mut v = vec![];
    if d.system { return v; }
    if d.removable || d.can_power_off || d.can_eject { v.push(act("eject", "Safely remove", false)); }
    if !d.read_only && d.kind != "optical" {
        v.push(act("games", "Use for games", true));
        v.push(act("format_drive", if d.kind == "sd" { "Format SD card" } else { "Format drive" }, true));
    }
    let _ = host;
    v
}

/// What can be done with one partition or stretch of free space.
pub fn partition_actions(d: &Drive, p: &Partition, host: &str) -> Vec<Action> {
    let mut v = vec![];
    if p.kind == "free" {
        if !d.read_only {
            v.push(act("create", "Make a partition here", false));
            v.push(act("games", "Use for games", false));
        }
        return v;
    }
    let win = host == "windows";
    if p.mounted && (!p.mount_points.is_empty() || !p.letter.is_empty()) { v.push(act("open", "Open files", false)); }
    if p.system { return v; }
    if p.mountable && !p.mounted { v.push(act("mount", if win { "Give it a drive letter" } else { "Mount" }, false)); }
    if p.mountable && p.mounted { v.push(act("unmount", if win { "Remove its drive letter" } else { "Unmount" }, false)); }
    if d.read_only { return v; }
    let guarded = p.protected && d.system;
    if p.mountable && !guarded {
        if !p.steam_library { v.push(act("games", "Use for games", !games_ready(host, &p.fs))); }
        v.push(act("rename", "Rename", false));
        v.push(act("check", "Check for problems", false));
        v.push(act("repair", "Repair", false));
    }
    if !guarded && p.kind != "extended" {
        v.push(act("format", "Format", true));
        v.push(act("delete", "Delete partition", true));
    }
    v
}

/// Fills in every drive's and partition's actions.
pub fn annotate(drives: &mut [Drive], host: &str) {
    for d in drives.iter_mut() {
        d.actions = drive_actions(d, host);
        let snapshot = d.clone();
        for p in d.partitions.iter_mut() { p.actions = partition_actions(&snapshot, p, host); }
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

/// What an action will do, or why it can't. `p`: the partition or free space (None: the whole drive).
pub fn plan(action: &str, d: &Drive, p: Option<&Partition>, host: &str) -> Plan {
    let mut plan = Plan { action: action.into(), confirm: "OK".into(), fingerprint: fingerprint(d, p), ..Default::default() };
    let offered = match p { Some(p) => partition_actions(d, p, host), None => drive_actions(d, host) };
    let refuse = |plan: &mut Plan, why: &str| { plan.refuse = Some(why.into()); };
    let sys_name = if host == "windows" { "Windows" } else { "Hearth OS" };
    if let Some(p) = p {
        if p.system && action != "open" {
            refuse(&mut plan, &format!("{sys_name} is running from this partition, so Disks won't change it."));
            return plan;
        }
        if p.protected && d.system && matches!(action, "format" | "delete" | "games" | "rename" | "repair") {
            refuse(&mut plan, "Your PC needs this partition to start up, so Disks won't change it.");
            return plan;
        }
    } else if d.system && action != "open" {
        refuse(&mut plan, &format!("{sys_name} is running from this drive, so Disks won't erase it or remove it."));
        return plan;
    }
    if !offered.iter().any(|a| a.id == action) {
        refuse(&mut plan, "That can't be done here.");
        return plan;
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
            plan.confirm = if fmt { "Erase and format".into() } else { "Delete".into() };
            if fmt {
                plan.fs_choices = fs_choices(host, false, p.size);
                plan.ask_label = true;
                plan.label_default = p.label.clone();
            }
            if d.system || p.protected || !d.removable || matches!(p.kind.as_str(), "windows" | "steamos" | "hearth" | "encrypted") {
                plan.typed = Some(ERASE_WORD.into());
            }
            if d.system { plan.warn.insert(0, system_drive_note.clone()); }
            if p.protected { plan.warn.push("This is a start-up or recovery partition. Another system on this drive may stop starting without it.".into()); }
            if p.kind == "windows" { plan.warn.push("This looks like Windows. Erasing it removes that Windows and everything in it.".into()); }
            if p.kind == "encrypted" { plan.warn.push("It's locked, so Disks can't tell what's in it. All of it will be lost.".into()); }
        }
        ("create", Some(p)) => {
            plan.title = format!("Make a partition in {} of free space", human(p.size));
            plan.fs_choices = fs_choices(host, false, p.size);
            plan.ask_label = true;
            plan.max_size = p.size;
            plan.confirm = "Make partition".into();
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

/// The final check before running: the plan again (from a fresh look at the drives), the typed word, the
/// options. Returns the plan that's going to run.
pub fn approve(action: &str, d: &Drive, p: Option<&Partition>, host: &str, o: &Opts) -> Result<Plan, String> {
    let plan = plan(action, d, p, host);
    if let Some(why) = &plan.refuse { return Err(why.clone()); }
    if !o.expect.is_empty() && o.expect != plan.fingerprint {
        return Err("This isn't the same drive or partition you confirmed (it may have been swapped or changed). Nothing was changed. Look at it again and choose what to do.".into());
    }
    if let Some(word) = &plan.typed {
        if o.typed.trim().to_uppercase() != *word { return Err(format!("Type {word} to confirm.")); }
    }
    if !plan.fs_choices.is_empty() && !plan.fs_choices.iter().any(|c| c.id == o.fs) {
        return Err("Choose a format from the list.".into());
    }
    if plan.ask_label {
        let fs = if o.fs.is_empty() { p.map(|p| p.fs.clone()).unwrap_or_default() } else { o.fs.clone() };
        check_label(&fs, &o.label)?;
    }
    if plan.max_size > 0 && o.size > plan.max_size { return Err("That's more than the free space.".into()); }
    if plan.max_size > 0 && o.size != 0 && o.size < 32 * crate::model::MIB { return Err("Make it at least 32 MB.".into()); }
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
        assert_eq!(pl.typed.as_deref(), Some(ERASE_WORD));
        assert!(!pl.lose.is_empty());
        let o = Opts { fs: "ext4".into(), ..Default::default() };
        assert!(approve("format", &d, Some(data), "linux", &o).is_err());
        assert!(approve("format", &d, Some(data), "linux", &Opts { typed: "erase".into(), ..o.clone() }).is_ok());
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
        assert_eq!(plan("delete", &d, Some(&d.partitions[1]), "linux").typed.as_deref(), Some(ERASE_WORD));
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
        let o = Opts { fs: "exfat".into(), expect: pl.fingerprint.clone(), ..Default::default() };
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
}
