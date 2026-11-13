//! Against a real drive, through udisks2: a disk image set up with `udisksctl loop-setup` (CI does this, see
//! .github/workflows/disks.yml), given as HEARTH_DISKS_LOOP=/dev/loopN. Without it the test does nothing.
//! It lists the image, formats it, mounts and unmounts it, renames, checks, deletes and makes partitions, sets
//! one up for games (with a stand-in Steam folder) and finally removes the image, all as Disks does it. Disk
//! Management's own: a drive letter through hearth_drive_letters.py, Extend into the free space and Shrink (ext4,
//! unmounted, checked first), and that a delete or format needs the volume's name typed exactly.
#![cfg(target_os = "linux")]

use hearth_disks::safety::Opts;
use hearth_disks::model::Drive;

fn image(dev: &str) -> Drive {
    let drives = hearth_disks::list().expect("listing the drives through udisks2");
    drives.into_iter().find(|d| d.device == dev).unwrap_or_else(|| panic!("{dev} isn't listed"))
}

fn run(action: &str, id: &str, o: Opts) -> String {
    let progress = |t: &str, p: Option<f64>| eprintln!("  {action}: {t} {}", p.map(|p| format!("{:.0}%", p * 100.0)).unwrap_or_default());
    match hearth_disks::run(action, id, &o, &progress) {
        Ok(m) => { eprintln!("{action}: {m}"); m }
        Err(e) => panic!("{action} failed: {e}"),
    }
}

fn opts(fs: &str, label: &str) -> Opts { Opts { fs: fs.into(), label: label.into(), ..Default::default() } }

#[test]
fn loop_device_through_udisks() {
    let Ok(dev) = std::env::var("HEARTH_DISKS_LOOP") else { eprintln!("HEARTH_DISKS_LOOP isn't set: skipped"); return };
    // A stand-in Steam, so "Use for games" has a libraryfolders.vdf to add to.
    let home = std::env::temp_dir().join(format!("hearth-disks-home-{}", std::process::id()));
    std::fs::create_dir_all(home.join(".local/share/Steam/steamapps")).unwrap();
    std::env::set_var("HOME", &home);
    std::env::remove_var("XDG_CONFIG_HOME");
    // hearth_drive_letters.py from this repository (Hearth OS has it in /usr/share/nimbus/desktop/shell).
    std::env::set_var("HEARTH_DRIVE_LETTERS_PY", concat!(env!("CARGO_MANIFEST_DIR"), "/../linux/desktop/shell/hearth_drive_letters.py"));

    let d = image(&dev);
    eprintln!("{}", serde_json::to_string_pretty(&d).unwrap());
    assert_eq!(d.kind, "image");
    assert!(!d.system);
    assert!(d.actions.iter().any(|a| a.id == "format_drive"));

    // Format the whole image as exFAT: one partition, mounted (it counts as removable).
    let m = run("format_drive", &d.id, opts("exfat", "HEARTHTEST"));
    assert!(!m.contains("couldn't be mounted"), "{m}");
    let d = image(&dev);
    let parts: Vec<_> = d.partitions.iter().filter(|p| p.kind != "free").collect();
    assert_eq!(parts.len(), 1, "{:?}", d.partitions);
    let p = parts[0].clone();
    assert_eq!((p.fs.as_str(), p.label.as_str(), d.table.as_str()), ("exfat", "HEARTHTEST", "dos"));
    assert!(p.mounted, "it's mounted after formatting");
    assert!(p.used.is_some());

    run("unmount", &p.id, Opts::default());
    assert!(!image(&dev).partitions.iter().any(|x| x.mounted));
    run("mount", &p.id, Opts::default());
    run("rename", &p.id, opts("", "RENAMED"));
    let d = image(&dev);
    assert!(d.partitions.iter().any(|x| x.label == "RENAMED"));
    let m = run("check", &p.id, Opts::default());
    assert_eq!(m, "No problems found.");

    // Formatting asks for the file system it knows, and refuses others.
    assert!(hearth_disks::run("format", &p.id, &opts("zfs", "x"), &|_: &str, _: Option<f64>| {}).is_err());

    // Delete it (its name typed exactly), then make a 64 MB ext4 partition at the start of the free space.
    assert!(hearth_disks::run("delete", &p.id, &Opts { typed: "renamed".into(), ..Default::default() }, &|_: &str, _: Option<f64>| {}).is_err(), "the name, exactly");
    run("delete", &p.id, Opts { typed: "RENAMED".into(), ..Default::default() });
    let d = image(&dev);
    let free = d.partitions.iter().find(|x| x.kind == "free").expect("free space after deleting").clone();
    run("create", &free.id, Opts { size: 64 << 20, ..opts("ext4", "PART1") });
    let d = image(&dev);
    let p1 = d.partitions.iter().find(|x| x.label == "PART1").expect("the new partition").clone();
    assert_eq!(p1.fs, "ext4");
    assert!(p1.mounted, "a new volume on a removable drive is mounted");

    // A drive letter, Hearth's way (hearth_drive_letters.py keeps it in ~/.config/hearth/drive-letters.json).
    let lp = hearth_disks::plan("letter", &p1.id).unwrap();
    assert!(lp.refuse.is_none(), "{:?}", lp.refuse);
    assert!(lp.letters.contains(&"F".to_string()), "{:?}", lp.letters);
    run("letter", &p1.id, Opts { letter: "F".into(), ..Default::default() });
    assert_eq!(image(&dev).partitions.iter().find(|x| x.label == "PART1").unwrap().letter, "F");
    assert!(home.join(".config/hearth/drive-letters.json").is_file());
    assert!(hearth_disks::run("letter", &p1.id, &Opts { letter: "C".into(), ..Default::default() }, &|_: &str, _: Option<f64>| {}).is_err(), "C: is Hearth's");

    // Extend PART1 by 32 MB into the free space right after it (ext4 grows while mounted)...
    let ep = hearth_disks::plan("extend", &p1.id).unwrap();
    assert!(ep.refuse.is_none(), "{:?}", ep.refuse);
    assert!(ep.resize_max >= 64 << 20, "the free space after it: {}", ep.resize_max);
    run("extend", &p1.id, Opts { size: 32 << 20, expect: ep.fingerprint.clone(), ..Default::default() });
    let d = image(&dev);
    let big = d.partitions.iter().find(|x| x.label == "PART1").expect("PART1 after extending").clone();
    assert!(big.size >= p1.size + (31 << 20), "{} -> {}", p1.size, big.size);
    assert!(big.mounted);
    let fs_size = big.used.unwrap_or(0) + big.free.unwrap_or(0);
    assert!(fs_size > p1.used.unwrap_or(0) + p1.free.unwrap_or(0) + (20 << 20), "the file system grew too");
    std::fs::write(std::path::Path::new(&big.mount_points[0]).join("keep.txt"), "still here").unwrap();
    // ...then shrink it by 32 MB again (unmounted, checked, mounted again), and the file is still there.
    let sp = hearth_disks::plan("shrink", &big.id).unwrap();
    assert!(sp.refuse.is_none(), "{:?}", sp.refuse);
    assert!(sp.offline, "ext4 shrinks unmounted");
    assert!(sp.resize_max >= 32 << 20, "it can give up 32 MB: {}", sp.resize_max);
    assert!(hearth_disks::run("shrink", &big.id, &Opts { size: sp.resize_max + (64 << 20), ..Default::default() }, &|_: &str, _: Option<f64>| {}).is_err(), "not more than it can give up");
    run("shrink", &big.id, Opts { size: 32 << 20, expect: sp.fingerprint.clone(), ..Default::default() });
    let d = image(&dev);
    let small = d.partitions.iter().find(|x| x.label == "PART1").expect("PART1 after shrinking").clone();
    assert!(small.size <= big.size - (31 << 20), "{} -> {}", big.size, small.size);
    assert!(small.mounted, "mounted again after shrinking");
    assert_eq!(std::fs::read_to_string(std::path::Path::new(&small.mount_points[0]).join("keep.txt")).unwrap(), "still here");
    let m = run("check", &small.id, Opts::default());
    assert_eq!(m, "No problems found.", "the file system is clean after shrinking");
    let d = image(&dev);
    let p1 = d.partitions.iter().find(|x| x.label == "PART1").unwrap().clone();
    // Formatting it asks for its name exactly too.
    assert!(hearth_disks::run("format", &p1.id, &opts("ext4", "PART1"), &|_: &str, _: Option<f64>| {}).is_err());
    assert_eq!(hearth_disks::plan("format", &p1.id).unwrap().typed.as_deref(), Some("PART1"));

    // Use the rest for games: a partition, mounted by itself (fstab through udisks), and a Steam library on it.
    let free = d.partitions.iter().find(|x| x.kind == "free").expect("free space left").clone();
    run("games", &free.id, opts("ext4", "CIGames"));
    let d = image(&dev);
    let g = d.partitions.iter().find(|x| x.label == "CIGames").expect("the games partition").clone();
    assert!(g.mounted && g.mount_points[0].starts_with("/var/mnt/games-"), "{:?}", g.mount_points);
    assert!(g.steam_library, "it's seen as a Steam library");
    let vdf = std::fs::read_to_string(home.join(".local/share/Steam/steamapps/libraryfolders.vdf")).unwrap();
    assert!(vdf.contains(&format!("{}/SteamLibrary", g.mount_points[0])), "{vdf}");
    assert!(std::path::Path::new(&g.mount_points[0]).join("SteamLibrary/steamapps").is_dir());

    // Safely remove: everything unmounted and the image let go.
    run("eject", &d.id, Opts::default());
    assert!(hearth_disks::list().unwrap().iter().all(|x| x.device != dev), "the image is gone");
}
