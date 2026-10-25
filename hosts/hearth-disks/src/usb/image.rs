//! What's in an image, and how it has to be written:
//!  - raw: a hybrid ISO (an ISO that's also a disk: almost every Linux ISO) or a disk image (.img, also .img.xz,
//!    .img.zst, .img.gz, unpacked while it's written): copied to the drive as it is.
//!  - windows: a Windows ISO (sources/install.wim or install.esd, bootmgr): a FAT32 partition with its files
//!    copied (install.wim split into .swm parts when it's over FAT32's 4 GB limit).
//!  - uefi-copy: another ISO that isn't hybrid but has EFI start-up files: copied like Windows, with a warning
//!    (it starts only on UEFI PCs, and some systems look for their own disc label).
//!  - Anything else is refused.
//! The first sectors tell hybrid from not; an ISO's files are looked at through a read-only loop mount (udisks,
//! as you). Windows' edition, version and architecture come from wimlib's `wiminfo`.

use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Inspect {
    /// raw, windows, uefi-copy or unknown
    pub mode: String,
    /// iso, img, img.xz, img.zst, img.gz
    pub format: String,
    pub hybrid: bool,
    pub size: u64,
    /// The ISO's volume label (ISO 9660).
    pub label: String,
    pub windows: Option<WinInfo>,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WinInfo {
    /// "Windows 11", "Windows 10", "Windows Server 2025", "Windows"
    pub product: String,
    pub arch: String,
    pub build: String,
    pub editions: Vec<String>,
    /// install.wim or install.esd, and its size
    pub install: String,
    pub install_size: u64,
    pub needs_split: bool,
    pub server: bool,
}

pub fn format_of(path: &Path) -> String {
    let n = path.file_name().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    for (ext, f) in [(".img.xz", "img.xz"), (".img.zst", "img.zst"), (".img.zstd", "img.zst"), (".img.gz", "img.gz"), (".raw.xz", "img.xz"), (".iso.xz", "img.xz"), (".img", "img"), (".raw", "img"), (".iso", "iso")] {
        if n.ends_with(ext) { return f.into(); }
    }
    "unknown".into()
}

/// A command that unpacks a compressed image to stdout, for formats that need it.
pub fn unpacker(format: &str) -> Option<(&'static str, Vec<&'static str>)> {
    match format {
        "img.xz" => Some(("xz", vec!["-dc", "-T0"])),
        "img.zst" => Some(("zstd", vec!["-dc", "-q"])),
        "img.gz" => Some(("gzip", vec!["-dc"])),
        _ => None,
    }
}

/// From the first 64 KiB of an image: (it has an MBR or GPT partition table, it's an ISO 9660, its label).
pub fn sniff(head: &[u8]) -> (bool, bool, String) {
    let mbr_sig = head.len() >= 512 && head[510] == 0x55 && head[511] == 0xAA;
    let gpt = head.len() >= 520 && &head[512..520] == b"EFI PART";
    // An MBR with at least one partition entry (type byte != 0), or a GPT.
    let mut parts = false;
    if mbr_sig { for i in 0..4 { if head[446 + i * 16 + 4] != 0 { parts = true; } } }
    let table = gpt || (mbr_sig && parts);
    let iso = head.len() >= 32774 && &head[32769..32774] == b"CD001";
    let label = if head.len() >= 32808 && iso { String::from_utf8_lossy(&head[32808..32840]).trim_matches(|c: char| c == '\0' || c.is_whitespace()).to_string() } else { String::new() };
    (table, iso, label)
}

pub fn read_head(path: &Path, format: &str) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; 65536];
    let mut n = 0;
    if let Some((cmd, args)) = unpacker(format) {
        let mut child = std::process::Command::new(cmd).args(&args).arg(path).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null())
            .spawn().map_err(|e| format!("{cmd} isn't installed ({e}), so this image can't be unpacked"))?;
        let mut out = child.stdout.take().unwrap();
        while n < buf.len() { match out.read(&mut buf[n..]) { Ok(0) => break, Ok(k) => n += k, Err(_) => break } }
        let _ = child.kill();
        let _ = child.wait();
    } else {
        let mut f = std::fs::File::open(path).map_err(|e| format!("Couldn't read the image: {e}"))?;
        while n < buf.len() { match f.read(&mut buf[n..]) { Ok(0) => break, Ok(k) => n += k, Err(e) => return Err(e.to_string()) } }
    }
    buf.truncate(n);
    Ok(buf)
}

/// Case-insensitive path inside a mounted ISO (Windows ISOs mix cases).
pub fn find_ci(root: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = root.to_path_buf();
    for part in rel.split('/') {
        let entry = std::fs::read_dir(&cur).ok()?.flatten().find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        cur = entry.path();
    }
    Some(cur)
}

/// FAT32's largest file: 4 GiB - 1. CI makes it smaller (HEARTH_USB_SPLIT_MB) to test splitting with a small WIM.
pub fn split_limit() -> u64 {
    std::env::var("HEARTH_USB_SPLIT_MB").ok().and_then(|v| v.parse::<u64>().ok()).filter(|v| *v > 0).map(|mb| mb * 1024 * 1024).unwrap_or(4 * 1024 * 1024 * 1024 - 1)
}

/// Looks at a mounted ISO's files: Windows (with wiminfo), EFI start-up files, or neither.
pub fn look_inside(root: &Path) -> (Option<WinInfo>, bool) {
    let efi = find_ci(root, "efi/boot/bootx64.efi").is_some() || find_ci(root, "efi/boot/bootaa64.efi").is_some();
    let install = find_ci(root, "sources/install.wim").or_else(|| find_ci(root, "sources/install.esd"));
    let Some(install) = install else { return (None, efi) };
    let has_boot = find_ci(root, "bootmgr").is_some() || find_ci(root, "efi/boot/bootx64.efi").is_some() || find_ci(root, "efi/boot/bootaa64.efi").is_some();
    if !has_boot { return (None, efi); }
    let size = std::fs::metadata(&install).map(|m| m.len()).unwrap_or(0);
    let text = std::process::Command::new("wiminfo").arg(&install).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let mut w = parse_wiminfo(&text);
    w.install = install.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    w.install_size = size;
    w.needs_split = size > split_limit() && w.install.to_lowercase().ends_with(".wim");
    if w.arch.is_empty() { w.arch = if find_ci(root, "efi/boot/bootaa64.efi").is_some() { "ARM64".into() } else { "x86_64".into() }; }
    (Some(w), efi)
}

/// Windows facts from `wiminfo`'s text: each image's Name, Architecture, Build and Installation Type.
pub fn parse_wiminfo(text: &str) -> WinInfo {
    let mut w = WinInfo::default();
    let mut builds: Vec<u32> = vec![];
    for line in text.lines() {
        let Some((k, v)) = line.split_once(':') else { continue };
        let (k, v) = (k.trim(), v.trim());
        match k {
            "Name" if !v.is_empty() => { if !w.editions.iter().any(|e| e == v) { w.editions.push(v.to_string()); } }
            "Architecture" if !v.is_empty() && w.arch.is_empty() => w.arch = match v { "x86_64" | "AMD64" | "amd64" => "x86_64".into(), "ARM64" | "arm64" | "aarch64" => "ARM64".into(), "x86" | "i386" => "x86".into(), o => o.into() },
            "Build" => if let Ok(b) = v.parse::<u32>() { builds.push(b) },
            "Installation Type" => if v.eq_ignore_ascii_case("server") || v.to_lowercase().starts_with("server") { w.server = true },
            _ => {}
        }
    }
    let build = builds.iter().max().copied();
    w.build = build.map(|b| b.to_string()).unwrap_or_default();
    let named = |s: &str| w.editions.iter().any(|e| e.contains(s));
    w.product = if w.server || named("Server") {
        w.server = true;
        match build { Some(b) if b >= 26100 => "Windows Server 2025".into(), Some(b) if b >= 20348 => "Windows Server 2022".into(), Some(b) if b >= 17763 => "Windows Server 2019".into(), _ => "Windows Server".into() }
    } else if named("Windows 11") || build.map(|b| b >= 22000).unwrap_or(false) { "Windows 11".into() }
    else if named("Windows 10") || build.map(|b| b >= 10240).unwrap_or(false) { "Windows 10".into() }
    else { "Windows".into() };
    w
}

/// Windows 11's optional changes, made with autounattend.xml at the root of the USB drive.
#[derive(serde::Deserialize, Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Tweaks {
    /// Install on PCs without TPM 2.0, Secure Boot or 4 GB of memory (Setup's LabConfig checks).
    pub skip_checks: bool,
    /// No Microsoft account needed while setting up (BypassNRO, online account screens hidden).
    pub skip_online: bool,
    /// Make this local account (no password: Windows asks for one at the first sign-in).
    pub local_account: String,
}

impl Tweaks {
    pub fn any(&self) -> bool { self.skip_checks || self.skip_online || !self.local_account.trim().is_empty() }
}

/// A local account name Windows accepts: 1 to 20 characters, none of " / \ [ ] : ; | = , + * ? < > @, not only
/// dots or spaces, not a built-in name.
pub fn check_account(name: &str) -> Result<(), String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 20 { return Err("The account name must be 1 to 20 characters.".into()); }
    if n.chars().any(|c| c.is_control() || "\"/\\[]:;|=,+*?<>@%".contains(c)) { return Err("Account names can't have \" / \\ [ ] : ; | = , + * ? < > @ %".into()); }
    if n.chars().all(|c| c == '.' || c == ' ') { return Err("That isn't a name Windows accepts.".into()); }
    let lower = n.to_lowercase();
    if ["administrator", "guest", "defaultaccount", "wdagutilityaccount", "system", "none", "user", "users", "admin"].contains(&lower.as_str()) { return Err("That name is taken by Windows itself. Pick another.".into()); }
    Ok(())
}

fn xml_escape(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;") }

/// autounattend.xml for the chosen tweaks (Windows Setup reads it from the USB drive's root). `arch`: x86_64 or ARM64.
pub fn autounattend(t: &Tweaks, arch: &str) -> Result<String, String> {
    let a = if arch == "ARM64" { "arm64" } else { "amd64" };
    let comp = |name: &str, body: &str| format!("    <component name=\"{name}\" processorArchitecture=\"{a}\" publicKeyToken=\"31bf3856ad364e35\" language=\"neutral\" versionScope=\"nonSxS\" xmlns:wcm=\"http://schemas.microsoft.com/WMIConfig/2002/State\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">\n{body}    </component>\n");
    let cmd = |order: usize, path: &str| format!("        <RunSynchronousCommand wcm:action=\"add\">\n          <Order>{order}</Order>\n          <Path>{}</Path>\n        </RunSynchronousCommand>\n", xml_escape(path));
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!-- Made by Hearth Disks (Make a bootable USB). Delete this file to install Windows the usual way. -->\n<unattend xmlns=\"urn:schemas-microsoft-com:unattend\">\n");
    if t.skip_checks {
        let mut cmds = String::new();
        for (i, v) in ["BypassTPMCheck", "BypassSecureBootCheck", "BypassRAMCheck"].iter().enumerate() {
            cmds += &cmd(i + 1, &format!("reg add HKLM\\SYSTEM\\Setup\\LabConfig /v {v} /t REG_DWORD /d 1 /f"));
        }
        out += "  <settings pass=\"windowsPE\">\n";
        out += &comp("Microsoft-Windows-Setup", &format!("      <UserData>\n        <ProductKey>\n          <WillShowUI>OnError</WillShowUI>\n        </ProductKey>\n      </UserData>\n      <RunSynchronous>\n{cmds}      </RunSynchronous>\n"));
        out += "  </settings>\n";
    }
    if t.skip_online {
        out += "  <settings pass=\"specialize\">\n";
        out += &comp("Microsoft-Windows-Deployment", &format!("      <RunSynchronous>\n{}      </RunSynchronous>\n", cmd(1, "reg add HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\OOBE /v BypassNRO /t REG_DWORD /d 1 /f")));
        out += "  </settings>\n";
    }
    let account = t.local_account.trim();
    if t.skip_online || !account.is_empty() {
        let mut body = String::from("      <OOBE>\n");
        if t.skip_online { body += "        <HideOnlineAccountScreens>true</HideOnlineAccountScreens>\n"; }
        body += "        <ProtectYourPC>3</ProtectYourPC>\n      </OOBE>\n";
        if !account.is_empty() {
            check_account(account)?;
            let n = xml_escape(account);
            body += &format!("      <UserAccounts>\n        <LocalAccounts>\n          <LocalAccount wcm:action=\"add\">\n            <Name>{n}</Name>\n            <DisplayName>{n}</DisplayName>\n            <Group>Administrators</Group>\n            <Password>\n              <Value></Value>\n              <PlainText>true</PlainText>\n            </Password>\n          </LocalAccount>\n        </LocalAccounts>\n      </UserAccounts>\n");
            body += &format!("      <FirstLogonCommands>\n        <SynchronousCommand wcm:action=\"add\">\n          <Order>1</Order>\n          <CommandLine>net user &quot;{n}&quot; /logonpasswordchg:yes</CommandLine>\n          <Description>Ask for a password at the first sign-in</Description>\n        </SynchronousCommand>\n        <SynchronousCommand wcm:action=\"add\">\n          <Order>2</Order>\n          <CommandLine>net accounts /maxpwage:unlimited</CommandLine>\n        </SynchronousCommand>\n      </FirstLogonCommands>\n");
        }
        out += "  <settings pass=\"oobeSystem\">\n";
        out += &comp("Microsoft-Windows-Shell-Setup", &body);
        out += "  </settings>\n";
    }
    out += "</unattend>\n";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(format_of(Path::new("/x/Fedora.iso")), "iso");
        assert_eq!(format_of(Path::new("/x/raspios.img.xz")), "img.xz");
        assert_eq!(format_of(Path::new("/x/a.IMG.ZST")), "img.zst");
        assert_eq!(format_of(Path::new("/x/a.img")), "img");
        assert_eq!(format_of(Path::new("/x/a.zip")), "unknown");
        assert_eq!(unpacker("img.xz").unwrap().0, "xz");
        assert!(unpacker("iso").is_none());
    }

    #[test]
    fn hybrid_or_not() {
        let mut h = vec![0u8; 65536];
        assert_eq!(sniff(&h), (false, false, String::new()));
        // ISO 9660 descriptor with a label, no partition table: a Windows-style ISO.
        h[32769..32774].copy_from_slice(b"CD001");
        h[32808..32808 + 23].copy_from_slice(b"CCCOMA_X64FRE_EN-US_DV9");
        let (table, iso, label) = sniff(&h);
        assert!(!table && iso);
        assert_eq!(label, "CCCOMA_X64FRE_EN-US_DV9");
        // An isohybrid MBR with one partition.
        h[510] = 0x55; h[511] = 0xAA; h[446 + 4] = 0x17;
        assert!(sniff(&h).0);
        // A boot signature with no partitions isn't a disk.
        h[446 + 4] = 0;
        assert!(!sniff(&h).0);
        // GPT.
        h[512..520].copy_from_slice(b"EFI PART");
        assert!(sniff(&h).0);
    }

    #[test]
    fn reads_wiminfo() {
        let t = "WIM Information:\n----------------\nPath:           install.wim\nImage Count:    2\n\nAvailable Images:\n-----------------\nIndex:                  1\nName:                   Windows 11 Home\nArchitecture:           x86_64\nInstallation Type:      Client\nMajor Version:          10\nBuild:                  26100\n\nIndex:                  2\nName:                   Windows 11 Pro\nArchitecture:           x86_64\nBuild:                  26100\n";
        let w = parse_wiminfo(t);
        assert_eq!(w.product, "Windows 11");
        assert_eq!(w.editions, vec!["Windows 11 Home", "Windows 11 Pro"]);
        assert_eq!((w.arch.as_str(), w.build.as_str(), w.server), ("x86_64", "26100", false));
        let w = parse_wiminfo("Name: Windows 10 Pro\nArchitecture: x86_64\nBuild: 19045\n");
        assert_eq!(w.product, "Windows 10");
        let w = parse_wiminfo("Name: Windows Server 2025 Standard\nInstallation Type: Server\nBuild: 26100\nArchitecture: x86_64\n");
        assert_eq!((w.product.as_str(), w.server), ("Windows Server 2025", true));
        let w = parse_wiminfo("Name: Windows 11 Pro\nArchitecture: ARM64\nBuild: 26100\n");
        assert_eq!(w.arch, "ARM64");
        // Images made without Windows' own facts (a test WIM): the name still tells.
        assert_eq!(parse_wiminfo("Name: Windows 11 Pro\n").product, "Windows 11");
    }

    #[test]
    fn unattend() {
        assert!(!Tweaks::default().any());
        let x = autounattend(&Tweaks { skip_checks: true, ..Default::default() }, "x86_64").unwrap();
        assert!(x.contains("BypassTPMCheck") && x.contains("BypassSecureBootCheck") && x.contains("BypassRAMCheck"));
        assert!(x.contains("pass=\"windowsPE\"") && x.contains("processorArchitecture=\"amd64\""));
        assert!(!x.contains("BypassNRO") && !x.contains("LocalAccount"));
        let x = autounattend(&Tweaks { skip_online: true, local_account: "Sam".into(), ..Default::default() }, "ARM64").unwrap();
        assert!(x.contains("BypassNRO") && x.contains("<HideOnlineAccountScreens>true") && x.contains("<Name>Sam</Name>") && x.contains("logonpasswordchg"));
        assert!(x.contains("processorArchitecture=\"arm64\""));
        assert!(!x.contains("LabConfig"));
        // Names are checked and escaped.
        assert!(autounattend(&Tweaks { local_account: "a<b".into(), ..Default::default() }, "x86_64").is_err());
        assert!(check_account("Administrator").is_err());
        assert!(check_account("").is_err());
        assert!(check_account("abcdefghijklmnopqrstu").is_err());
        assert!(check_account("Sam & Ana").is_ok());
        assert!(autounattend(&Tweaks { local_account: "Sam & Ana".into(), ..Default::default() }, "x86_64").unwrap().contains("Sam &amp; Ana"));
        // Well-formed: every element closed.
        for tag in ["unattend", "settings", "component", "OOBE", "UserAccounts", "LocalAccount", "FirstLogonCommands"] {
            let x = autounattend(&Tweaks { skip_checks: true, skip_online: true, local_account: "Sam".into() }, "x86_64").unwrap();
            assert_eq!(x.matches(&format!("<{tag}>")).count() + x.matches(&format!("<{tag} ")).count(), x.matches(&format!("</{tag}>")).count(), "{tag}");
        }
    }

    #[test]
    fn split_limit_is_fat32s() {
        std::env::remove_var("HEARTH_USB_SPLIT_MB");
        assert_eq!(split_limit(), 4294967295);
    }
}
