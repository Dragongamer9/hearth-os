//! One note changed on two PCs (page 6): This PC and GitHub side by side, and Keep mine, Keep theirs or Merge both.
//! Nothing is lost: whichever is picked, both versions are first kept as copies in ~/.local/share/hearth/notes/kept/
//! (outside the workspace, so its check stays tidy), and GitHub keeps every earlier version too.

use std::path::{Path, PathBuf};

/// Each line of `a` and `b` and whether it's only on that side (the side-by-side view's highlights).
pub fn sides(a: &str, b: &str) -> (Vec<(String, bool)>, Vec<(String, bool)>) {
    let (x, y): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    let pairs = lcs(&x, &y);
    let mut ia = vec![true; x.len()];
    let mut ib = vec![true; y.len()];
    for (i, j) in pairs {
        ia[i] = false;
        ib[j] = false;
    }
    (x.iter().zip(ia).map(|(l, c)| (l.to_string(), c)).collect(), y.iter().zip(ib).map(|(l, c)| (l.to_string(), c)).collect())
}

/// The lines both have, in order (longest common subsequence), as index pairs.
fn lcs(x: &[&str], y: &[&str]) -> Vec<(usize, usize)> {
    let (n, m) = (x.len(), y.len());
    if n == 0 || m == 0 {
        return Vec::new();
    }
    if n.saturating_mul(m) > 9_000_000 {
        // Very long notes: only equal lines at the same place.
        return (0..n.min(m)).filter(|&i| x[i] == y[i]).map(|i| (i, i)).collect();
    }
    let mut t = vec![0u32; (n + 1) * (m + 1)];
    let at = |i: usize, j: usize| i * (m + 1) + j;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[at(i, j)] = if x[i] == y[j] { t[at(i + 1, j + 1)] + 1 } else { t[at(i + 1, j)].max(t[at(i, j + 1)]) };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < n && j < m {
        if x[i] == y[j] {
            out.push((i, j));
            i += 1;
            j += 1;
        } else if t[at(i + 1, j)] >= t[at(i, j + 1)] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// "- 03:04 Made: …" → 03:04 (the log's lines' times).
fn time_of(l: &str) -> Option<u32> {
    let t = l.trim_start().trim_start_matches(['-', '*']).trim_start();
    if t.len() < 5 || t.as_bytes()[2] != b':' {
        return None;
    }
    let h: u32 = t[0..2].parse().ok()?;
    let m: u32 = t[3..5].parse().ok()?;
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// Merge both: every line from each side. Where they differ, both sides' lines go in, in time order when they're
/// timed log lines (else this PC's first); lines without a time stay right after the line they followed.
pub fn merge_both(mine: &str, theirs: &str) -> String {
    let (x, y): (Vec<&str>, Vec<&str>) = (mine.lines().collect(), theirs.lines().collect());
    let pairs = lcs(&x, &y);
    let mut out: Vec<String> = Vec::new();
    let (mut i, mut j) = (0, 0);
    let flush = |a: &[&str], b: &[&str], out: &mut Vec<String>| {
        // Groups: a timed line and the untimed lines after it stay together.
        let group = |v: &[&str], side: u8| -> Vec<(Option<u32>, u8, Vec<String>)> {
            let mut g: Vec<(Option<u32>, u8, Vec<String>)> = Vec::new();
            for l in v {
                match (time_of(l), g.last_mut()) {
                    (None, Some(last)) => last.2.push(l.to_string()),
                    (t, _) => g.push((t, side, vec![l.to_string()])),
                }
            }
            g
        };
        let mut all = group(a, 0);
        all.extend(group(b, 1));
        if all.iter().all(|g| g.0.is_some()) {
            all.sort_by_key(|g| (g.0, g.1));
        }
        for g in all {
            out.extend(g.2);
        }
    };
    for (pi, pj) in pairs {
        flush(&x[i..pi], &y[j..pj], &mut out);
        out.push(x[pi].to_string());
        i = pi + 1;
        j = pj + 1;
    }
    flush(&x[i..], &y[j..], &mut out);
    let mut t = out.join("\n");
    if mine.ends_with('\n') || theirs.ends_with('\n') {
        t.push('\n');
    }
    t
}

/// Keeps both versions as copies before anything is picked: kept/<date time>/<note>.this-pc and .github.
pub fn keep_copies(rel: &str, mine: &str, theirs: &str, stamp: &str) -> std::io::Result<PathBuf> {
    let dir = crate::paths::kept().join(stamp);
    let base = dir.join(rel);
    let a = PathBuf::from(format!("{}.this-pc", base.display()));
    let b = PathBuf::from(format!("{}.github", base.display()));
    crate::save::atomic(&a, mine.as_bytes())?;
    crate::save::atomic(&b, theirs.as_bytes())?;
    Ok(dir)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Mine,
    Theirs,
    Both,
}

/// Settles one conflicted note with `pick`: copies kept, the note written, and when it was the last one the merge is
/// committed (with Git's own message, as the person). Returns whether the merge is finished.
pub fn settle(dir: &Path, rel: &str, pick: Pick, stamp: &str) -> Result<bool, String> {
    let mine = crate::git::stage(dir, 2, rel).unwrap_or_default();
    let theirs = crate::git::stage(dir, 3, rel).unwrap_or_default();
    keep_copies(rel, &mine, &theirs, stamp).map_err(|e| format!("Couldn't keep the copies, so nothing was changed: {e}"))?;
    let text = match pick {
        Pick::Mine => mine,
        Pick::Theirs => theirs,
        Pick::Both => merge_both(&mine, &theirs),
    };
    crate::save::note(dir, rel, &text).map_err(|e| e.to_string())?;
    let a = crate::git::git(dir, &["add", "--", rel], 30);
    if !a.ok {
        return Err(a.why());
    }
    if !crate::git::conflicts(dir).is_empty() {
        return Ok(false);
    }
    let c = crate::git::git(dir, &["commit", "-q", "--no-edit"], 60);
    if !c.ok {
        return Err(c.why());
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINE: &str = "# 2026-11-20\n\n- 00:22 Made: KDE settings pages\n- 01:10 Made: Server mode\n- 03:04 Made: Hearth Notes mockups\nStored: hearth/mockups/hearth-notes.\n- 03:10 Decided: graph modes go in\n";
    const THEIRS: &str = "# 2026-11-20\n\n- 00:22 Made: KDE settings pages\n- 01:10 Made: Server mode\n- 03:06 Changed: laptop autostart\nufw keeps the Hearth Link rules.\n";

    #[test]
    fn merge_both_keeps_every_line_in_time_order() {
        let m = merge_both(MINE, THEIRS);
        assert_eq!(
            m,
            "# 2026-11-20\n\n- 00:22 Made: KDE settings pages\n- 01:10 Made: Server mode\n- 03:04 Made: Hearth Notes mockups\nStored: hearth/mockups/hearth-notes.\n- 03:06 Changed: laptop autostart\nufw keeps the Hearth Link rules.\n- 03:10 Decided: graph modes go in\n"
        );
        for l in MINE.lines().chain(THEIRS.lines()) {
            assert!(m.lines().any(|x| x == l), "nothing lost: {l}");
        }
        // Untimed differences: this PC's lines first, then theirs.
        assert_eq!(merge_both("a\nb\nz\n", "a\nc\nz\n"), "a\nb\nc\nz\n");
        let (a, b) = sides(MINE, THEIRS);
        assert!(a[4].1 && !a[3].1, "the changed lines are marked");
        assert!(b[4].1);
    }

    #[test]
    fn copies_are_kept_outside_the_workspace() {
        let _l = crate::env_lock();
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_DATA_HOME", d.path().join("data"));
        let k = keep_copies("log/2026-11-20.md", MINE, THEIRS, "2026-11-20-0310").unwrap();
        assert_eq!(std::fs::read_to_string(k.join("log/2026-11-20.md.this-pc")).unwrap(), MINE);
        assert_eq!(std::fs::read_to_string(k.join("log/2026-11-20.md.github")).unwrap(), THEIRS);
        std::env::remove_var("XDG_DATA_HOME");
    }
}
