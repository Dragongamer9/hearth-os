//! Against a real drive, through udisks2: a disk image set up with `udisksctl loop-setup` (CI does this, see
//! .github/workflows/disks.yml), given as HEARTH_DISKS_LOOP=/dev/loopN. Without it the test does nothing.
//! It lists the image, formats it, mounts and unmounts it, renames, checks, deletes and makes partitions, sets
//! one up for games (with a stand-in Steam folder) and finally removes the image, all as Disks does it.
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

    let d = image(&dev);
    eprintln!("{}", serde_json::to_string_pretty(&d).unwrap());
    assert_eq!(d.kind, "image");
    assert!(!d.system);
    assert!(d.actions.iter().any(|a| a.id == "format_drive"));

    // Format the whole image as exFAT: one partition, mounted (it counts as removable).
    run("format_drive", &d.id, opts("exfat", "HEARTHTEST"));
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

    // Delete it, then make a 64 MB ext4 partition at the start of the free space.
    run("delete", &p.id, Opts::default());
    let d = image(&dev);
    let free = d.partitions.iter().find(|x| x.kind == "free").expect("free space after deleting").clone();
    run("create", &free.id, Opts { size: 64 << 20, ..opts("ext4", "PART1") });
    let d = image(&dev);
    let p1 = d.partitions.iter().find(|x| x.label == "PART1").expect("the new partition").clone();
    assert_eq!(p1.fs, "ext4");

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
