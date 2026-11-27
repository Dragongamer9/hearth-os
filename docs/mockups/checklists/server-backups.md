# Backups for every device (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-server-expansion)

Mockups: server-expansion/06-backups.png
Release: next

The owner, Nov 26: "Ok we're gonna expand heavily on the Server side of hearth OS", "Let's do it", then on the boards
"thoes look good". Every part installs and updates through the Hearth Store ("All this has to be updated thru the
store"), never as a Hearth OS update; anything needing system pieces installs itself with the admin password once
("Debian system files are unlocked"). Made of blocks.

## Every part of the board
- [x] The window: the HS mark, "Home server helper", the page after it, the tags on the right, the window buttons — hosts/hearth-ui/apps/home-server/src/main.rs: "hearth_oss::w::mark(\"HS\", \"or\", 22.0), \"Home server helper\""
- [x] Sidebar: "Search the server" (it filters the rows) — hosts/hearth-ui/apps/home-server/src/main.rs: "\"Search the server\""
- [x] Sidebar: HOME SERVER, then Server home and every part from one list (one line each), the open one lit, each with its count — hosts/hearth-ui/apps/home-server/src/main.rs: "home::count(&a.home, pt.id)"
- [x] Sidebar counts: Dashboard "12 ok", Model vault "1.25 TB", Backups "4 devices" — os/system_files/usr/libexec/nimbus/hearth-server-parts/dash.py: "'count': '%d ok' % s['healthy']"
- [ ] Sidebar rows for Game servers, Game cache, Remote play, Media and PCs (feature/server-2 adds their lines to hearth_server_parts::parts)
- [x] THIS SERVER: the dot, "Nvidia PC · RTX 5070 12 GB", "Hearth OS · up 9 days", "Reached over Tailscale only" — os/system_files/usr/libexec/nimbus/hearth-server-parts/dash.py: "'Reached over Tailscale only' if ts else 'At home only'"
- [x] "● Updates in the Hearth Store" and "every part is its own Store app · never a Hearth OS update" — hosts/hearth-ui/apps/home-server/src/main.rs: "\"every part is its own Store app · never a Hearth OS update\""
- [x] "Made of blocks · Edit layout" at the bottom of the sidebar — hosts/hearth-ui/apps/home-server/src/main.rs: "t(\"Made of blocks · \", 12.0, p.muted)"
- [x] the title bar: "Home server helper · Backups" and "ALL 4 BACKED UP LAST NIGHT" — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "('All %d backed up last night' % len(cards))"
- [x] Backups, "Every device backs up to the server each night over Tailscale, wherever it is · only what changed is sent" — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "\"Every device backs up to the server each night over Tailscale, wherever it is · only what changed is sent\""
- [x] Back up all now, ＋ Add a device (in the accent) — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "\"＋ Add a device\""
- [x] Add a device: its name and kind, its code (shown once), its key kept in the password store — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "'backup-rest-%s' % user, rest_pw"
- [x] Each device's card: its outline (a handheld, a laptop, a PC), its name, Hearth OS · where it is (at home, at work · over 5G hotspot, at Dad's), its dot — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "fn shape<"
- [x] Last: tonight 2:04 AM · 1.8 GB sent; Whole device: 212 GB · 41 versions — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "'whole': '%s · %d versions'"
- [x] The last 14 nights: green backed up, orange tried again later — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "outl.append({'night': key, 'tone': 'late' if late else 'ok'})"
- [x] "Last 14 nights · green backed up · orange tried again later (the truck laptop was off on Nov 15; Dad's PC was asleep on Nov 20)" — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "'%s was %s on %s'"
- [x] The picked device's card lit — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "set(\"backupPick\", json!(s(x, \"id\")))"
- [x] ROG Ally (the picked one) and Back up now — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "w::chip(&p, \"Back up now\", false, Sz::Sm"
- [x] WHAT'S BACKED UP: Your Home (documents, pictures, settings) 38 GB, Game saves (Steam, Hearth Saves, emulators) 4.2 GB, Hearth's settings and apps list 0.3 GB, The whole device (to put it all back) 212 GB, Installed games (the Game cache has them, off), each with its switch — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "what(\"Installed games\", \"games\", \"the Game cache has them\".into())"
- [x] Each part's size really from the device (it measures them and writes them in its backup) (needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "tags = tags + ['size-%s=%d' % (k, n) for k, n in part_sizes(plan, conf['owner']).items()]"
- [x] WHEN: Every night 2:00 AM, "plugged in, or 50% battery"; Also when it comes home after a day away — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "\"Also when it comes home after a day away\""
- [x] It really waits for power and comes home after a day away (the device's hourly look) (needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "return now >= today_at or (plan.get('away') and now - last > 24 * 3600)"
- [x] KEPT HOW LONG: Every night for 7 days / 30 days / 90 days; Then one a week for 3 months / 1 year / Forever — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "(\"Forever\", weeks == \"forever\", \"weeks\", \"forever\".into())"
- [x] Kept by those rules on the server (restic forget: nights, then weeks) — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "'--keep-daily', str(int(d.get('nights') or 30)), '--keep-weekly', KEEP_WEEKS.get(d.get('weeks'), '52')"
- [x] Locked so it can't be changed or deleted from the Ally (the receiver takes new backups only: rclone --append-only) — os/system_files/usr/libexec/nimbus/hearth-home-server-helper: "text.replace('/usr/bin/rclone serve restic ', '/usr/bin/rclone serve restic --append-only ', 1)"
- [x] Encrypted · only you have the key (ON) — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "draw::lr(t(\"Encrypted · only you have the key\", 12.0, p.text), w::tag(&p, \"On\", Tg::Ok))"
- [x] Each device backs up to the server nightly over Tailscale: joining it (password once) puts its code in a root-only file and its timer in the system (needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "C._atomic(DEVICE_CONF, json.dumps(conf) + '\\n', 0o600)"
- [x] Restore with A file or folder / The whole device / From the Recovery USB — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "(\"From the Recovery USB\".into(), mode == \"usb\""
- [x] The search, the device ▾ and the night ▾ — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "w::field(&p, format!(\"{name} ▾\"), 0.0, None)"
- [x] Pick a night · 41 versions, and Tonight, Nov 25 … Nov 15 (week), Nov 8 (week) — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "label = C.day(x['at']) + ' (week)'"
- [x] The night's files: Saves/Gravesleep (3 files · changed Nov 25), slot1.save 12 MB · 11:52 PM DIFFERENT NOW, slot2.save, settings.cfg (DIFFERENT NOW: it differs in the newest backup) — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "'different': bool(now_x and now_x.get('mtime') != x.get('mtime'))"
- [x] Put back slot1.save, Put it beside the new one, Open — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "\"Put it beside the new one\""
- [x] Putting back never goes over anything: on the server into Home › Restored from backups (and to the device over Taildrop); on the device in place with the one there kept beside it (needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "os.rename(real, '%s (before %s)%s' % (base, stamp, ext))"
- [x] A device puts back only its own files (inside its Home) — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "raise Fail('Only your own files are put back from here')"
- [x] THE WHOLE DEVICE: "Put the ROG Ally back the way it was on a night you pick: Hearth OS, apps, settings and files. On the Ally: Settings › Updates & recovery › Restore from the server, or start it from the Recovery USB and pick "Restore from my home server" (it finds the server over Wi-Fi or Tailscale)." — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "\" and pick \\\"Restore from my home server\\\" (it finds the server over Wi-Fi or Tailscale).\""
- [x] Restore the whole device from the server (your Home, Hearth's settings and lists, /etc/hearth; never Hearth OS or the boot chain, so it always starts) (needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "def device_restore(snap):"
- [x] Make a Recovery USB, Restore to a new device (the device's code again), about 25 min at home — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "\"Restore to a new device\""
- [ ] Restoring from the Recovery USB ("Restore from my home server" in Recovery): needs the next Recovery ISO (the owner's go-ahead)
- [x] LAST NIGHT, STEP BY STEP: 5.6 GB sent in all · 34 min · Full log, and 2:00 server's 9 services, 2:04 ROG Ally · 1.8 GB, 2:11 Laptop, 2:19 Truck laptop, 2:36 Dad's PC, 3:10 copy to R2 · Sunday — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "'%s sent in all · %s'"
- [x] The server's own services backed up at 2:00 — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "['backup', '--json', '--tag', 'hearth-server', '--exclude-caches']"
- [x] Space for backups: Backup drive · 4 TB, its bar, "1.9 TB used by 4 devices and 9 services" — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "'%s used by %d devices and %s'"
- [x] BY DEVICE · ALL VERSIONS: each device and the server's own services with its bar and size — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "cap(&p, \"By device · all versions\")"
- [x] A SECOND COPY AWAY FROM HOME: Copy backups to Cloudflare R2 and "Infrequent Access · encrypted · 1.9 TB ≈ $19 / month · weekly" — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "'Infrequent Access · encrypted · %s ≈ $%d / month · weekly'"
- [x] The copy really goes to R2 on Sundays (restic's own copy, still encrypted) (needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "dest = 's3:https://%s.r2.cloudflarestorage.com/%s/backups/%s'"
- [x] PHONE AND XBOX: "The phone's photos and files go to Hearth Cloud. The Xbox keeps its saves in its own cloud: nothing to do here." — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "\"The phone's photos and files go to Hearth Cloud. The Xbox keeps its saves in its own cloud: nothing to do here.\""
- [x] CHECKS: Test a restore each month and "Nov 1: put back 50 random files from each device, all matched" — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "'%s: put back %d random files from each device, %s'"
- [x] The restore test really puts back 50 random files from each device and checks them (needs testing on a device) — os/system_files/usr/libexec/nimbus/hearth-server-parts/backups.py: "pick = random.sample(files, min(n, len(files)))"
- [x] The Backups app on each device, Updates in the Hearth Store, Install it · asks for your password once — hosts/hearth-ui/apps/home-server/src/pages/backups.rs: "\"Install it · asks for your password once\""
