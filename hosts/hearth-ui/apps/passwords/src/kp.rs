//! KeePass files (.kdbx) through KeePassXC's own command (keepassxc-cli, GPL-3): read with `export --format xml`
//! (the file's master password on its stdin, the answer only in memory), a password changed with `edit -p`. The file
//! itself stays as KeePassXC keeps it: encrypted, where it is.

use crate::vault::{Item, Kind};
use hearth_ui::run::Output;

fn cli() -> Result<String, String> {
    hearth_oss::which(&["keepassxc-cli"], &[]).map(|p| p.to_string_lossy().to_string()).ok_or_else(|| "KeePass files open with KeePassXC's own command: install KeePassXC from the Hearth Store".to_string())
}

fn why(o: &Output) -> String {
    let e = o.err.to_lowercase();
    if e.contains("invalid credentials") || e.contains("wrong key") || e.contains("hmac mismatch") {
        "Wrong master password for that file".into()
    } else {
        format!("KeePassXC couldn't open it: {}", o.why())
    }
}

/// The file's entries (its groups as folders).
pub fn open(file: &str, pw: &str) -> Result<(String, Vec<Item>), String> {
    let c = cli()?;
    let o = hearth_oss::proc::blocking(&[c, "export".into(), "-q".into(), "--format".into(), "xml".into(), file.into()], format!("{pw}\n").as_bytes(), &[]);
    if !o.ok {
        return Err(why(&o));
    }
    let name = std::path::Path::new(file).file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
    Ok((file.to_string(), parse(&o.out, &format!("KeePass · {name}"))))
}

pub fn parse(xml: &str, folder: &str) -> Vec<Item> {
    let Ok(doc) = roxmltree::Document::parse(xml) else { return Vec::new() };
    let mut out = Vec::new();
    for e in doc.descendants().filter(|n| n.has_tag_name("Entry") && n.parent().map(|p| p.has_tag_name("Group")).unwrap_or(false)) {
        let get = |k: &str| {
            e.children()
                .filter(|s| s.has_tag_name("String"))
                .find(|s| s.children().find(|c| c.has_tag_name("Key")).and_then(|c| c.text()) == Some(k))
                .and_then(|s| s.children().find(|c| c.has_tag_name("Value")).and_then(|c| c.text()))
                .unwrap_or("")
                .to_string()
        };
        let mut path: Vec<String> = e.ancestors().filter(|a| a.has_tag_name("Group")).filter_map(|g| g.children().find(|c| c.has_tag_name("Name")).and_then(|c| c.text()).map(str::to_string)).collect();
        path.reverse();
        if path.first().map(|r| r == "Recycle Bin").unwrap_or(false) || path.iter().any(|p| p == "Recycle Bin") {
            continue;
        }
        let title = get("Title");
        let id = format!("kp:{}/{}", path.iter().skip(1).cloned().collect::<Vec<_>>().join("/"), title);
        let group = path.last().cloned().unwrap_or_default();
        let times = |k: &str| e.children().find(|c| c.has_tag_name("Times")).and_then(|t| t.children().find(|c| c.has_tag_name(k))).and_then(|c| c.text()).unwrap_or("").to_string();
        out.push(Item {
            id,
            kind: Some(if group.eq_ignore_ascii_case("wi-fi") { Kind::Wifi } else if get("Password").is_empty() && !get("Notes").is_empty() { Kind::Note } else { Kind::Login }),
            name: title,
            user: get("UserName"),
            password: get("Password"),
            totp: get("otp"),
            uri: get("URL"),
            notes: get("Notes"),
            folder: folder.to_string(),
            favourite: false,
            card_last4: String::new(),
            changed: times("LastModificationTime"),
            made: times("CreationTime"),
            history: Vec::new(),
            org: String::new(),
            from: "keepass".into(),
            raw: serde_json::Value::Null,
        });
    }
    out
}

/// Use it for this entry: its password changed in the file by KeePassXC.
pub fn set_password(file: &str, pw: &str, it: &Item, new: &str) -> Result<String, String> {
    let c = cli()?;
    let entry = it.id.trim_start_matches("kp:").to_string();
    let o = hearth_oss::proc::blocking(&[c, "edit".into(), "-q".into(), "-p".into(), file.into(), entry], format!("{pw}\n{new}\n{new}\n").as_bytes(), &[]);
    if o.ok {
        Ok(format!("\"{}\" has its new password in {}.", it.name, std::path::Path::new(file).file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default()))
    } else {
        Err(why(&o))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn keepassxcs_export_is_read() {
        let x = r#"<KeePassFile><Root><Group><Name>Passwords</Name>
            <Group><Name>Games</Name><Entry><String><Key>Title</Key><Value>GOG</Value></String><String><Key>UserName</Key><Value>me</Value></String>
            <String><Key>Password</Key><Value ProtectInMemory="True">p@ss</Value></String><String><Key>URL</Key><Value>gog.com</Value></String>
            <Times><CreationTime>2026-08-19T10:00:00Z</CreationTime></Times></Entry></Group>
            <Group><Name>Recycle Bin</Name><Entry><String><Key>Title</Key><Value>old</Value></String></Entry></Group></Group></Root></KeePassFile>"#;
        let v = super::parse(x, "KeePass · mine.kdbx");
        assert_eq!(v.len(), 1);
        assert_eq!((v[0].name.as_str(), v[0].user.as_str(), v[0].password.as_str(), v[0].id.as_str()), ("GOG", "me", "p@ss", "kp:Games/GOG"));
        assert_eq!(v[0].made, "2026-08-19T10:00:00Z");
    }
}
