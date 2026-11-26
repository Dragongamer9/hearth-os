# Hearth Passwords (Vaultwarden + KeePassXC, AGPL-3/GPL-3) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/16-passwords.html)

Mockups: open-source-apps/16-passwords.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps/passwords), made of blocks (studio: Edit layout, Customize, its
own blocks window), built on the open-source projects the board credits: Vaultwarden on the Nvidia PC (hearth-oss-server,
its own image in rootless Podman, 127.0.0.1 and Tailscale only) through the Bitwarden protocol with RustCrypto's crates,
and KeePass files through KeePassXC's own command (keepassxc-cli).

Board: Hearth Passwords (Vaultwarden + KeePassXC, AGPL-3/GPL-3): the vault synced from his own server and the password maker.
The board's top bar and wallpaper are the desktop's. Pictures: hearth-apps-rust.yml, passwords.sh (passwords-1-board …).

## Every part of the board

### The title bar
- [x] The PW letter mark and the name Hearth Passwords — hosts/hearth-ui/apps/passwords/src/main.rs: "w::mark(\"PW\", \"or\", 22.0)"
- [x] "· your vault, kept on your own server" — hosts/hearth-ui/apps/passwords/src/main.rs: "· your vault, kept on your own server"
- [x] SYNCED · NVIDIA PC · 2 MIN AGO (the copy on this PC when the server can't be reached) — hosts/hearth-ui/apps/passwords/src/main.rs: "format!(\"Synced · {} · {ago}\""
- [x] Lock (keys and opened items wiped from memory) — hosts/hearth-ui/apps/passwords/src/main.rs: "fn lock(&mut self)"
- [x] – ▢ ✕ — hosts/hearth-ui/apps/passwords/src/main.rs: "TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close }"

### The side
- [x] Everything, Favourites, Sign-ins, 2-step codes, Cards, Wi-Fi, Secure notes, each with its count — hosts/hearth-ui/apps/passwords/src/main.rs: "(\"codes\", \"2-step codes\", n(&|i| !i.totp.is_empty()))"
- [x] FOLDERS: your vault's own folders in their colours with their counts (Hearth, Games, Home, Shared with Dad) — hosts/hearth-ui/apps/passwords/src/main.rs: "w::cap(&p, \"Folders\")"
- [x] HEALTH: "● 3 used twice", "● 1 weak", "● none found in leaks" (Have I Been Pwned's range check: only 5 letters of each SHA-1 leave the PC) — hosts/hearth-ui/apps/passwords/src/main.rs: "https://api.pwnedpasswords.com/range/{}"

### The vault
- [x] ⌕ Search the vault — hosts/hearth-ui/apps/passwords/src/main.rs: "w::search(&p, \"Search the vault\", &a.q, Msg::Search)"
- [x] ＋ (in the accent): a new sign-in, encrypted and sent to your server — hosts/hearth-ui/apps/passwords/src/main.rs: "bw::send(&base, \"POST\", \"/api/ciphers\", &token, &body)?;"
- [x] "412 items" and "Sort: Name ▾" — hosts/hearth-ui/apps/passwords/src/main.rs: "if a.sort_name { \"Sort: Name ▾\" } else { \"Sort: Changed ▾\" }"
- [x] Each item: its letter mark in a colour, its name, what's under it (user, "Wi-Fi · WPA3", "card · ends 4821") — hosts/hearth-ui/apps/passwords/src/vault.rs: "Kind::Card => format!(\"card · ends {}\", self.card_last4),"
- [x] The open one lit, with ⧉ — hosts/hearth-ui/apps/passwords/src/main.rs: "r = r.push(t(\"⧉\", 12.0, p.muted));"

### The item
- [x] Its big mark, its name, "Folder Hearth · changed Nov 15 · used today" — hosts/hearth-ui/apps/passwords/src/main.rs: "meta.push(\"used today\".into());"
- [x] ☆ Favourite (sent to your server) — hosts/hearth-ui/apps/passwords/src/main.rs: "format!(\"/api/ciphers/{}/partial\", i.id)"
- [x] Edit: every field changed in place, encrypted, sent back (the old password into its history) — hosts/hearth-ui/apps/passwords/src/main.rs: "body[\"passwordHistory\"] = Value::Array(h);"
- [x] ⋯ (Make a password for it, Delete to the bin) — hosts/hearth-ui/apps/passwords/src/main.rs: "(\"Delete (to the bin)\".into(), false, Msg::Delete)"
- [x] USER NAME with ⧉ — hosts/hearth-ui/apps/passwords/src/main.rs: "fl(&p, \"User name\""
- [x] PASSWORD as dots with ◉ (show) and ⧉ (copied for 30 s), its strength bar, "very strong · 24 characters" — hosts/hearth-ui/apps/passwords/src/main.rs: "format!(\"{} characters\", pw.chars().count())"
- [x] 2-STEP CODE "482 913" in the accent, its ring and "18 s", ⧉ (RFC 6238, from the item's own secret) — hosts/hearth-ui/apps/passwords/src/vault.rs: "pub fn totp_at(secret: &[u8], t: u64, digits: u32, period: u64, algo: &str) -> String"
- [x] WEBSITE github.com with ↗ — hosts/hearth-ui/apps/passwords/src/main.rs: "fl(&p, \"Website\""
- [x] NOTES — hosts/hearth-ui/apps/passwords/src/main.rs: "tb(\"NOTES\", 11.0, p.muted)"
- [x] Filling it in: "Fill in by itself in browsers" (off), "Ask before filling" (on), "Also for apps (Hearth AI, Store)" (on), kept — hosts/hearth-ui/apps/passwords/src/main.rs: "w::kv(&p, \"Also for apps (Hearth AI, Store)\""
- [ ] Filling in browsers and apps itself: the three choices are kept, but Hearth Passwords can't type into other programs' pages yet (Vaultwarden works with Bitwarden's own browser extension pointed at your server; a Hearth way for browsers and Hearth's apps to ask the vault is still to be made)
- [x] History: "Password changed · Nov 15", "Code added · Oct 2", "Made · Aug 19" (the server's history and dates; a code added here is noted here) — hosts/hearth-ui/apps/passwords/src/main.rs: "hist.push(format!(\"Made · {}\", day_of(&i.made)));"

### Where your vault is
- [x] Where your vault is, and Back up now (sqlite's own backup of the vault's database on the server; elsewhere a copy of the encrypted vault) — hosts/linux/desktop/shell/hearth-oss-server: "def backup(name):"
- [x] Your server: "Nvidia PC · reached over Tailscale · the main copy" — hosts/hearth-ui/apps/passwords/src/main.rs: "format!(\"{} · {how} · the main copy\""
- [x] This laptop: "Locked copy, works with no internet" (the server's own encrypted copy; opens offline) — hosts/hearth-ui/apps/passwords/src/main.rs: "\"Locked copy, works with no internet\""
- [x] Phone and Ally: "Synced · open with fingerprint or PIN" (your other devices, from your server's own list) — hosts/hearth-ui/apps/passwords/src/main.rs: "\"/api/devices\""

### Security check
- [x] Security check, "4 things worth fixing · checked tonight", Fix them one by one (in the accent: each opened with the password maker) — hosts/hearth-ui/apps/passwords/src/main.rs: "\"Fix them one by one\""
- [x] Used twice: "Epic and GOG share one password", "Router and Home Wi-Fi" — hosts/hearth-ui/apps/passwords/src/vault.rs: "format!(\"{} and {} share one password\", names[0], names[1])"
- [x] Weak: "Electric company · 8 characters" — hosts/hearth-ui/apps/passwords/src/vault.rs: "format!(\"{} · {} characters\", a.name, a.password.chars().count())"
- [x] Add a 2-step code: "Truck insurance offers one" — hosts/hearth-ui/apps/passwords/src/vault.rs: "format!(\"{} offers one\", a.name)"

### Make a password
- [x] Its card over the item, its mark, "Make a password" and ✕ — hosts/hearth-ui/apps/passwords/src/main.rs: "tb(\"Make a password\", 14.0, p.text)"
- [x] The password in monospace, digits blue, signs in the accent — hosts/hearth-ui/apps/passwords/src/main.rs: "let c = if ch.is_ascii_digit() { w::blue() } else if ch.is_ascii_alphanumeric() { p.text } else { p.accent };"
- [x] Its strength bar, "very strong · centuries to guess", ↻ Again — hosts/hearth-ui/apps/passwords/src/vault.rs: "(\"very strong\", \"centuries to guess\")"
- [x] Password / Words / PIN — hosts/hearth-ui/apps/passwords/src/main.rs: "vec![how(\"password\", \"Password\"), how(\"words\", \"Words\"), how(\"pin\", \"PIN\")]"
- [x] Length slider and 24 — hosts/hearth-ui/apps/passwords/src/main.rs: "slider(8.0..=64.0, m.length as f32, Msg::MakerLen)"
- [x] A–Z, a–z, 0–9, ! # % & *, Leave out look-alikes (l, 1, O, 0) — hosts/hearth-ui/apps/passwords/src/main.rs: "w::kv(&p, \"Leave out look-alikes (l, 1, O, 0)\""
- [x] Copy, Cancel, "Use it for GitHub" (in the accent: the item's password changed on your server, or in the KeePass file) — hosts/hearth-ui/apps/passwords/src/main.rs: "format!(\"Use it for {name}\")"

### The credit, the blocks, the server
- [x] "Hearth Passwords · Built on Vaultwarden (AGPL-3) and KeePassXC (GPL-3)" — hosts/hearth-ui/apps/passwords/src/main.rs: "\"Built on Vaultwarden (AGPL-3) and KeePassXC (GPL-3)\""
- [x] Every part a block (4 pages, 15 blocks), Edit layout in the side, its own blocks window (12 buttons, 3 panes, 4 info) — hosts/hearth-ui/apps/passwords/src/main.rs: "name: \"Passwords\","
- [x] Real encryption from well-known crates (PBKDF2 / Argon2id, HKDF, AES-256-CBC + HMAC-SHA256, RSA-OAEP), keys wiped when locked, the master password never kept — hosts/hearth-ui/apps/passwords/src/bw.rs: "m.verify_slice(&mac)"
- [x] KeePass files opened and changed through KeePassXC's own command — hosts/hearth-ui/apps/passwords/src/kp.rs: "\"export\".into(), \"-q\".into(), \"--format\".into(), \"xml\".into()"
- [x] Vaultwarden installed and turned on the Hearth way when it isn't there (its image in your Podman, its user service guarded so nothing else waits for it) — os/system_files/usr/lib/systemd/user/hearth-vaultwarden.service: "ConditionPathExists=%h/.local/state/nimbus/oss-server/vaultwarden.ready"
