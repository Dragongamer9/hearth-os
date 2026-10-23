//! Printers & Scanner. Printers: the same CUPS commands as Hearth Settings › Printers (SetPrinters.qml):
//! the printer list from hearth-system, adding, removing and the default through hearth-admin; and here the
//! print queue (lpstat), cancelling and holding your jobs, pausing a printer (hearth-admin printer-pause /
//! printer-resume / printer-cancel-job). Scanner: SANE's scanimage (sane-airscan finds network scanners):
//! a quick preview, an area, resolution and colour, then PNG files or one PDF of several pages (written here).

use crate::util;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

fn shell_helper(name: &str) -> Option<String> {
    [format!("/usr/share/nimbus/desktop/shell/{name}"), format!("/usr/local/share/nimbus/desktop/shell/{name}")].into_iter().find(|p| Path::new(p).exists())
}

/// A printer's name as CUPS allows (the same rule as hearth-admin's PRINTER_NAME_RE).
pub fn valid_printer(n: &str) -> bool {
    !n.is_empty() && n.len() <= 127 && n.chars().next().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false) && n.chars().all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
}
/// A job id: printer-number.
pub fn valid_job(j: &str) -> bool {
    match j.rsplit_once('-') { Some((p, n)) => valid_printer(p) && !n.is_empty() && n.len() < 10 && n.chars().all(|c| c.is_ascii_digit()), None => false }
}

/// `lpstat -o` lines: "HP-12 sam 1024 Wed 22 Oct 2026 10:00:00 AM CEST" -> {id, printer, user, size, when}.
pub fn parse_jobs(text: &str) -> Vec<Value> {
    text.lines().filter_map(|l| {
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() < 3 || !valid_job(f[0]) { return None; }
        let printer = f[0].rsplit_once('-').map(|x| x.0).unwrap_or("");
        Some(json!({"id": f[0], "printer": printer, "user": f[1], "size": f[2].parse::<u64>().ok(), "when": f[3..].join(" ")}))
    }).collect()
}

pub fn state() -> Result<Value, String> {
    let printers: Value = match shell_helper("hearth-system") {
        Some(h) => util::run(&h, &["printers"], 20).ok().and_then(|o| o.out.lines().rev().find_map(|l| serde_json::from_str::<Value>(l).ok())).unwrap_or(json!({"cups": false, "printers": []})),
        None => json!({"cups": util::which("lpstat").is_some(), "printers": []}),
    };
    let jobs = util::run("lpstat", &["-W", "not-completed", "-o"], 15).map(|o| parse_jobs(&o.out)).unwrap_or_default();
    // Held jobs say so in the long listing.
    let held: Vec<String> = util::run("lpstat", &["-W", "not-completed", "-l", "-o"], 15).map(|o| {
        let mut out = vec![]; let mut cur = String::new();
        for l in o.out.lines() {
            if !l.starts_with(char::is_whitespace) { cur = l.split_whitespace().next().unwrap_or("").to_string(); }
            else if l.contains("job-hold-until") || l.contains("held") { out.push(cur.clone()); }
        }
        out
    }).unwrap_or_default();
    let me = std::env::var("USER").unwrap_or_default();
    let jobs: Vec<Value> = jobs.into_iter().map(|mut j| { let id = j["id"].as_str().unwrap_or("").to_string(); j["held"] = json!(held.contains(&id)); j["mine"] = json!(j["user"].as_str() == Some(me.as_str())); j }).collect();
    Ok(json!({"cups": printers["cups"], "printers": printers["printers"], "jobs": jobs}))
}

pub fn job_action(args: &Value) -> Result<Value, String> {
    let job = util::arg(args, "job");
    if !valid_job(job) { return Err("Unknown print job.".into()); }
    match util::arg(args, "action") {
        "cancel" => {
            let o = util::run("cancel", &[job], 20)?;
            if o.ok() { return Ok(json!({"ok": true})); }
            // Someone else's job: through hearth-admin (CUPS lets root cancel any job).
            util::admin(&["printer-cancel-job", job], None, 60)
        }
        "hold" | "release" => {
            let o = util::run("lp", &["-i", job, "-H", if util::arg(args, "action") == "hold" { "hold" } else { "resume" }], 20)?;
            if o.ok() { Ok(json!({"ok": true})) } else { Err(format!("CUPS said no ({})", o.why())) }
        }
        _ => Err("Unknown action.".into()),
    }
}

pub fn printer_action(args: &Value) -> Result<Value, String> {
    let name = util::arg(args, "printer");
    if !valid_printer(name) { return Err("Unknown printer.".into()); }
    match util::arg(args, "action") {
        "pause" => util::admin(&["printer-pause", name], None, 60),
        "resume" => util::admin(&["printer-resume", name], None, 60),
        "default" => util::admin(&["printer-default", name], None, 60),
        "remove" => util::admin(&["printer-remove", name], None, 60),
        "test" => {
            let h = shell_helper("hearth-system").ok_or("Hearth's settings helper isn't installed.")?;
            let o = util::run(&h, &["printer-test", name], 60)?;
            if o.ok() { Ok(json!({"ok": true})) } else { Err(o.out.lines().chain(o.err.lines()).find_map(|l| l.strip_prefix("error: ")).map(str::to_string).unwrap_or_else(|| o.why())) }
        }
        _ => Err("Unknown action.".into()),
    }
}

pub fn discover() -> Result<Value, String> { util::admin(&["printer-discover"], None, 120) }

pub fn add(args: &Value) -> Result<Value, String> {
    let (name, uri, info) = (util::arg(args, "name"), util::arg(args, "uri"), util::arg(args, "info"));
    if !valid_printer(name) { return Err("Use letters, digits, dots, dashes and underscores for the name.".into()); }
    if uri.is_empty() || uri.len() > 500 || uri.starts_with('-') { return Err("Unknown printer address.".into()); }
    let mut a = vec!["printer-add", name, uri];
    if !info.is_empty() { a.push("--info"); a.push(info); }
    util::admin(&a, None, 180)
}

pub fn open_settings() -> Result<Value, String> { util::spawn("hearth-settings", &["printers"]).map(|_| json!({"ok": true})) }

// ------------------------------------------------------------------ scanner (SANE)

/// `scanimage -f '%d|%v|%m|%t%n'` lines -> [{id, vendor, model, type}].
pub fn parse_devices(text: &str) -> Vec<Value> {
    text.lines().filter_map(|l| {
        let f: Vec<&str> = l.splitn(4, '|').collect();
        (f.len() == 4 && !f[0].is_empty()).then(|| json!({"id": f[0], "vendor": f[1], "model": f[2], "type": f[3]}))
    }).collect()
}

pub fn devices() -> Result<Value, String> {
    if util::which("scanimage").is_none() { return Err("The scanner tools (sane-utils) aren't installed.".into()); }
    let o = util::run("scanimage", &["-f", "%d|%v|%m|%t%n"], 60)?;
    Ok(json!({"devices": parse_devices(&o.out)}))
}

/// A SANE device name: what scanimage -L printed (letters, digits and : / . _ - = @ % only).
pub fn valid_device(d: &str) -> bool {
    !d.is_empty() && d.len() < 300 && !d.starts_with('-') && d.chars().all(|c| c.is_ascii_alphanumeric() || ":/._-=@%+[] ".contains(c))
}

/// From `scanimage -d DEV -A`: modes, resolutions and the largest area (mm).
pub fn parse_options(text: &str) -> Value {
    let mut modes: Vec<String> = vec![];
    let mut res: Vec<u32> = vec![];
    let (mut w, mut h) = (215.9f64, 297.0f64);
    let mut sources: Vec<String> = vec![];
    for l in text.lines().map(str::trim) {
        let list = |rest: &str| -> Vec<String> { rest.split_whitespace().next().unwrap_or("").split('|').map(str::to_string).collect() };
        if let Some(r) = l.strip_prefix("--mode ") { modes = list(r); }
        else if let Some(r) = l.strip_prefix("--source ") { sources = list(r); }
        else if let Some(r) = l.strip_prefix("--resolution ") {
            let spec = r.split_whitespace().next().unwrap_or("").trim_end_matches("dpi");
            if let Some((lo, hi)) = spec.split_once("..") {
                let (lo, hi) = (lo.parse::<f64>().unwrap_or(75.0) as u32, hi.parse::<f64>().unwrap_or(600.0) as u32);
                res = [75, 100, 150, 200, 300, 600, 1200].into_iter().filter(|r| *r >= lo && *r <= hi).collect();
            } else { res = spec.split('|').filter_map(|x| x.parse::<f64>().ok().map(|v| v as u32)).collect(); }
        }
        else if let Some(r) = l.strip_prefix("-x ") { if let Some(v) = r.split("..").nth(1).and_then(|x| x.trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.').split(|c: char| !c.is_ascii_digit() && c != '.').next()).and_then(|x| x.parse().ok()) { w = v; } }
        else if let Some(r) = l.strip_prefix("-y ") { if let Some(v) = r.split("..").nth(1).and_then(|x| x.split(|c: char| !c.is_ascii_digit() && c != '.').next()).and_then(|x| x.parse().ok()) { h = v; } }
    }
    if modes.is_empty() { modes = vec!["Color".into(), "Gray".into()]; }
    if res.is_empty() { res = vec![75, 150, 300, 600]; }
    json!({"modes": modes, "resolutions": res, "width": w, "height": h, "sources": sources})
}

pub fn options(args: &Value) -> Result<Value, String> {
    let dev = util::arg(args, "device");
    if !valid_device(dev) { return Err("Unknown scanner.".into()); }
    let o = util::run("scanimage", &["-d", dev, "-A"], 60)?;
    if !o.ok() && o.out.trim().is_empty() { return Err(format!("The scanner didn't answer ({})", o.why())); }
    Ok(parse_options(&o.out))
}

static PAGES: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

fn scan_dir() -> PathBuf {
    let d = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth-scans");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Scans once: a preview (low resolution, the whole glass) or a page (the chosen area). Pages are kept until saved.
pub fn scan(args: &Value, preview: bool, cancel: &AtomicBool, progress: &(dyn Fn(Value) + Sync)) -> Result<Value, String> {
    let dev = util::arg(args, "device");
    if !valid_device(dev) { return Err("Unknown scanner.".into()); }
    let mode = match util::arg(args, "mode") { m @ ("Color" | "Gray" | "Lineart" | "Colour" | "Grayscale" | "Black & White") => m.to_string(), _ => "Color".into() };
    let pdf = util::arg(args, "format") != "png";
    let res = if preview { util::arg_u64(args, "previewRes", 75).clamp(25, 150) } else { util::arg_u64(args, "resolution", 300).clamp(50, 2400) };
    // JPEG for PDF pages (their pictures go into the PDF as they are), PNG otherwise. Lineart has no JPEG: grey.
    let jpeg = pdf || preview;
    let mode = if jpeg && mode == "Lineart" { "Gray".to_string() } else { mode };
    let n = PAGES.lock().map(|p| p.len()).unwrap_or(0);
    let file = scan_dir().join(if preview { format!("preview.{}", if jpeg { "jpg" } else { "png" }) } else { format!("page-{}-{}.{}", std::process::id(), n + 1, if jpeg { "jpg" } else { "png" }) });
    let rs = res.to_string();
    let fstr = file.to_string_lossy().into_owned();
    let mut a: Vec<String> = vec!["-d".into(), dev.into(), "--mode".into(), mode, "--resolution".into(), rs, "--format".into(), if jpeg { "jpeg".into() } else { "png".into() }, "-o".into(), fstr, "-p".into()];
    let source = util::arg(args, "source");
    if !source.is_empty() && source.len() < 60 && source.chars().all(|c| c.is_ascii_alphanumeric() || " ()-_".contains(c)) { a.push("--source".into()); a.push(source.into()); }
    if !preview {
        let num = |k: &str| args.get("area").and_then(|v| v.get(k)).and_then(Value::as_f64);
        if let (Some(l), Some(t), Some(w), Some(h)) = (num("l"), num("t"), num("w"), num("h")) {
            if w > 1.0 && h > 1.0 { for (k, v) in [("-l", l), ("-t", t), ("-x", w), ("-y", h)] { a.push(k.into()); a.push(format!("{:.1}", v.max(0.0))); } }
        }
    }
    let _ = std::fs::remove_file(&file);
    let mut last_err = String::new();
    let code = util::run_lines("scanimage", &a.iter().map(String::as_str).collect::<Vec<_>>(), cancel, |l| {
        // "-p" prints "Progress: 42.3%"
        if let Some(p) = l.trim().strip_prefix("Progress: ").and_then(|x| x.trim_end_matches('%').parse::<f64>().ok()) { progress(json!({"pct": p})); }
        else if !l.trim().is_empty() { last_err = l.trim().to_string(); }
    })?;
    if code.is_none() { return Err("Stopped.".into()); }
    let data = std::fs::read(&file).map_err(|_| format!("The scan didn't work{}", if last_err.is_empty() { String::new() } else { format!(" ({last_err})") }))?;
    if data.len() < 64 { return Err(format!("The scan didn't work ({last_err})")); }
    let mime = if jpeg { "image/jpeg" } else { "image/png" };
    let size = if jpeg { jpeg_size(&data) } else { png_size(&data).map(|(w, h)| (w, h, 3)) };
    let mut out = json!({"image": format!("data:{mime};base64,{}", util::base64(&data)), "width": size.map(|s| s.0), "height": size.map(|s| s.1), "resolution": res});
    if !preview {
        let mut p = PAGES.lock().map_err(|_| "busy")?;
        p.push(file);
        out["page"] = json!(p.len());
        out["pages"] = json!(p.len());
    }
    Ok(out)
}

pub fn pages_clear(args: &Value) -> Result<Value, String> {
    let mut p = PAGES.lock().map_err(|_| "busy")?;
    match args.get("page").and_then(Value::as_u64) {
        Some(n) if n >= 1 && (n as usize) <= p.len() => { let f = p.remove(n as usize - 1); let _ = std::fs::remove_file(f); }
        _ => { for f in p.drain(..) { let _ = std::fs::remove_file(f); } }
    }
    Ok(json!({"pages": p.len()}))
}

/// Width, height and colour components of a JPEG (from its frame header).
pub fn jpeg_size(b: &[u8]) -> Option<(u32, u32, u8)> {
    if b.len() < 4 || b[0] != 0xFF || b[1] != 0xD8 { return None; }
    let mut i = 2;
    while i + 9 < b.len() {
        if b[i] != 0xFF { i += 1; continue; }
        let m = b[i + 1];
        if m == 0xD8 || m == 0x01 || (0xD0..=0xD7).contains(&m) || m == 0xFF { i += if m == 0xFF { 1 } else { 2 }; continue; }
        let len = ((b[i + 2] as usize) << 8) | b[i + 3] as usize;
        if (0xC0..=0xCF).contains(&m) && m != 0xC4 && m != 0xC8 && m != 0xCC {
            let h = ((b[i + 5] as u32) << 8) | b[i + 6] as u32;
            let w = ((b[i + 7] as u32) << 8) | b[i + 8] as u32;
            return Some((w, h, b[i + 9]));
        }
        i += 2 + len;
    }
    None
}

pub fn png_size(b: &[u8]) -> Option<(u32, u32)> {
    if b.len() < 24 || &b[..8] != b"\x89PNG\r\n\x1a\n" { return None; }
    Some((u32::from_be_bytes([b[16], b[17], b[18], b[19]]), u32::from_be_bytes([b[20], b[21], b[22], b[23]])))
}

/// A PDF with one JPEG picture per page, each page the size it was scanned at (`dpi`).
pub fn pdf_from_jpegs(pages: &[(Vec<u8>, u32)]) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets: Vec<usize> = vec![];
    let n = pages.len();
    // Objects: 1 catalog, 2 pages, then per page: page, contents, image.
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 3 + i * 3)).collect();
    let mut objs: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).into_bytes(),
    ];
    for (i, (jpg, dpi)) in pages.iter().enumerate() {
        let (w, h, comps) = jpeg_size(jpg).ok_or("A scanned page isn't a JPEG picture.")?;
        let dpi = (*dpi).max(1) as f64;
        let (pw, ph) = (w as f64 * 72.0 / dpi, h as f64 * 72.0 / dpi);
        let (page, contents, image) = (3 + i * 3, 4 + i * 3, 5 + i * 3);
        let _ = page;
        objs.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw:.2} {ph:.2}] /Resources << /XObject << /Im{i} {image} 0 R >> >> /Contents {contents} 0 R >>").into_bytes());
        let stream = format!("q {pw:.2} 0 0 {ph:.2} 0 0 cm /Im{i} Do Q");
        objs.push(format!("<< /Length {} >>\nstream\n{stream}\nendstream", stream.len()).into_bytes());
        let cs = match comps { 1 => "/DeviceGray", 4 => "/DeviceCMYK", _ => "/DeviceRGB" };
        let mut img = format!("<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace {cs} /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", jpg.len()).into_bytes();
        img.extend_from_slice(jpg);
        img.extend_from_slice(b"\nendstream");
        objs.push(img);
    }
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in &offsets { out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes()); }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    Ok(out)
}

/// Saves the scanned pages: one PDF, or one PNG per page, in Documents/Scans. Returns the files.
pub fn save(args: &Value) -> Result<Value, String> {
    let pages: Vec<PathBuf> = PAGES.lock().map_err(|_| "busy")?.clone();
    if pages.is_empty() { return Err("Scan a page first.".into()); }
    let dir = util::user_dir("DOCUMENTS", "Documents").join("Scans");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't make {}: {e}", dir.display()))?;
    let base: String = util::arg(args, "name").chars().filter(|c| !"/\\\0".contains(*c)).take(80).collect::<String>().trim().to_string();
    let base = if base.is_empty() || base.starts_with('.') { format!("Scan {}", util::stamp()) } else { base };
    let res = util::arg_u64(args, "resolution", 300) as u32;
    let mut saved = vec![];
    if pages.iter().all(|p| p.extension().map(|e| e == "jpg").unwrap_or(false)) {
        let data: Vec<(Vec<u8>, u32)> = pages.iter().map(|p| std::fs::read(p).map(|d| (d, res)).map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
        let pdf = pdf_from_jpegs(&data)?;
        let f = util::unique_path(&dir, &format!("{base}.pdf"));
        std::fs::write(&f, pdf).map_err(|e| format!("Couldn't save: {e}"))?;
        saved.push(f.to_string_lossy().into_owned());
    } else {
        for (i, p) in pages.iter().enumerate() {
            let name = if pages.len() == 1 { format!("{base}.png") } else { format!("{base} {}.png", i + 1) };
            let f = util::unique_path(&dir, &name);
            std::fs::copy(p, &f).map_err(|e| format!("Couldn't save: {e}"))?;
            saved.push(f.to_string_lossy().into_owned());
        }
    }
    let _ = pages_clear(&json!({}));
    Ok(json!({"files": saved, "folder": dir.to_string_lossy()}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert!(valid_printer("HP_LaserJet-M404") && !valid_printer("-x") && !valid_printer("a b") && !valid_printer(""));
        assert!(valid_job("HP_LaserJet-12") && !valid_job("HP") && !valid_job("HP-x") && !valid_job("-1"));
        assert!(valid_device("airscan:e0:Canon TS5000") && valid_device("escl:https://192.168.1.20:443") && !valid_device("-h") && !valid_device("a;b"));
    }

    #[test]
    fn jobs() {
        let j = parse_jobs("HP-12 sam 1024 Wed 22 Oct 2026 10:00:00 AM CEST\nnonsense\n");
        assert_eq!(j.len(), 1);
        assert_eq!((j[0]["printer"].as_str(), j[0]["user"].as_str(), j[0]["size"].as_u64()), (Some("HP"), Some("sam"), Some(1024)));
    }

    #[test]
    fn scanner_lists() {
        let d = parse_devices("airscan:e0:Canon TS5000|Canon|TS5000 series|eSCL network scanner\n");
        assert_eq!(d[0]["model"], "TS5000 series");
        let o = parse_options("All options specific to device `x':\n  Standard:\n    --resolution 75|150|300|600dpi [300]\n    --mode Color|Gray [Color]\n    --source Flatbed|ADF [Flatbed]\n  Geometry:\n    -l 0..215.9mm [0]\n    -t 0..297.011mm [0]\n    -x 0..215.9mm [215.9]\n    -y 0..297.011mm [297.011]\n");
        assert_eq!(o["resolutions"], json!([75, 150, 300, 600]));
        assert_eq!(o["modes"], json!(["Color", "Gray"]));
        assert_eq!(o["sources"], json!(["Flatbed", "ADF"]));
        assert!((o["height"].as_f64().unwrap() - 297.011).abs() < 0.01);
        let r = parse_options("    --resolution 50..1200dpi (in steps of 1) [300]\n");
        assert_eq!(r["resolutions"], json!([75, 100, 150, 200, 300, 600, 1200]));
    }

    fn tiny_jpeg(w: u16, h: u16, comps: u8) -> Vec<u8> {
        let mut b = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0];
        b.extend_from_slice(&[0xFF, 0xC0, 0, 11, 8, (h >> 8) as u8, h as u8, (w >> 8) as u8, w as u8, comps, 1, 0x11, 0]);
        b.extend_from_slice(&[0xFF, 0xD9]);
        b
    }

    #[test]
    fn pdf() {
        assert_eq!(jpeg_size(&tiny_jpeg(2550, 3300, 3)), Some((2550, 3300, 3)));
        let p = pdf_from_jpegs(&[(tiny_jpeg(2550, 3300, 3), 300), (tiny_jpeg(1275, 1650, 1), 150)]).unwrap();
        let find = |hay: &[u8], n: &[u8]| hay.windows(n.len()).rposition(|w| w == n);
        let t = String::from_utf8_lossy(&p).into_owned();
        assert!(p.starts_with(b"%PDF-1.4") && t.trim_end().ends_with("%%EOF"));
        assert!(t.contains("/Count 2") && t.contains("/MediaBox [0 0 612.00 792.00]") && t.contains("/DeviceGray"));
        // The cross-reference table points at each object (byte offsets in the file).
        let sx = find(&p, b"startxref\n").unwrap() + 10;
        let at: usize = std::str::from_utf8(&p[sx..]).unwrap().lines().next().unwrap().parse().unwrap();
        assert!(p[at..].starts_with(b"xref\n0 9\n"));
        let first: usize = std::str::from_utf8(&p[at + 9 + 20..at + 9 + 30]).unwrap().parse().unwrap();
        assert!(p[first..].starts_with(b"1 0 obj"));
        assert_eq!(png_size(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\x01\0\0\0\0\x80"), Some((256, 128)));
    }
}
