//! Make a bootable USB end to end against a disk image (a loop device set up with udisksctl, HEARTH_USB_LOOP), the
//! way CI runs it (.github/workflows/disks.yml): downloads from a local web server (HEARTH_USB_SERVER) with a
//! checksum file signed by a test key (HEARTH_USB_TEST_KEY, HEARTH_USB_TEST_FPR), the helper run through sudo
//! (HEARTH_DISKS_ADMIN), and fake images made by the workflow in HEARTH_USB_FIXTURES:
//!   good/fake-linux-1.iso  + SHA256SUMS + SHA256SUMS.gpg   a hybrid ISO (grub-mkrescue)
//!   tampered/...           the same, but SHA256SUMS (validly signed) has another checksum
//!   badsig/...             SHA256SUMS.gpg made by another key
//!   win/fake-win11.iso     a non-hybrid ISO with bootmgr, efi/boot/bootx64.efi and a 12 MB sources/install.wim
//!   uefi/fake-uefi.iso     a non-hybrid ISO with only EFI start-up files
//!   own/disk.img.zst       a compressed disk image with disk.img.zst.sha256 beside it
//! The workflow checks what landed on the image afterwards (partition table, FAT32, the files). Without
//! HEARTH_USB_LOOP each test does nothing.
#![cfg(target_os = "linux")]

use hearth_disks::usb::{self, catalog, Job};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

fn env(k: &str) -> Option<String> { std::env::var(k).ok().filter(|v| !v.is_empty()) }

fn setup() -> Option<(String, PathBuf)> {
    let dev = env("HEARTH_USB_LOOP")?;
    let fx = PathBuf::from(env("HEARTH_USB_FIXTURES").expect("HEARTH_USB_FIXTURES"));
    std::env::set_var("HEARTH_USB_ALLOW_LOOP", "1");
    std::env::set_var("HEARTH_USB_TEST_HTTP", "1");
    let home = std::env::temp_dir().join("hearth-usb-home");
    std::fs::create_dir_all(home.join("Downloads")).unwrap();
    std::env::set_var("HOME", &home);
    Some((dev, fx))
}

/// A command as root (CI's runner has sudo without a password): its output.
fn sudo(args: &[&str]) -> String {
    let o = std::process::Command::new("sudo").arg("-n").args(args).output().expect("sudo");
    String::from_utf8_lossy(&o.stdout).into_owned() + &String::from_utf8_lossy(&o.stderr)
}

/// The disk image's first `len` bytes are exactly `file`.
fn same_bytes(dev: &str, file: &std::path::Path) {
    let len = std::fs::metadata(file).unwrap().len().to_string();
    let o = std::process::Command::new("sudo").args(["-n", "cmp", "-n", &len]).arg(file).arg(dev).output().unwrap();
    assert!(o.status.success(), "{dev} doesn't hold {}: {}", file.display(), String::from_utf8_lossy(&o.stdout));
}

/// The partition table as sfdisk sees it.
fn table(dev: &str) -> Value {
    let _ = std::process::Command::new("udevadm").arg("settle").status();
    let o = std::process::Command::new("sudo").args(["-n", "sfdisk", "--json", dev]).output().expect("sfdisk");
    serde_json::from_slice::<Value>(&o.stdout).map(|v| v["partitiontable"].clone())
        .unwrap_or_else(|e| panic!("sfdisk --json {dev}: {e}: {}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
}

/// Mounts the drive's first partition (as you, through udisks), runs `f` on it, unmounts it.
fn with_mounted(dev: &str, f: impl FnOnce(&std::path::Path)) {
    let part = format!("{dev}p1");
    let _ = std::process::Command::new("udevadm").arg("settle").status();
    let o = std::process::Command::new("udisksctl").args(["mount", "--no-user-interaction", "-o", "ro", "-b", &part]).output().unwrap();
    let t = String::from_utf8_lossy(&o.stdout).to_string();
    let at = t.split(" at ").nth(1).map(|s| s.trim().trim_end_matches('.').to_string()).unwrap_or_else(|| panic!("couldn't mount {part}: {t}{}", String::from_utf8_lossy(&o.stderr)));
    f(std::path::Path::new(&at));
    let _ = std::process::Command::new("udisksctl").args(["unmount", "--no-user-interaction", "-b", &part]).output();
}

/// The disk image as Make a bootable USB offers it (looked at again each time: its partitions change).
fn drive(dev: &str) -> usb::UsbDrive {
    let _ = std::process::Command::new("udevadm").arg("settle").status();
    std::thread::sleep(std::time::Duration::from_millis(1500));
    let d = usb::drives(false).expect("listing drives");
    d.into_iter().find(|d| d.device == dev).unwrap_or_else(|| panic!("{dev} isn't offered"))
}

fn test_catalog(dir: &str) -> catalog::Catalog {
    let server = env("HEARTH_USB_SERVER").expect("HEARTH_USB_SERVER");
    let fpr = env("HEARTH_USB_TEST_FPR").expect("HEARTH_USB_TEST_FPR");
    let json = serde_json::json!({
        "schema": 1, "groups": [{"id": "linux", "name": "Linux"}],
        "keys": {"test": {"file": "test.asc", "owner": "Hearth CI test key", "fingerprints": [fpr], "source": "made by CI"}},
        "entries": [{
            "id": "fake", "group": "linux", "family": "Fake", "name": "Fake Linux", "homepage": "https://example.org/",
            "resolve": {"kind": "dir", "dir": format!("{server}/{dir}/")},
            "checksums": {"file": "{dir}SHA256SUMS", "pick": "^fake-linux-(\\d+)\\.iso$"},
            "signature": {"kind": "detached", "of": "checksums", "file": "{dir}SHA256SUMS.gpg", "key": "test"}
        }]
    });
    serde_json::from_value(json).expect("test catalog")
}

fn test_key(file: &str) -> Option<String> {
    if file != "test.asc" { return None; }
    std::fs::read_to_string(env("HEARTH_USB_TEST_KEY").expect("HEARTH_USB_TEST_KEY")).ok()
}

fn emit() -> usb::Emit {
    Arc::new(|v: Value| {
        if v["pct"].is_null() || v["stage"] != "download" { eprintln!("  {} {} {}", v["stage"], v["text"], v["pct"]); }
    })
}

fn job_for(d: &usb::UsbDrive) -> Job { Job { drive: d.id.clone(), expect: d.fingerprint.clone(), keep: true, ..Default::default() } }

#[test]
fn a_download_check_and_write_a_hybrid_iso() {
    let Some((dev, fx)) = setup() else { eprintln!("HEARTH_USB_LOOP isn't set: skipped"); return };
    let d = drive(&dev);
    let job = Job { source: "catalog".into(), id: "fake".into(), ..job_for(&d) };
    let r = usb::run_in(&job, &test_catalog("good"), &test_key, emit()).unwrap_or_else(|e| panic!("writing failed: {e}"));
    eprintln!("{r:#}");
    let iso = std::fs::read(fx.join("good/fake-linux-1.iso")).unwrap();
    assert_eq!(r["summary"]["mode"], "raw");
    assert_eq!(r["summary"]["sha256"].as_str().unwrap(), usb::verify::sha256_bytes(&iso));
    assert!(r["verifiedBy"].as_str().unwrap().contains("signed by Hearth CI test key"), "{}", r["verifiedBy"]);
    same_bytes(&dev, &fx.join("good/fake-linux-1.iso"));
    // A hybrid ISO brings its own partition table.
    assert!(table(&dev)["partitions"].as_array().map(|p| !p.is_empty()).unwrap_or(false), "no partitions after writing the hybrid ISO");
    // Downloaded into ~/Downloads/Hearth USB, kept, with what checked it.
    let kept = usb::cache_dir().join("fake-linux-1.iso");
    assert!(kept.is_file(), "the download is kept");
    assert!(usb::cache_list().iter().any(|f| f["sha256"] == r["summary"]["sha256"]));
    // A second time: the earlier download is used (no download), still checked.
    let d = drive(&dev);
    let job = Job { source: "catalog".into(), id: "fake".into(), keep: false, ..job_for(&d) };
    usb::run_in(&job, &test_catalog("good"), &test_key, emit()).unwrap_or_else(|e| panic!("second write failed: {e}"));
    assert!(!kept.exists(), "deleted after writing when not kept");
}

#[test]
fn b_a_download_that_doesnt_match_is_never_written() {
    let Some((dev, _)) = setup() else { return };
    let d = drive(&dev);
    let job = Job { source: "catalog".into(), id: "fake".into(), ..job_for(&d) };
    let e = usb::run_in(&job, &test_catalog("tampered"), &test_key, emit()).unwrap_err();
    assert!(e.contains("doesn't match"), "{e}");
    assert!(!usb::cache_dir().join("fake-linux-1.iso").exists() || std::fs::read(usb::cache_dir().join("fake-linux-1.iso.hearth.json")).is_err());
    let e = usb::run_in(&job, &test_catalog("badsig"), &test_key, emit()).unwrap_err();
    assert!(e.contains("signed") || e.contains("key"), "{e}");
    // The drive is as it was: still the hybrid ISO from test a (its fingerprint didn't change).
    assert_eq!(drive(&dev).fingerprint, d.fingerprint);
}

#[test]
fn c_a_swapped_drive_or_an_unconfirmed_file_is_refused() {
    let Some((dev, fx)) = setup() else { return };
    let d = drive(&dev);
    let job = Job { source: "file".into(), path: fx.join("uefi/fake-uefi.iso").to_string_lossy().into(), accept_unverified: true, expect: "something else".into(), ..job_for(&d) };
    assert!(usb::run(&job, emit()).unwrap_err().contains("isn't the same drive"));
    let job = Job { source: "file".into(), path: fx.join("uefi/fake-uefi.iso").to_string_lossy().into(), ..job_for(&d) };
    assert!(usb::run(&job, emit()).unwrap_err().contains("Confirm that you trust it"));
}

#[test]
fn d_windows_iso_gpt_split_and_windows_11_options() {
    let Some((dev, fx)) = setup() else { return };
    std::env::set_var("HEARTH_USB_SPLIT_MB", "4");
    let iso = fx.join("win/fake-win11.iso");
    let info = usb::inspect(&iso).expect("inspecting the fake Windows ISO");
    eprintln!("{info:#?}");
    assert_eq!(info.mode, "windows");
    assert!(!info.hybrid);
    let w = info.windows.clone().unwrap();
    assert_eq!(w.product, "Windows 11");
    assert!(w.needs_split, "12 MB is over the test's 4 MB limit");
    let d = drive(&dev);
    let job = Job { source: "file".into(), path: iso.to_string_lossy().into(), accept_unverified: true, scheme: "gpt".into(),
                    tweaks: usb::image::Tweaks { skip_checks: true, skip_online: true, local_account: "Hearth".into() }, ..job_for(&d) };
    let r = usb::run(&job, emit()).unwrap_or_else(|e| panic!("Windows write failed: {e}"));
    eprintln!("{r:#}");
    assert_eq!(r["summary"]["mode"], "windows");
    assert_eq!(r["summary"]["scheme"], "gpt");
    assert!(r["summary"]["swmParts"].as_u64().unwrap() >= 2, "{}", r["summary"]);
    assert_eq!(r["summary"]["unattend"], true);
    std::env::remove_var("HEARTH_USB_SPLIT_MB");
    // GPT with one Microsoft basic data partition, FAT32, and the files.
    let t = table(&dev);
    eprintln!("{t:#}");
    assert_eq!(t["label"], "gpt");
    let parts = t["partitions"].as_array().cloned().unwrap_or_default();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["type"].as_str().unwrap_or("").to_uppercase(), "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7");
    assert_eq!(sudo(&["blkid", "-o", "value", "-s", "TYPE", &format!("{dev}p1")]).trim(), "vfat");
    assert!(sudo(&["fsck.vfat", "-n", &format!("{dev}p1")]).contains("files"), "fsck.vfat");
    with_mounted(&dev, |m| {
        for f in ["bootmgr", "efi/boot/bootx64.efi", "sources/boot.wim", "sources/install.swm", "sources/install2.swm", "autounattend.xml"] {
            assert!(m.join(f).is_file(), "{f} is missing on the drive");
        }
        assert!(!m.join("sources/install.wim").exists(), "install.wim should have been split");
        let x = std::fs::read_to_string(m.join("autounattend.xml")).unwrap();
        assert!(x.contains("BypassTPMCheck") && x.contains("BypassNRO") && x.contains("<Name>Hearth</Name>"));
        // The parts make one readable WIM again.
        let o = std::process::Command::new("wimlib-imagex").arg("verify").arg(m.join("sources/install.swm")).arg(format!("--ref={}", m.join("sources/install*.swm").display())).output().unwrap();
        assert!(o.status.success(), "wimlib-imagex verify on the split WIM: {}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
        let o = std::process::Command::new("wiminfo").arg(m.join("sources/install.swm")).output().unwrap();
        assert!(String::from_utf8_lossy(&o.stdout).contains("Windows 11 Pro"), "{}", String::from_utf8_lossy(&o.stdout));
        let a = std::fs::read(fx.join("win/root/efi/boot/bootx64.efi")).unwrap();
        assert_eq!(std::fs::read(m.join("efi/boot/bootx64.efi")).unwrap(), a);
    });
}

#[test]
fn e_uefi_iso_copied_with_mbr() {
    let Some((dev, fx)) = setup() else { return };
    let iso = fx.join("uefi/fake-uefi.iso");
    let info = usb::inspect(&iso).unwrap();
    assert_eq!(info.mode, "uefi-copy");
    let d = drive(&dev);
    let job = Job { source: "file".into(), path: iso.to_string_lossy().into(), accept_unverified: true, scheme: "mbr".into(), ..job_for(&d) };
    let r = usb::run(&job, emit()).unwrap_or_else(|e| panic!("UEFI copy failed: {e}"));
    assert_eq!(r["summary"]["mode"], "uefi-copy");
    assert_eq!(r["summary"]["scheme"], "mbr");
    let t = table(&dev);
    eprintln!("{t:#}");
    assert_eq!(t["label"], "dos");
    let p = &t["partitions"][0];
    assert_eq!(p["bootable"], true);
    assert!(matches!(p["type"].as_str(), Some("c") | Some("b")), "FAT32 type: {p}");
    with_mounted(&dev, |m| assert!(m.join("efi/boot/bootx64.efi").is_file()));
}

#[test]
fn f_compressed_disk_image_with_its_checksum_beside_it() {
    let Some((dev, fx)) = setup() else { return };
    let img = fx.join("own/disk.img.zst");
    let checked = usb::check_file(&img.to_string_lossy(), &emit()).unwrap();
    assert_eq!(checked["verified"], true, "{checked}");
    assert_eq!(checked["inspect"]["mode"], "raw");
    let d = drive(&dev);
    let job = Job { source: "file".into(), path: img.to_string_lossy().into(), ..job_for(&d) };
    let r = usb::run(&job, emit()).unwrap_or_else(|e| panic!("compressed image failed: {e}"));
    assert!(r["verifiedBy"].as_str().unwrap().contains("disk.img.zst.sha256"), "{}", r["verifiedBy"]);
    let raw = std::fs::read(fx.join("own/disk.img")).unwrap();
    assert_eq!(r["summary"]["sha256"].as_str().unwrap(), usb::verify::sha256_bytes(&raw));
    same_bytes(&dev, &fx.join("own/disk.img"));
}

#[test]
fn g_ventoy_with_two_isos() {
    let Some((dev, fx)) = setup() else { return };
    let d = drive(&dev);
    let items = vec![
        usb::ventoy::Item { source: "file".into(), path: fx.join("good/fake-linux-1.iso").to_string_lossy().into(), ..Default::default() },
        usb::ventoy::Item { source: "file".into(), path: fx.join("uefi/fake-uefi.iso").to_string_lossy().into(), accept_unverified: true, ..Default::default() },
    ];
    // Unconfirmed own file: refused before anything is written.
    let bad = vec![usb::ventoy::Item { source: "file".into(), path: fx.join("uefi/fake-uefi.iso").to_string_lossy().into(), ..Default::default() }];
    let e = usb::run(&Job { ventoy: true, items: bad, ..job_for(&d) }, emit()).unwrap_err();
    assert!(e.contains("Confirm"), "{e}");
    let r = usb::run(&Job { ventoy: true, items, ..job_for(&d) }, emit()).unwrap_or_else(|e| panic!("Ventoy failed: {e}"));
    eprintln!("{r:#}");
    assert_eq!(r["summary"]["mode"], "ventoy");
    assert_eq!(r["summary"]["ventoy"], usb::ventoy::VERSION);
    assert_eq!(r["summary"]["isos"].as_array().map(|a| a.len()), Some(2));
    let t = table(&dev);
    eprintln!("{t:#}");
    assert_eq!(t["partitions"].as_array().map(|p| p.len()), Some(2), "Ventoy makes two partitions: {t}");
    let blk = sudo(&["blkid", "-o", "value", "-s", "TYPE", &format!("{dev}p1")]);
    assert!(blk.contains("exfat"), "the ISO partition is exFAT: {blk}");
    with_mounted(&dev, |m| {
        assert!(m.join("fake-linux-1.iso").is_file(), "fake-linux-1.iso on the drive");
        assert!(m.join("fake-uefi.iso").is_file(), "fake-uefi.iso on the drive");
        assert_eq!(std::fs::metadata(m.join("fake-uefi.iso")).unwrap().len(), std::fs::metadata(fx.join("uefi/fake-uefi.iso")).unwrap().len());
    });
}
