//! Your vault in memory while it's unlocked: the items opened from Vaultwarden's sync (or a KeePass file through
//! KeePassXC's own command), their folders, 2-step codes (RFC 6238), the password maker and the security check.
//! Nothing here is written anywhere in the open: the copy on this PC (vault.json) is the server's, still encrypted.

use crate::bw::{self, Keys};
use hmac::{Hmac, Mac};
use serde_json::Value;
use zeroize::Zeroize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Login,
    Note,
    Card,
    Identity,
    Wifi,
}

#[derive(Clone, Default)]
pub struct Item {
    pub id: String,
    pub kind: Option<Kind>,
    pub name: String,
    pub user: String,
    pub password: String,
    pub totp: String,
    pub uri: String,
    pub notes: String,
    pub folder: String,
    pub favourite: bool,
    pub card_last4: String,
    pub changed: String,
    pub made: String,
    /// (when, what): password changes from its history
    pub history: Vec<(String, String)>,
    pub org: String,
    /// where it came from: "server", or the KeePass file's name
    pub from: String,
    /// the server's item as it came (still encrypted), to send back changed
    pub raw: Value,
}
impl Item {
    /// Its secrets wiped from memory (the app locks).
    pub fn wipe(&mut self) {
        self.password.zeroize();
        self.totp.zeroize();
        self.notes.zeroize();
    }
}
impl std::fmt::Debug for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Item({})", self.id)
    }
}

impl Item {
    pub fn kind(&self) -> Kind {
        self.kind.unwrap_or(Kind::Login)
    }
    /// Under its name: the user name, "Wi-Fi · WPA3", "card · ends 4821"…
    pub fn sub(&self) -> String {
        match self.kind() {
            Kind::Card => format!("card · ends {}", self.card_last4),
            Kind::Wifi if self.user.is_empty() => "Wi-Fi".into(),
            Kind::Wifi => format!("Wi-Fi · {}", self.user),
            Kind::Note if self.user.is_empty() => "secure note".into(),
            _ if !self.user.is_empty() => self.user.clone(),
            _ => self.uri.clone(),
        }
    }
    pub fn letters(&self) -> String {
        let w: Vec<&str> = self.name.split_whitespace().filter(|w| w.chars().next().map(char::is_alphanumeric).unwrap_or(false)).collect();
        let s: String = match w.len() {
            0 => "?".into(),
            // one word: its first letter and its next capital (GitHub: GH), else its first two
            1 => {
                let mut c = w[0].chars();
                let first = c.next().unwrap_or('?');
                match c.clone().find(|x| x.is_uppercase()) {
                    Some(cap) => format!("{first}{cap}"),
                    None => w[0].chars().take(2).collect(),
                }
            }
            _ => w.iter().take(2).filter_map(|x| x.chars().next()).collect(),
        };
        s.to_uppercase()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Folder {
    pub id: String,
    pub name: String,
}

fn s(v: &Value, k: &[&str]) -> String {
    hearth_oss::ci_str(v, k)
}

/// The sync answer opened with your vault's key (and the organisations' keys for shared items).
pub fn open(sync: &Value, user: &Keys, private_der: Option<&[u8]>) -> (Vec<Item>, Vec<Folder>) {
    let mut orgs: Vec<(String, Keys)> = Vec::new();
    if let (Some(der), Some(list)) = (private_der, hearth_oss::ci(sync, &["profile", "Profile"]).and_then(|p| hearth_oss::ci(p, &["organizations", "Organizations"])).and_then(Value::as_array)) {
        for o in list {
            if let Some(k) = bw::org_key(&s(o, &["key", "Key"]), der) {
                orgs.push((s(o, &["id", "Id"]), k));
            }
        }
    }
    let folders: Vec<Folder> = hearth_oss::ci(sync, &["folders", "Folders"]).and_then(Value::as_array).map(|a| a.iter().map(|f| Folder { id: s(f, &["id", "Id"]), name: bw::decrypt_str(&s(f, &["name", "Name"]), user) }).collect()).unwrap_or_default();
    let mut items = Vec::new();
    for c in hearth_oss::ci(sync, &["ciphers", "Ciphers"]).and_then(Value::as_array).cloned().unwrap_or_default() {
        if hearth_oss::ci(&c, &["deletedDate", "DeletedDate"]).map(|d| !d.is_null()).unwrap_or(false) {
            continue;
        }
        let org = s(&c, &["organizationId", "OrganizationId"]);
        let k = if org.is_empty() { user } else { match orgs.iter().find(|o| o.0 == org) { Some(o) => &o.1, None => continue } };
        let d = |v: &Value, key: &[&str]| bw::decrypt_str(&s(v, key), k);
        let login = hearth_oss::ci(&c, &["login", "Login"]).cloned().unwrap_or(Value::Null);
        let card = hearth_oss::ci(&c, &["card", "Card"]).cloned().unwrap_or(Value::Null);
        let t = hearth_oss::ci(&c, &["type", "Type"]).and_then(Value::as_u64).unwrap_or(1);
        let name = d(&c, &["name", "Name"]);
        let folder_id = s(&c, &["folderId", "FolderId"]);
        let folder = folders.iter().find(|f| f.id == folder_id).map(|f| f.name.clone()).unwrap_or_default();
        let uri = hearth_oss::ci(&login, &["uris", "Uris"]).and_then(Value::as_array).and_then(|a| a.first()).map(|u| d(u, &["uri", "Uri"])).unwrap_or_default();
        let mut kind = match t {
            2 => Kind::Note,
            3 => Kind::Card,
            4 => Kind::Identity,
            _ => Kind::Login,
        };
        if folder.eq_ignore_ascii_case("wi-fi") || name.to_lowercase().contains("wi-fi") || uri.starts_with("wifi:") {
            kind = Kind::Wifi;
        }
        let number = d(&card, &["number", "Number"]);
        let history = hearth_oss::ci(&c, &["passwordHistory", "PasswordHistory"])
            .and_then(Value::as_array)
            .map(|a| a.iter().map(|h| (s(h, &["lastUsedDate", "LastUsedDate"]), "Password changed".to_string())).collect())
            .unwrap_or_default();
        items.push(Item {
            id: s(&c, &["id", "Id"]),
            kind: Some(kind),
            name,
            user: d(&login, &["username", "Username"]),
            password: d(&login, &["password", "Password"]),
            totp: d(&login, &["totp", "Totp"]),
            uri,
            notes: d(&c, &["notes", "Notes"]),
            folder,
            favourite: hearth_oss::ci(&c, &["favorite", "Favorite"]).and_then(Value::as_bool).unwrap_or(false),
            card_last4: number.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect(),
            changed: s(&c, &["revisionDate", "RevisionDate"]),
            made: s(&c, &["creationDate", "CreationDate"]),
            history,
            org,
            from: "server".into(),
            raw: c.clone(),
        });
    }
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    (items, folders)
}

// ------------------------------------------------------------------ 2-step codes (RFC 6238)

/// (secret, digits, period, algorithm) from an otpauth:// address or a bare base32 secret.
pub fn otp_params(s: &str) -> (Vec<u8>, u32, u64, String) {
    let s = s.trim();
    if let Some(q) = s.strip_prefix("otpauth://").and_then(|r| r.split_once('?')).map(|x| x.1) {
        let mut secret = Vec::new();
        let (mut digits, mut period, mut algo) = (6u32, 30u64, "SHA1".to_string());
        for kv in q.split('&') {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            let v = hearth_oss::http::dec(v);
            match k.to_lowercase().as_str() {
                "secret" => secret = hearth_oss::b64::base32(&v),
                "digits" => digits = v.parse().unwrap_or(6),
                "period" => period = v.parse().unwrap_or(30),
                "algorithm" => algo = v.to_uppercase(),
                _ => {}
            }
        }
        return (secret, digits, period.max(1), algo);
    }
    (hearth_oss::b64::base32(s), 6, 30, "SHA1".into())
}

pub fn totp_at(secret: &[u8], t: u64, digits: u32, period: u64, algo: &str) -> String {
    let c = (t / period).to_be_bytes();
    let h: Vec<u8> = match algo {
        "SHA256" => {
            let mut m = <Hmac<sha2::Sha256> as Mac>::new_from_slice(secret).expect("any key");
            m.update(&c);
            m.finalize().into_bytes().to_vec()
        }
        "SHA512" => {
            let mut m = <Hmac<sha2::Sha512> as Mac>::new_from_slice(secret).expect("any key");
            m.update(&c);
            m.finalize().into_bytes().to_vec()
        }
        _ => {
            let mut m = <Hmac<sha1::Sha1> as Mac>::new_from_slice(secret).expect("any key");
            m.update(&c);
            m.finalize().into_bytes().to_vec()
        }
    };
    let o = (h[h.len() - 1] & 15) as usize;
    let n = u32::from_be_bytes([h[o] & 0x7f, h[o + 1], h[o + 2], h[o + 3]]) % 10u32.pow(digits.min(9));
    format!("{:0width$}", n, width = digits as usize)
}

/// The code now and the seconds it has left.
pub fn totp_now(s: &str, now: u64) -> Option<(String, u64, u64)> {
    let (secret, digits, period, algo) = otp_params(s);
    if secret.is_empty() {
        return None;
    }
    Some((totp_at(&secret, now, digits, period, &algo), period - now % period, period))
}

// ------------------------------------------------------------------ the password maker

#[derive(Debug, Clone, PartialEq)]
pub struct Maker {
    /// "password" | "words" | "pin"
    pub how: String,
    pub length: u32,
    pub upper: bool,
    pub lower: bool,
    pub digits: bool,
    pub symbols: bool,
    pub no_lookalikes: bool,
}

const WORDS: &str = include_str!("words.txt");

fn rand_below(n: u32) -> u32 {
    // rejection sampling: every choice equally likely
    let lim = u32::MAX - u32::MAX % n.max(1);
    loop {
        let mut b = [0u8; 4];
        if getrandom::getrandom(&mut b).is_err() {
            continue;
        }
        let x = u32::from_le_bytes(b);
        if x < lim {
            return x % n.max(1);
        }
    }
}

pub fn make(m: &Maker) -> String {
    match m.how.as_str() {
        "pin" => (0..m.length.clamp(4, 12)).map(|_| char::from(b'0' + rand_below(10) as u8)).collect(),
        "words" => {
            let w: Vec<&str> = WORDS.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
            let n = (m.length / 3).clamp(4, 12);
            (0..n).map(|_| w[rand_below(w.len() as u32) as usize]).collect::<Vec<_>>().join("-")
        }
        _ => {
            let mut sets: Vec<Vec<char>> = Vec::new();
            let look = |c: &char| !m.no_lookalikes || !"lI1O0o".contains(*c);
            if m.upper {
                sets.push(('A'..='Z').filter(look).collect());
            }
            if m.lower {
                sets.push(('a'..='z').filter(look).collect());
            }
            if m.digits {
                sets.push(('0'..='9').filter(look).collect());
            }
            if m.symbols {
                sets.push("!#%&*@$?".chars().collect());
            }
            if sets.is_empty() {
                sets.push(('a'..='z').collect());
            }
            let all: Vec<char> = sets.iter().flatten().cloned().collect();
            let len = m.length.clamp(8, 64) as usize;
            loop {
                let s: Vec<char> = (0..len).map(|_| all[rand_below(all.len() as u32) as usize]).collect();
                // one of each kind asked for
                if sets.iter().all(|set| s.iter().any(|c| set.contains(c))) {
                    return s.into_iter().collect();
                }
            }
        }
    }
}

/// How strong, in bits, and in words.
pub fn strength(p: &str) -> (f64, &'static str, &'static str) {
    if p.is_empty() {
        return (0.0, "empty", "");
    }
    let mut pool = 0f64;
    if p.chars().any(|c| c.is_ascii_lowercase()) {
        pool += 26.0;
    }
    if p.chars().any(|c| c.is_ascii_uppercase()) {
        pool += 26.0;
    }
    if p.chars().any(|c| c.is_ascii_digit()) {
        pool += 10.0;
    }
    if p.chars().any(|c| !c.is_ascii_alphanumeric()) {
        pool += 12.0;
    }
    let mut bits = p.chars().count() as f64 * pool.max(2.0).log2();
    if p.contains('-') && p.split('-').count() >= 3 && p.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
        bits = p.split('-').count() as f64 * (WORDS.lines().count().max(2) as f64).log2();
    }
    let (w, t) = if bits >= 100.0 {
        ("very strong", "centuries to guess")
    } else if bits >= 70.0 {
        ("strong", "years to guess")
    } else if bits >= 45.0 {
        ("fair", "days to guess")
    } else {
        ("weak", "minutes to guess")
    };
    (bits, w, t)
}

/// The security check: passwords used twice, weak ones, and sign-ins with a website that could have a 2-step code.
pub fn check(items: &[Item]) -> Vec<(&'static str, String, Vec<String>)> {
    let logins: Vec<&Item> = items.iter().filter(|i| matches!(i.kind(), Kind::Login | Kind::Wifi) && !i.password.is_empty()).collect();
    let mut out = Vec::new();
    let mut seen: Vec<usize> = Vec::new();
    for (i, a) in logins.iter().enumerate() {
        if seen.contains(&i) {
            continue;
        }
        let same: Vec<usize> = logins.iter().enumerate().filter(|(j, b)| *j != i && b.password == a.password).map(|(j, _)| j).collect();
        if !same.is_empty() {
            seen.extend(same.iter().cloned());
            let mut names = vec![a.name.clone()];
            names.extend(same.iter().map(|j| logins[*j].name.clone()));
            let words = if names.len() == 2 { format!("{} and {} share one password", names[0], names[1]) } else { format!("{} share one password", names.join(", ")) };
            out.push(("Used twice", words, logins.iter().enumerate().filter(|(j, _)| *j == i || same.contains(j)).map(|(_, x)| x.id.clone()).collect()));
        }
    }
    for a in &logins {
        let (bits, w, _) = strength(&a.password);
        if w == "weak" || bits < 45.0 {
            out.push(("Weak", format!("{} · {} characters", a.name, a.password.chars().count()), vec![a.id.clone()]));
        }
    }
    for a in &logins {
        if a.kind() == Kind::Login && a.totp.is_empty() && a.notes.to_lowercase().contains("offers 2-step") {
            out.push(("Add a 2-step code", format!("{} offers one", a.name), vec![a.id.clone()]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_rfc_6238s() {
        let k = b"12345678901234567890";
        assert_eq!(totp_at(k, 59, 8, 30, "SHA1"), "94287082");
        assert_eq!(totp_at(k, 1111111109, 8, 30, "SHA1"), "07081804");
        let (sec, d, p, a) = otp_params("otpauth://totp/GitHub:me?secret=JBSWY3DPEHPK3PXP&issuer=GitHub&digits=6&period=30");
        assert_eq!((sec.len(), d, p, a.as_str()), (10, 6, 30, "SHA1"));
        let (code, left, of) = totp_now("JBSWY3DPEHPK3PXP", 1_000_012).unwrap();
        assert_eq!((code.len(), left, of), (6, 30 - 1_000_012 % 30, 30));
    }

    #[test]
    fn the_maker_keeps_to_what_you_asked() {
        let m = Maker { how: "password".into(), length: 24, upper: true, lower: true, digits: true, symbols: true, no_lookalikes: true };
        for _ in 0..50 {
            let p = make(&m);
            assert_eq!(p.chars().count(), 24);
            assert!(p.chars().any(|c| c.is_ascii_digit()) && p.chars().any(|c| "!#%&*@$?".contains(c)));
            assert!(!p.contains(['l', 'I', '1', 'O', '0']));
        }
        assert_eq!(make(&Maker { how: "pin".into(), length: 6, ..m.clone() }).len(), 6);
        assert!(make(&Maker { how: "words".into(), ..m.clone() }).split('-').count() >= 3);
        assert_eq!(strength("r7#Vq4mLp!8eZw2Tn%kHs9Yb").1, "very strong");
        assert_eq!(strength("sunny123").1, "weak");
    }

    #[test]
    fn the_check_finds_what_the_board_shows() {
        let it = |id: &str, name: &str, pw: &str| Item { id: id.into(), kind: Some(Kind::Login), name: name.into(), password: pw.into(), ..Item::default() };
        let v = vec![it("1", "Epic", "same-pass-Abc123!"), it("2", "GOG", "same-pass-Abc123!"), it("3", "Electric company", "sunny123"), it("4", "GitHub", "r7#Vq4mLp!8eZw2Tn%kHs9Yb")];
        let c = check(&v);
        assert!(c.iter().any(|x| x.0 == "Used twice" && x.1 == "Epic and GOG share one password"));
        assert!(c.iter().any(|x| x.0 == "Weak" && x.1 == "Electric company · 8 characters"));
        assert_eq!(it("x", "Hearth Store developer", "").letters(), "HS");
        assert_eq!(it("x", "GitHub", "").letters(), "GH");
    }
}
