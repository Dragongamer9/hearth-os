//! Hearth OS version numbers as people see them (docs/UPDATES.md, "Version numbers"). From 1.0 on, Hearth OS is
//! MAJOR.MINOR: the build files say 1.0.0, 1.1.0, people see "1.0", "1.1". 0.x versions (0.6.63) are shown whole.
//! Comparing versions is appchannel::cmp_versions / appupdate::newer (missing parts count as 0, so "1.0" == "1.0.0").

/// This build's release date (YYYY-MM-DD) from its docs/ROADMAP.md heading (build.rs), or "" before it's released.
pub const BUILD_DATE: &str = env!("HEARTH_VERSION_DATE");
/// Where a Hearth OS image keeps its release date (written at image build, base-*.yml, next to .../version).
pub const IMAGE_DATE_FILE: &str = "/usr/share/nimbus/version-date";

/// 1.0.0 -> "1.0", 1.10.0 -> "1.10", v2.0.0 -> "v2.0", 1.0.0+app.3 -> "1.0+app.3"; 0.6.63 and 1.0.1 stay as they are.
pub fn show(v: &str) -> String {
    let v = v.trim();
    let (pre, rest) = match v.strip_prefix('v') { Some(r) => ("v", r), None => ("", v) };
    let end = rest.find(|c: char| c == '+' || c == '-').unwrap_or(rest.len());
    let (base, tail) = rest.split_at(end);
    let parts: Vec<&str> = base.split('.').collect();
    let num = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if parts.len() == 3 && parts.iter().all(|p| num(p)) && parts[2] == "0" && parts[0].parse::<u64>().map(|m| m >= 1).unwrap_or(false) {
        return format!("{pre}{}.{}{tail}", parts[0], parts[1]);
    }
    v.to_string()
}

/// A YYYY-MM-DD date, or None.
pub fn valid_date(d: &str) -> Option<String> {
    let d = d.trim();
    let ok = d.len() == 10 && d.chars().enumerate().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() });
    ok.then(|| d.to_string())
}

/// The release date of `version` when it's this build's own version (a bundle or AppImage carries its own).
pub fn date_of_build(version: &str) -> Option<String> {
    (crate::appchannel::cmp_versions(version, crate::VERSION) == std::cmp::Ordering::Equal).then(|| valid_date(BUILD_DATE)).flatten()
}

/// The Hearth OS image's version and release date (/usr/share/nimbus/version and version-date), when this is one.
pub fn image() -> (Option<String>, Option<String>) {
    let read = |p: &str| std::fs::read_to_string(p).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty() && s != "0.0.0");
    (read("/usr/share/nimbus/version"), read(IMAGE_DATE_FILE).and_then(|d| valid_date(&d)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shown() {
        for (v, want) in [("1.0.0", "1.0"), ("1.1.0", "1.1"), ("1.10.0", "1.10"), ("2.0.0", "2.0"), ("v1.0.0", "v1.0"), ("1.0", "1.0"),
                          ("1.0.0+app.3", "1.0+app.3"), ("1.0.1", "1.0.1"), ("0.6.63", "0.6.63"), ("0.6.0", "0.6.0"), ("", ""), ("Preview 3", "Preview 3")] {
            assert_eq!(show(v), want, "{v}");
        }
    }
    #[test]
    fn dates() {
        assert_eq!(valid_date("2026-11-02\n").as_deref(), Some("2026-11-02"));
        assert_eq!(valid_date("2026-11-2"), None);
        assert_eq!(valid_date(""), None);
        // The date built in is either empty (before the release heading has one) or a real date.
        assert!(BUILD_DATE.is_empty() || valid_date(BUILD_DATE).is_some(), "{BUILD_DATE}");
        assert_eq!(date_of_build("0.0.1"), None);
    }
}
