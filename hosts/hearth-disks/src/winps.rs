//! Windows: the Storage PowerShell module. Looking is done as you (no prompt); every change runs in an elevated
//! PowerShell (Start-Process -Verb RunAs), so Windows' permission prompt (UAC) comes up first. The elevated
//! scripts check again that they aren't touching the drive or partitions Windows runs from.

use crate::model::{Drive, Partition};
use crate::safety::Opts;
use crate::Progress;
use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

const NO_WINDOW: u32 = 0x0800_0000;

/// What Disks reads: disks, partitions, volumes, physical disks and (allowed for administrators only, so
/// often missing) their reliability counters.
const LIST: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
$disks = @(Get-Disk | ForEach-Object { [ordered]@{ Number = $_.Number; FriendlyName = $_.FriendlyName; Model = $_.Model; SerialNumber = $_.SerialNumber;
  Size = $_.Size; BusType = [string]$_.BusType; PartitionStyle = [string]$_.PartitionStyle; IsSystem = $_.IsSystem; IsBoot = $_.IsBoot;
  IsReadOnly = $_.IsReadOnly; IsOffline = $_.IsOffline; HealthStatus = [string]$_.HealthStatus } })
$parts = @(Get-Partition | ForEach-Object { [ordered]@{ DiskNumber = $_.DiskNumber; PartitionNumber = $_.PartitionNumber; Offset = $_.Offset; Size = $_.Size;
  Type = [string]$_.Type; GptType = $_.GptType; MbrType = $_.MbrType; DriveLetter = $(if ($_.DriveLetter -and $_.DriveLetter -ne [char]0) { [string]$_.DriveLetter } else { '' });
  IsSystem = $_.IsSystem; IsBoot = $_.IsBoot; IsActive = $_.IsActive; IsHidden = $_.IsHidden; AccessPaths = @($_.AccessPaths); Guid = $_.Guid } })
$vols = @(Get-Volume | ForEach-Object { [ordered]@{ Path = $_.Path; DriveLetter = $(if ($_.DriveLetter) { [string]$_.DriveLetter } else { '' });
  FileSystem = $_.FileSystem; FileSystemLabel = $_.FileSystemLabel; Size = $_.Size; SizeRemaining = $_.SizeRemaining; DriveType = [string]$_.DriveType } })
$phys = @(Get-PhysicalDisk | ForEach-Object { [ordered]@{ DeviceId = [string]$_.DeviceId; MediaType = [string]$_.MediaType; HealthStatus = [string]$_.HealthStatus; BusType = [string]$_.BusType } })
$rel = @(Get-PhysicalDisk | ForEach-Object { $id = [string]$_.DeviceId; $r = $_ | Get-StorageReliabilityCounter -ErrorAction SilentlyContinue
  if ($r) { [ordered]@{ DeviceId = $id; Temperature = $r.Temperature; Wear = $r.Wear; PowerOnHours = $r.PowerOnHours; ReadErrorsUncorrected = $r.ReadErrorsUncorrected } } })
[ordered]@{ disks = $disks; partitions = $parts; volumes = $vols; physical = $phys; reliability = $rel; systemDrive = $env:SystemDrive } | ConvertTo-Json -Depth 5 -Compress
"#;

fn b64(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for ch in bytes.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if ch.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if ch.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Runs a script in PowerShell as you, without a window; returns what it printed.
fn ps(script: &str) -> Result<String, String> {
    let full = format!("[Console]::OutputEncoding = [Text.Encoding]::UTF8\n{script}");
    let utf16: Vec<u8> = full.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let out = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64(&utf16)])
        .stdin(Stdio::null()).creation_flags(NO_WINDOW).output()
        .map_err(|e| format!("Couldn't start PowerShell: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).trim_start_matches('\u{feff}').to_string();
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        if err.to_lowercase().contains("canceled by the user") || err.to_lowercase().contains("cancelled by the user") {
            return Err("You said no to Windows' permission prompt, so nothing was changed.".into());
        }
        return Err(err.lines().find(|l| !l.trim().is_empty()).unwrap_or("PowerShell failed").trim().to_string());
    }
    Ok(text)
}

/// A PowerShell string literal. PowerShell also takes the curly quotes (‘ ’ ‚ ‛) as ', so those are doubled too:
/// a name like "Sam’s Games" ended the string early, and the rest ran as commands in the elevated script.
pub fn q(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}') { out.push(c); }
        out.push(c);
    }
    out.push('\'');
    out
}

fn steam_here(letter: &str) -> bool {
    let root = format!("{letter}:\\");
    let p = std::path::Path::new(&root);
    p.join("SteamLibrary\\steamapps").is_dir() || p.join("steamapps").is_dir() || p.join("libraryfolder.vdf").is_file()
}

pub fn list() -> Result<Vec<Drive>, String> {
    let out = ps(LIST)?;
    let v: Value = serde_json::from_str(out.trim()).map_err(|e| format!("Couldn't read Windows' list of drives: {e}"))?;
    Ok(crate::windows::parse(&v, &steam_here))
}

/// Runs `body` in an elevated PowerShell (Windows asks first) and returns the message it sets in $msg.
fn elevated(body: &str) -> Result<String, String> {
    let dir = std::env::temp_dir();
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let script = dir.join(format!("hearth-disks-{}-{n}.ps1", std::process::id()));
    let result = dir.join(format!("hearth-disks-{}-{n}.json", std::process::id()));
    let text = format!(
        "$ErrorActionPreference = 'Stop'\n$msg = ''\ntry {{\n{body}\n@{{ ok = $true; message = [string]$msg }} | ConvertTo-Json -Compress | Set-Content -Encoding UTF8 -LiteralPath {out}\n}} catch {{\n@{{ ok = $false; error = $_.Exception.Message }} | ConvertTo-Json -Compress | Set-Content -Encoding UTF8 -LiteralPath {out}\n}}\n",
        out = q(&result.to_string_lossy()));
    // A byte-order mark, so Windows PowerShell reads the script as UTF-8 (labels can have any letters).
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    std::fs::write(&script, bytes).map_err(|e| format!("Couldn't prepare the change: {e}"))?;
    let launch = format!(
        "$ErrorActionPreference = 'Stop'\n$p = Start-Process -FilePath 'powershell.exe' -Verb RunAs -WindowStyle Hidden -Wait -PassThru -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File',{})\nexit $p.ExitCode",
        q(&format!("\"{}\"", script.to_string_lossy())));
    let r = ps(&launch);
    let _ = std::fs::remove_file(&script);
    r?;
    let out = std::fs::read_to_string(&result).map_err(|_| "The change didn't report back. Look at the drive again to see what happened.".to_string())?;
    let _ = std::fs::remove_file(&result);
    let v: Value = serde_json::from_str(out.trim_start_matches('\u{feff}').trim()).map_err(|e| format!("The change reported back strangely: {e}"))?;
    if v["ok"] == true { Ok(v["message"].as_str().unwrap_or("").to_string()) } else { Err(plain(v["error"].as_str().unwrap_or("It didn't work."))) }
}

fn plain(e: &str) -> String {
    let low = e.to_lowercase();
    if low.contains("in use") || low.contains("access is denied") || low.contains("being used") {
        return "It's in use. Close apps using files on it (and Steam, if games run from it), then try again.".into();
    }
    format!("It didn't work: {}", e.trim())
}

/// (disk number, partition number) from "disk:N:part:M" or "disk:N".
pub fn ids(id: &str) -> (u64, u64) {
    let bits: Vec<&str> = id.split(':').collect();
    let n = bits.get(1).and_then(|x| x.parse().ok()).unwrap_or(u64::MAX);
    let m = bits.get(3).and_then(|x| x.parse().ok()).unwrap_or(0);
    (n, m)
}

fn fs_name(fs: &str) -> &'static str { match fs { "ntfs" => "NTFS", "fat32" => "FAT32", _ => "exFAT" } }

/// Lines the elevated scripts start with: the partition, and a refusal if it's one Windows runs from.
fn guard_part(n: u64, m: u64) -> String {
    format!("$part = Get-Partition -DiskNumber {n} -PartitionNumber {m}\nif ($part.IsBoot -or $part.IsSystem -or ($part.DriveLetter -and ([string]$part.DriveLetter + ':') -eq $env:SystemDrive)) {{ throw 'Windows is running from this partition, so it was left alone.' }}\n")
}
fn guard_disk(n: u64) -> String {
    format!("$disk = Get-Disk -Number {n}\nif ($disk.IsSystem -or $disk.IsBoot) {{ throw 'Windows is running from this drive, so it was left alone.' }}\n")
}
fn format_line(target: &str, fs: &str, label: &str) -> String {
    format!("{target} | Format-Volume -FileSystem {} -NewFileSystemLabel {} -Confirm:$false -Force | Out-Null\n", fs_name(fs), q(label))
}

/// Steam's folder from the registry (as you, not the administrator).
fn steam_dir() -> Option<String> {
    ps("(Get-ItemProperty -Path 'HKCU:\\Software\\Valve\\Steam' -Name SteamPath -ErrorAction SilentlyContinue).SteamPath").ok()
        .map(|s| s.trim().replace('/', "\\")).filter(|s| !s.is_empty())
}

/// Adds <letter>:\SteamLibrary to Steam, as you: Steam closes for a moment if it's open, libraryfolders.vdf
/// gets the new entry (the same one hearth-core's steam::with_library writes on Linux), and Steam opens again.
fn add_to_steam(letter: &str) -> Result<String, String> {
    let steam = steam_dir().ok_or("Steam isn't installed or hasn't been opened yet. Open Steam once, then try again.")?;
    let folder = format!("{letter}:\\SteamLibrary");
    let script = format!(r#"
$ErrorActionPreference = 'Stop'
$steam = {steam}; $folder = {folder}
New-Item -ItemType Directory -Force -Path (Join-Path $folder 'steamapps') | Out-Null
$id = [string](Get-Random -Minimum 100000000 -Maximum 2147483647)
$marker = Join-Path $folder 'libraryfolder.vdf'
if (-not (Test-Path $marker)) {{ Set-Content -LiteralPath $marker -Encoding ASCII -Value ("""libraryfolder""`n{{`n`t""contentid""`t`t""$id""`n`t""label""`t`t""""`n}}") }}
$vdf = Join-Path $steam 'steamapps\libraryfolders.vdf'
$text = if (Test-Path $vdf) {{ Get-Content -LiteralPath $vdf -Raw }} else {{ '' }}
$want = $folder.TrimEnd('\')
$paths = [regex]::Matches($text, '"path"\s+"([^"]*)"') | ForEach-Object {{ $_.Groups[1].Value.Replace('\\', '\').TrimEnd('\') }}
if ($paths -contains $want) {{ 'already'; exit 0 }}
$keys = [regex]::Matches($text, '(?m)^\s*"(\d+)"\s*$') | ForEach-Object {{ [int]$_.Groups[1].Value }}
$next = if ($keys) {{ ($keys | Measure-Object -Maximum).Maximum + 1 }} else {{ 1 }}
$esc = $want.Replace('\', '\\')
$entry = "`t""$next""`n`t{{`n`t`t""path""`t`t""$esc""`n`t`t""label""`t`t""""`n`t`t""contentid""`t`t""$id""`n`t`t""totalsize""`t`t""0""`n`t`t""update_clean_bytes_tally""`t`t""0""`n`t`t""time_last_update_verified""`t`t""0""`n`t`t""apps""`n`t`t{{`n`t`t}}`n`t}}`n"
if ($text -notmatch '\}}') {{ $s0 = $steam.Replace('\', '\\'); $text = """libraryfolders""`n{{`n`t""0""`n`t{{`n`t`t""path""`t`t""$s0""`n`t}}`n}}`n" }}
$running = @(Get-Process steam -ErrorAction SilentlyContinue).Count -gt 0
if ($running) {{
  Start-Process -FilePath (Join-Path $steam 'steam.exe') -ArgumentList '-shutdown'
  for ($i = 0; $i -lt 60 -and @(Get-Process steam -ErrorAction SilentlyContinue).Count -gt 0; $i++) {{ Start-Sleep -Milliseconds 500 }}
  if (@(Get-Process steam -ErrorAction SilentlyContinue).Count -gt 0) {{ throw 'Steam did not close. Close Steam, then try again.' }}
}}
$end = $text.LastIndexOf('}}')
$text = $text.Substring(0, $end).TrimEnd("`t", ' ') + $entry + $text.Substring($end)
Set-Content -LiteralPath $vdf -Value $text -Encoding UTF8 -NoNewline
if ($running) {{ Start-Process -FilePath (Join-Path $steam 'steam.exe') -ArgumentList '-silent'; 'restarted' }} else {{ 'added' }}
"#, steam = q(&steam), folder = q(&folder));
    let out = ps(&script)?;
    Ok(if out.contains("restarted") { "Ready for games. Steam was restarted and lists it under Storage.".into() }
       else { "Ready for games. Steam lists it under Storage; pick it when you install a game.".into() })
}

pub fn execute(action: &str, d: &Drive, p: Option<&Partition>, o: &Opts, progress: Progress) -> Result<String, String> {
    let step = |t: &str| progress(t, None);
    let (n, _) = ids(&d.id);
    let label = o.label.as_str();
    match (action, p) {
        ("open", Some(p)) => {
            Command::new("explorer.exe").arg(format!("{}:\\", p.letter)).spawn().map(|_| String::new()).map_err(|e| format!("Couldn't open it: {e}"))
        }
        ("mount", Some(p)) => {
            let (n, m) = ids(&p.id);
            step("Giving it a drive letter");
            elevated(&format!("{}Add-PartitionAccessPath -DiskNumber {n} -PartitionNumber {m} -AssignDriveLetter\n$msg = 'It has a drive letter now.'", guard_part(n, m)))
        }
        ("unmount", Some(p)) => {
            let (n, m) = ids(&p.id);
            step("Removing its drive letter");
            elevated(&format!("{}Remove-PartitionAccessPath -DiskNumber {n} -PartitionNumber {m} -AccessPath ([string]$part.DriveLetter + ':\\')\n$msg = 'Its drive letter was removed.'", guard_part(n, m)))
        }
        ("eject", None) => {
            step("Letting go of it");
            let letters: Vec<String> = d.partitions.iter().filter(|p| !p.letter.is_empty()).map(|p| format!("{}:", p.letter)).collect();
            if letters.is_empty() { return Ok(format!("You can remove {} now.", d.name())); }
            let script = format!("$sh = New-Object -ComObject Shell.Application\nforeach ($l in @({})) {{ $i = $sh.Namespace(17).ParseName($l); if ($i) {{ $i.InvokeVerb('Eject') }} }}\nStart-Sleep -Seconds 2\n@({}) | Where-Object {{ Test-Path ($_ + '\\') }}",
                letters.iter().map(|l| q(l)).collect::<Vec<_>>().join(","), letters.iter().map(|l| q(l)).collect::<Vec<_>>().join(","));
            let still = ps(&script)?;
            if still.trim().is_empty() { Ok(format!("You can remove {} now.", d.name())) }
            else { Err("Windows couldn't let go of it yet. Close apps using files on it, then try again.".into()) }
        }
        ("rename", Some(p)) => {
            let (n, m) = ids(&p.id);
            step("Renaming");
            elevated(&format!("{}$part | Get-Volume | Set-Volume -NewFileSystemLabel {}\n$msg = 'Renamed.'", guard_part(n, m), q(label)))
        }
        ("check", Some(p)) | ("repair", Some(p)) => {
            let (n, m) = ids(&p.id);
            let repair = action == "repair";
            step(if repair { "Repairing" } else { "Checking" });
            let r = elevated(&format!("{}$r = $part | Get-Volume | Repair-Volume {}\n$msg = [string]$r", guard_part(n, m), if repair { "-OfflineScanAndFix" } else { "-Scan" }))?;
            Ok(match (r.as_str(), repair) {
                ("NoErrorsFound", _) => "No problems found.".into(),
                (_, false) => "It has problems. Choose Repair to fix them.".into(),
                (_, true) => "Repaired.".into(),
            })
        }
        ("format", Some(p)) => {
            let (n, m) = ids(&p.id);
            step("Formatting");
            elevated(&format!("{}{}$msg = 'Formatted.'", guard_part(n, m), format_line("$part", &o.fs, label)))
        }
        ("delete", Some(p)) => {
            let (n, m) = ids(&p.id);
            step("Deleting the partition");
            elevated(&format!("{}Remove-Partition -DiskNumber {n} -PartitionNumber {m} -Confirm:$false\n$msg = 'Deleted.'", guard_part(n, m)))
        }
        ("create", Some(p)) => {
            step("Making the partition");
            let size = if o.size == 0 { p.size } else { o.size };
            elevated(&format!("$new = New-Partition -DiskNumber {n} -Offset {} -Size {size} -AssignDriveLetter\n{}$msg = 'Partition made.'", p.offset, format_line("$new", &o.fs, label)))
        }
        ("format_drive", None) => {
            step("Erasing the drive");
            elevated(&format_drive_script(n, d.size, &o.fs, label)).map(|_| format!("{} is formatted.", d.name()))
        }
        ("games", target) => {
            let label = if label.is_empty() { "Games" } else { label };
            let letter_of_new = "$msg = [string]($new | Get-Volume).DriveLetter";
            let letter = match target {
                None => { step("Erasing the drive"); elevated(&format!("{}{letter_of_new}", format_drive_script(n, d.size, &o.fs, label)))? }
                Some(p) if p.kind == "free" => {
                    step("Making the partition");
                    let size = if o.size == 0 { p.size } else { o.size };
                    elevated(&format!("$new = New-Partition -DiskNumber {n} -Offset {} -Size {size} -AssignDriveLetter\n{}{letter_of_new}", p.offset, format_line("$new", &o.fs, label)))?
                }
                Some(p) => {
                    let (n, m) = ids(&p.id);
                    let fmt = if crate::safety::games_ready("windows", &p.fs) { String::new() } else { format_line("$part", &o.fs, label) };
                    if !fmt.is_empty() { step("Formatting"); }
                    if p.letter.is_empty() || !fmt.is_empty() {
                        elevated(&format!("{}{fmt}if (-not $part.DriveLetter -or $part.DriveLetter -eq [char]0) {{ Add-PartitionAccessPath -DiskNumber {n} -PartitionNumber {m} -AssignDriveLetter }}\n$part = Get-Partition -DiskNumber {n} -PartitionNumber {m}\n$msg = [string]$part.DriveLetter", guard_part(n, m)))?
                    } else { p.letter.clone() }
                }
            };
            let letter = letter.trim().chars().next().filter(|c| c.is_ascii_alphabetic()).ok_or("It didn't get a drive letter, so Steam can't use it.")?;
            step("Adding it to Steam");
            add_to_steam(&letter.to_string())
        }
        _ => Err("That can't be done here.".into()),
    }
}

/// Erase a whole drive and make one partition: MBR for exFAT and FAT32 (cameras, consoles), GPT for NTFS and
/// drives over 2 TB. Leaves the new partition in $new.
fn format_drive_script(n: u64, size: u64, fs: &str, label: &str) -> String {
    let style = if fs != "ntfs" && size <= 2 * 1024 * 1024 * 1024 * 1024 { "MBR" } else { "GPT" };
    format!("{}if ($disk.IsOffline) {{ Set-Disk -Number {n} -IsOffline $false }}\nif ($disk.IsReadOnly) {{ Set-Disk -Number {n} -IsReadOnly $false }}\nif ([string]$disk.PartitionStyle -ne 'RAW') {{ Clear-Disk -Number {n} -RemoveData -RemoveOEM -Confirm:$false }}\nInitialize-Disk -Number {n} -PartitionStyle {style}\n$new = New-Partition -DiskNumber {n} -UseMaximumSize -AssignDriveLetter\n{}",
        guard_disk(n), format_line("$new", fs, label))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoting_and_ids() {
        assert_eq!(q("it's"), "'it''s'");
        assert_eq!(q("Sam\u{2019}s"), "'Sam\u{2019}\u{2019}s'");
        assert_eq!(q("\u{2018}x\u{201B}"), "'\u{2018}\u{2018}x\u{201B}\u{201B}'");
        assert_eq!(ids("disk:3:part:2"), (3, 2));
        assert_eq!(ids("disk:1"), (1, 0));
        assert_eq!(b64(b"hello"), "aGVsbG8=");
        assert_eq!(b64(b"hi"), "aGk=");
        assert!(format_drive_script(1, 64 << 30, "exfat", "A'B").contains("-NewFileSystemLabel 'A''B'"));
        assert!(format_drive_script(1, 64 << 30, "exfat", "x").contains("PartitionStyle MBR"));
        assert!(format_drive_script(1, 64 << 30, "ntfs", "x").contains("PartitionStyle GPT"));
        assert!(guard_part(0, 3).contains("throw"));
    }
    #[test]
    fn powershell_lists_drives() {
        // Runs the real listing on the Windows runner: it must come back as JSON Disks can read.
        let d = list().expect("listing");
        assert!(!d.is_empty(), "the runner has at least one disk");
        assert!(d.iter().any(|d| d.system), "one of them is Windows' drive");
        eprintln!("{}", serde_json::to_string_pretty(&d).unwrap());
    }
}
