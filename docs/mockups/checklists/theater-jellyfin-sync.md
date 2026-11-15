# Theater: your Jellyfin and sync with your devices (approved Nov 15, 2026)

Mockups: next-update/theater-1-home.png, next-update/theater-2-add.png, next-update/theater-3-connect.png, next-update/theater-4-sync.png
Release: 1.13

The owner asked whether Theater syncs between devices on Tailscale and whether it can pull from Jellyfin, then
approved these four pictures. Sync goes through Hearth Link (your paired devices; the home server is one of them, so
it's what carries changes between devices that are never on together). Sign-ins never sync: a Jellyfin connects once
per device.

## Home screen with a Jellyfin (theater-1)
- [x] the Sync chip in the header: green dot, "Synced · N devices" — hosts/hearth-theater/ui/index.html: "`Synced · ${SYNC.devices.length + 1} devices`"
- [x] Continue watching from Jellyfin, with where you stopped (a bar on the poster) — hosts/hearth-theater/ui/index.html: "<span class=\"prog\"><i style=\"width:${Math.round(Math.min(1, it.progress) * 100)}%\"></i></span>"
- [x] "24 min left" / "1 h 10 min left" under the title — hosts/hearth-theater/src/jf.rs: "format!(\"{} h {} min left\", m / 60, m % 60)"
- [x] Episodes named "Show · S2 E5" — hosts/hearth-theater/src/jf.rs: "format!(\"S{s} E{e}\")"
- [x] the rows: Continue watching, Next up, Recently added, Movies, Shows — hosts/hearth-theater/src/remote.rs: "(\"Continue watching\", &a), (\"Next up\", &bb), (\"Recently added\", &c), (\"Movies\", &d), (\"Shows\", &e)"
- [x] the row's source shown beside its name (Jellyfin · server) — hosts/hearth-theater/src/remote.rs: "format!(\"Jellyfin · {server_name}\")"
- [x] Episode / Movie badges — hosts/hearth-theater/src/jf.rs: "\"Episode\" => \"episode\""
- [x] a title opens in Jellyfin's own player, where you stopped — hosts/hearth-theater/src/jf.rs: "web/index.html#/details?id={id}"

## Add a site: Your Jellyfin (theater-2)
- [x] "Your Jellyfin" at the top of Add a site, found by itself — hosts/hearth-theater/ui/index.html: "<span class=\"label\">Your Jellyfin</span>"
- [x] found on Tailscale (the home server first), the home network and this device — hosts/hearth-theater/src/jf.rs: "\"your home server, over Tailscale\""
- [x] its name, Jellyfin version and where it is — hosts/hearth-theater/src/remote.rs: "\"version\": th::jf::short_version("
- [x] Connect button (focused first) — hosts/hearth-theater/ui/index.html: "data-jf=\"${i}\" ${i === 0 ? 'data-autofocus' : ''}>Connect</button>"
- [x] "or add any video site by its link", then the usual Link, Paste, Name, Shelf, Cancel, Add — hosts/hearth-theater/ui/index.html: "<div class=\"orline\">or add any video site by its link</div>"

## Connect (theater-3)
- [x] "Connect <server>" with where to approve it (Settings › Quick Connect) — hosts/hearth-theater/ui/index.html: "<b>Settings › Quick Connect</b>, and enter"
- [x] the 6-digit code, large — hosts/hearth-theater/src/remote.rs: "QuickConnect/Initiate"
- [x] "Waiting for Jellyfin… Theater never sees your password." with a spinner — hosts/hearth-theater/ui/index.html: "Waiting for Jellyfin… Theater never sees your password."
- [x] waits for the approval, then signs in with it — hosts/hearth-theater/src/remote.rs: "Users/AuthenticateWithQuickConnect"
- [x] Shelf chips (Movies picked) — hosts/hearth-theater/ui/index.html: "${c === 'Movies' ? 'on' : ''}\" data-cat=\"${c}\""
- [x] Cancel (and B) stops it — hosts/hearth-theater/src/main.rs: "\"jellyfin_cancel\" => { remote::cancel("
- [x] the sign-in kept on this device only, readable by you only — hosts/hearth-theater/src/jf.rs: "o.mode(0o600);"

## Sync with your devices (theater-4)
- [x] "Sync your sites, favorites and Continue" switch — hosts/hearth-theater/ui/index.html: "<span>Sync your sites, favorites and Continue</span>"
- [x] "Through <server> (your home server) over Tailscale · last synced …" — hosts/hearth-theater/ui/index.html: "(your home server) over Tailscale${s.last ? ` · last synced ${esc(ago(s.last))}` : ''}"
- [x] your devices, each with when it last synced — hosts/hearth-theater/src/jf.rs: "format!(\"synced {}\", ago(now, at))"
- [x] what follows you, and that sign-ins stay on each device — hosts/hearth-theater/ui/index.html: "Sign-ins to sites stay on each device."
- [x] Sync now and Done — hosts/hearth-theater/src/remote.rs: "args([\"call\", \"sync.now\", \"{}\"])"
- [x] sites, favorites, Continue and hidden rows really go between devices (Hearth Link) — hosts/hearth-link/src/sync.rs: "g/theater/site/{id}"
- [x] turning it off on one device keeps that device's sites out of it, removing nothing elsewhere — hosts/hearth-link/src/sync.rs: "&& (theater || !k.starts_with(\"g/theater/\"))"
