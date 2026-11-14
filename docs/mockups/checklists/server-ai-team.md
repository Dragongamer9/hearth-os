# Server mode › AI team (approved Nov 13, 2026)

Mockups: next-update/server-ai-team-dash.png, next-update/server-ai-team-detail.png
Release: 1.12

The tile on Server mode's dashboard and the view it opens (A). The rest of the dashboard in the first picture is the 1.11
gold dashboard as it is.

## The tile (server-ai-team-dash)
- [x] An AI team tile, first among the services — hosts/linux/desktop/shell/hearth-server-mode: "tiles.insert(0, {'id': 'team', 'title': 'AI team'"
- [x] NEW beside its name — hosts/linux/desktop/shell/ServerTile.qml: "text: \"NEW\"; color: tile.cAcc"
- [x] "One AI address · 4 engines" under its name — hosts/linux/desktop/shell/hearth-server-mode: "'engine': 'One AI address · %d engines'"
- [x] A Busy / Ready / Off chip (Paused when you paused it) — hosts/linux/desktop/shell/hearth-server-mode: "'chip': 'Paused' if 'team' in mine else team['chip']"
- [x] The small live mesh inside the tile — hosts/linux/desktop/shell/ServerTile.qml: "AiMesh { anchors.fill: parent; anchors.margins: 4 * tile.k; team: tile.team; k: tile.k; compact: true"
- [x] The AI team's latest in the strip along the bottom ("Claude (Command Center) asked for … · answered by qwen3") — hosts/linux/desktop/shell/hearth-server-mode: "alerts.insert(0, {'level': 'team', 'text': team['event']"

## The mesh (both pictures)
- [x] The engines in a capsule in the middle: AI chat, Pictures, Voices, Speech to text, each with its icon, name and what it's doing — hosts/linux/desktop/shell/AiMesh.qml: "id: capsule"
- [x] AI chat's model and how many are asking ("qwen3:30b · 2 asking"), Pictures "making 1 of 3", Voices "Voice Studio", Speech to text "whisper · ready" — hosts/linux/desktop/shell/hearth-server-mode: "'status': ('%s · %d asking' % (loaded[0], asking)"
- [x] Every device and Command Center AI that sent work today around the engines, with today's count and what it last did ("Work laptop 15 · Asked qwen3:30b-a3b") — hosts/linux/desktop/shell/hearth-ai-share: "def team_note("
- [x] Devices connected now with nothing asked yet, dimmed (Living room TV · Jellyfin) — hosts/linux/desktop/shell/hearth-server-mode: "connected now but nothing asked today yet"
- [x] The pictures queue as a sender ("Making picture 1 of 3") — hosts/linux/desktop/shell/hearth-server-mode: "'Making picture 1 of %d'"
- [x] The web page as a sender ("opened the dashboard") — hosts/linux/desktop/shell/hearth-server-mode: "'last': 'opened the dashboard'"
- [x] Command Center AIs by name (Claude, Codex) with "Command Center" — hosts/linux/desktop/shell/hearth-ai-share: "return u[:1].upper() + u[1:], 'Command Center'"
- [x] A sender that just sent something lit (white for chat, pink for pictures and voices) with its line bright — hosts/linux/desktop/shell/AiMesh.qml: "(x.kind === \"pictures\" || x.kind === \"voice\") ? cPink : cChat"
- [x] A pulse running down a lit sender's line to the engines — hosts/linux/desktop/shell/AiMesh.qml: "NumberAnimation on t { running: modelData.active"
- [x] The faint grid and the warm glow behind the engines — hosts/linux/desktop/shell/AiMesh.qml: "A faint grid behind it all"

## Opened (server-ai-team-detail)
- [x] "AI team", "This PC's AI, for all your devices · One AI address: http://<PC>:8189/v1" and its chip — hosts/linux/desktop/shell/AiTeamDetail.qml: "This PC's AI, for all your devices · One AI address: "
- [x] Today: Requests, Answered here, Sent on to another device (how many, to which, for what), Busiest — hosts/linux/desktop/shell/AiTeamDetail.qml: "a: \"Sent on to another device\""
- [x] Engines: AI chat · model loaded, Pictures · ComfyUI 1 of 3, Voices · Voice Studio speaking, Speech to text · whisper ready — hosts/linux/desktop/shell/AiTeamDetail.qml: "title: \"Engines\""
- [x] While you play: AI chat small model, Pictures stop, keep the queue, Other devices take over — hosts/linux/desktop/shell/hearth-server-mode: "['Other devices', 'take over']"
- [x] Pause the team (A), Restart (X), Settings (Y) and "B Back to the dashboard" — hosts/linux/desktop/shell/AiTeamDetail.qml: "label: det.paused ? \"Resume the team\" : \"Pause the team\"; keyName: \"A\""
- [x] Pause the team pauses AI chat, Pictures and Voices together, and Resume brings them back — hosts/linux/desktop/shell/hearth-server-mode: "if name == 'team':"
- [x] Settings opens Settings › Home server › AI devices — hosts/linux/desktop/shell/ServerDash.qml: "dash.settings.open(\"hs-aidevices\")"
