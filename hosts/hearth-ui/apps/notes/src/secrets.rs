//! "Warn if a note looks like a secret" (on by default): before anything is committed, every changed text file is
//! looked through for what looks like a token, a password or a private key. Found: the sync stops and you decide. What
//! was found is shown with most of it hidden (never the whole thing), and none of it is kept or sent anywhere.

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    /// Line (from 1).
    pub line: usize,
    pub what: &'static str,
    /// The line with the secret-looking part hidden but its start ("ghp_ab…").
    pub shown: String,
}

fn word_chars(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn run_from(s: &str, at: usize, ok: impl Fn(char) -> bool) -> usize {
    s[at..].chars().take_while(|c| ok(*c)).map(|c| c.len_utf8()).sum()
}

fn hide(line: &str, start: usize, len: usize) -> String {
    let keep: String = line[start..start + len].chars().take(6).collect();
    let mut out = String::new();
    out.push_str(&line[..start]);
    out.push_str(&keep);
    out.push('…');
    out.push_str(&line[start + len..]);
    out.chars().take(140).collect()
}

/// Known token shapes: (prefix, what it is, characters after the prefix at least).
const PREFIXES: [(&str, &str, usize); 12] = [
    ("ghp_", "a GitHub token", 30),
    ("gho_", "a GitHub token", 30),
    ("ghu_", "a GitHub token", 30),
    ("ghs_", "a GitHub token", 30),
    ("ghr_", "a GitHub token", 30),
    ("github_pat_", "a GitHub token", 40),
    ("glpat-", "a GitLab token", 20),
    ("sk-ant-", "an Anthropic key", 30),
    ("sk-", "an API key", 32),
    ("xoxb-", "a Slack token", 20),
    ("AKIA", "an Amazon key", 16),
    ("AIza", "a Google key", 35),
];

const WORDS: [&str; 9] = ["password", "passwd", "pwd", "secret", "token", "api_key", "apikey", "api-key", "private_key"];

/// What in `text` looks like a secret.
pub fn scan(text: &str) -> Vec<Hit> {
    let mut out = Vec::new();
    let mut in_key = false;
    for (n, line) in text.lines().enumerate() {
        let no = n + 1;
        if line.contains("-----BEGIN") && line.contains("PRIVATE KEY") {
            out.push(Hit { line: no, what: "a private key", shown: "-----BEGIN … PRIVATE KEY-----".into() });
            in_key = true;
            continue;
        }
        if in_key {
            if line.contains("-----END") {
                in_key = false;
            }
            continue;
        }
        let mut found = false;
        for (p, what, min) in PREFIXES {
            let mut from = 0;
            while let Some(at) = line[from..].find(p) {
                let s = from + at;
                let before = line[..s].chars().next_back().unwrap_or(' ');
                let len = p.len() + run_from(line, s + p.len(), word_chars);
                from = s + p.len();
                if word_chars(before) || len - p.len() < min {
                    continue;
                }
                let body = &line[s + p.len()..s + len];
                // A real key mixes letters and digits; "sk-xxxxxxxx…" placeholders don't count.
                let mixed = body.chars().any(|c| c.is_ascii_digit()) && body.chars().any(|c| c.is_ascii_alphabetic()) && !body.chars().all(|c| c == 'x' || c == 'X');
                if mixed {
                    out.push(Hit { line: no, what, shown: hide(line, s, len) });
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        if found {
            continue;
        }
        // password = "…", token: …, API_KEY=… (an assignment with a value that isn't a word or a placeholder).
        let low = line.to_lowercase();
        for w in WORDS {
            let Some(at) = low.find(w) else { continue };
            let rest = &line[at + w.len()..];
            let rest_t = rest.trim_start_matches(|c: char| c == '"' || c == '\'' || c == '_' || c.is_ascii_alphabetic());
            let Some(after) = rest_t.trim_start().strip_prefix(['=', ':']) else { continue };
            let v = after.trim().trim_matches(|c| c == '"' || c == '\'' || c == '`' || c == ',' || c == ';');
            let v = v.split_whitespace().next().unwrap_or("");
            let placeholder = v.starts_with('<') || v.starts_with("${") || v.starts_with('$') || v.contains("...") || v.contains('…') || v.eq_ignore_ascii_case("none") || v.eq_ignore_ascii_case("null");
            let strong = v.len() >= 8 && v.chars().any(|c| c.is_ascii_digit()) && v.chars().any(|c| c.is_ascii_alphabetic()) && !v.contains('/') && !v.contains('.');
            if strong && !placeholder {
                let s = line.len() - after.trim_start().len();
                let len = after.trim_start().split_whitespace().next().unwrap_or("").len();
                out.push(Hit { line: no, what: "a password or token", shown: hide(line, s, len.min(line.len() - s)) });
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_secret_looking_lines_and_shows_only_their_start() {
        let fake_gh = format!("ghp_{}", "a1B2c3D4e5F6g7H8i9J0k1L2m3N4o5P6q7R8");
        let text = format!(
            "# Notes\nThe token is {fake_gh} here\npassword = \"hunter2abc9\"\n-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAA\n-----END OPENSSH PRIVATE KEY-----\nAWS AKIAABCDEFGH12345678 key\n"
        );
        let h = scan(&text);
        assert_eq!(h.len(), 4, "{h:?}");
        assert_eq!(h[0].line, 2);
        assert_eq!(h[0].what, "a GitHub token");
        assert!(!h[0].shown.contains(&fake_gh), "never the whole secret: {}", h[0].shown);
        assert_eq!(h[1].what, "a password or token");
        assert!(!h[1].shown.contains("hunter2abc9"));
        assert_eq!(h[2].what, "a private key");
        assert_eq!(h[3].what, "an Amazon key");
    }

    #[test]
    fn ordinary_notes_are_left_alone() {
        let text = "Hearth Notes never asks for or stores a password or token.\nThe password: use the PC's sign-in\ncommit 7c41e2a, run 44696274874, sha256 3f5a9c0e1b2d4f6a8c0e2b4d6f8a0c2e4b6d8f0a2c4e6b8d0f2a4c6e8b0d2f4a\nTOKEN=<your token>\napi_key: ${API_KEY}\nsk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\nThe ask-proxy and task-manager notes\n";
        assert_eq!(scan(text), vec![]);
    }
}
