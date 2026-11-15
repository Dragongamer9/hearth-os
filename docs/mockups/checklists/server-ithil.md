# Server mode › Ithil (approved Nov 15, 2026)

Mockups: next-update/server-ithil-dash.png, next-update/server-ithil-detail.png
Release: 1.13

Ithil is the owner's own AI app (Dragongamer9/hearth-ai). Hearth only reads her files (hosts/linux/desktop/shell/hearth_ithil.py);
the rest of the dashboard in the first picture is the 1.12 gold dashboard as it is.

## The dashboard (server-ithil-dash)
- [x] Her tile, two tiles wide, right after the AI team — hosts/linux/desktop/shell/hearth-server-mode: "tiles.insert(1, {'id': 'ithil'"
- [x] Moon icon, "Ithil" with NEW, "Her own mind · Hearth AI 0.6" — hosts/linux/desktop/shell/hearth-server-mode: "'Her own mind' + (' · Hearth AI '"
- [x] Awake / Asleep / Not running chip — hosts/linux/desktop/shell/hearth_ithil.py: "'chip': 'Not running' if not up else 'Asleep' if asleep else 'Awake'"
- [x] A small live 3D brain lit by her real log — hosts/linux/desktop/shell/ServerTile.qml: "IthilBrain { anchors.fill: parent; compact: true"
- [x] Mood, Focused on, Energy / Social / Curiosity / Stability bars — hosts/linux/desktop/shell/ServerTile.qml: "[\"Energy\", \"energy\"], [\"Social\", \"social\"]"
- [x] "Brain loaded · answered you 2 min ago" — hosts/linux/desktop/shell/ServerTile.qml: "\" · answered you \""
- [x] "Ithil" among the service pills, counted in "N services running" — hosts/linux/desktop/shell/hearth-server-mode: "('Ithil', '', ('running' if ithil['running']"
- [x] Her sleep in the strip: "Ithil slept 02:14–02:52 · 41 moments replayed, energy .29 → .88" — hosts/linux/desktop/shell/hearth_ithil.py: "'%d moments replayed'"
- [x] Arrows work around the two-wide tile — hosts/linux/desktop/shell/ServerDash.qml: "function cellsOf()"

## Opened (server-ithil-detail)
- [x] "Ithil", "Her own mind on this PC · Hearth AI 0.6 · reach her at http://<PC>:47860 (Tailscale)" and her chip — hosts/linux/desktop/shell/IthilDetail.qml: "\" · reach her at <b>"
- [x] Her big live 3D / 4D brain (the team's, the same as her app's Mind screen) — hosts/linux/desktop/shell/ithil-brain.js: "function draw(ctx, v)"
- [x] ● Live, Last 24 hours, Her whole life — hosts/linux/desktop/shell/IthilDetail.qml: "Pill { t: \"Her whole life\""
- [x] "Awake · live" (☾ Asleep while she sleeps) — hosts/linux/desktop/shell/IthilDetail.qml: "\"☾ Asleep\" : \"Awake\""
- [x] The timeline: her events in colour, her sleeps shaded, the bar — hosts/linux/desktop/shell/IthilDetail.qml: "id: strip"
- [x] "Ⓛ Ⓡ move through time (the 4th dimension) · right stick turns her brain · Ⓥ X-ray", all working (LB / RB arrive as Alt+Tab; the pointer over her brain turns it; View toggles X-ray) — hosts/linux/desktop/shell/ServerDash.qml: "K === Qt.Key_Tab || K === Qt.Key_Backtab"
- [x] Right now: mood · focused on, the four needs — hosts/linux/desktop/shell/IthilDetail.qml: "title: \"Right now\""
- [x] Today: talks with you, memories kept, blocked by her gate, when she sleeps — hosts/linux/desktop/shell/IthilDetail.qml: "\"blocked by her gate\""
- [x] Her brain on this PC: the model, Loaded, stays loaded / steps back while you play / pausing the AI team asks first — hosts/linux/desktop/shell/IthilDetail.qml: "Pausing the AI team asks first, because her brain is in it."
- [x] Reach her from: the Command Center (and who else reached her), over Tailscale; Anything not yours: Never — hosts/linux/desktop/shell/hearth_ithil.py: "'chip': 'Never'"
- [x] Ⓐ Talk to her (her app's window; Guide brings the dashboard back) — hosts/linux/desktop/shell/ServerDash.qml: "Guide brings the dashboard back"
- [x] Ⓧ Restart her app — hosts/linux/desktop/shell/hearth_ithil.py: "def restart():"
- [x] Ⓨ Put her to sleep (asks twice) — hosts/linux/desktop/shell/ServerDash.qml: "Press Y again to put her to sleep"
- [x] ☰ Her settings — hosts/linux/desktop/shell/hearth_ithil.py: "'--screen=' + screen"
- [x] Ⓑ Back to the dashboard — hosts/linux/desktop/shell/IthilDetail.qml: "Ⓑ Back to the dashboard"

## Behind it
- [x] Her app starts in the Server mode session — hosts/linux/desktop/shell/ServerDash.qml: "\"ithil-start\""
- [x] "Pause the AI team" asks first and says her brain is in it — hosts/linux/desktop/shell/ServerDash.qml: "'s brain with it)"
- [x] Her files only read (her log opened read-only); her key never shown — hosts/linux/test/test_server_ithil.py: "test_her_log_is_opened_read_only"
