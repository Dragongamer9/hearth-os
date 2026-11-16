# Server mode › Quiet hours and "When this, do that" rules (approved Nov 15, 2026)

Mockups: next-update/server-rules.png
Release: 1.13

- [x] Its tile on the dashboard, and A opens this view — hosts/linux/desktop/shell/hearth-server-mode: "return {'id': 'rules', 'title': 'Quiet hours & rules'"
- [x] "Quiet hours & rules", "Your times and your rules for this server · nothing here is on until you make it", "Quiet now" chip — hosts/linux/desktop/shell/RulesDetail.qml: "Your times and your rules for this server · nothing here is on until you make it"
- [x] Quiet hours: each with its name, times and days, and its switch — hosts/linux/desktop/shell/RulesDetail.qml: "modelData.from + \" – \" + modelData.to + \" · \" + modelData.daysText"
- [x] While quiet: fans capped, power capped, pictures and dubs wait, downloads at half speed — os/system_files/usr/libexec/nimbus/hearth-server-rules: "out['downloads'] = out['downloads'] or bool(w['downloads'])"
- [x] Games you play and things you start yourself always run at full speed — os/system_files/usr/libexec/nimbus/hearth-server-rules: "if not f['game']:            # games you play always run at full speed"
- [x] Now: quiet until when, jobs waiting, fan speed — hosts/linux/desktop/shell/RulesDetail.qml: "\"Quiet until <b>\""
- [x] When this, do that: each rule as "When … → …" with its switch, and Paused on a paused rule — hosts/linux/desktop/shell/RulesDetail.qml: "chip: modelData.paused ? \"Paused\" : \"\""
- [x] New rule: pick a When, then a Do — hosts/linux/desktop/shell/RulesDetail.qml: "+  New rule: pick a When, then a Do"
- [x] What a rule can watch (Jellyfin watching, a game running, a device connected, a time, a drive's heat, power use) and do (AI pictures wait, voices wait, downloads slower, backups wait, fans capped, tell my phone) — os/system_files/usr/libexec/nimbus/hearth-server-rules: "def when_true(w, f, now):"
- [x] Ⓐ Turn on or off, Ⓧ Edit, Ⓨ Pause this rule, ☰ New rule, Ⓑ Back — hosts/linux/desktop/shell/RulesDetail.qml: "[\"Y\", \"Pause this rule\", \"pause\"]"
- [x] Every rule off until you make it; each can be paused — os/system_files/usr/libexec/nimbus/hearth-server-rules: "'on': False, 'pausedUntil': 0"
