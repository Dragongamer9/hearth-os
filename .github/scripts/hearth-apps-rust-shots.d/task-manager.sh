# Task Manager's extra pictures (sourced by hearth-apps-rust-test.sh inside its headless screen; run() is its).
# Performance's settings tabs (mockups tm-3 … tm-7, task-manager-memory, task-manager-swap-settings), Details ›
# Columns… (tm-2) and Scheduled tasks › + New task (tm-8).
#
# CI has no smartctl, NetworkManager, LACT, battery or graphics card, so most pictures read test answers instead of
# the helpers: HEARTH_TM_FIXTURES=<folder> (never set on a PC) makes Task Manager read <folder>/<what>.json for each
# helper's answer (hw = hearth-hwset status, mem = hearth-memory status, …) and devices.json adds a graphics card, a
# battery, Wi-Fi, Tailscale and some programs. One picture (tm-20) reads the real helpers, to show the plain states.
# The rail is folded for these, as in the mockups (their own config folder, so nothing of the person's is touched).
FIX=/tmp/tm-fix
mkdir -p "$FIX" /tmp/tm-cfg/nimbus
echo '{"railFolded": true}' > /tmp/tm-cfg/nimbus/taskmgr.json
G=1073741824
# two test icons (a PNG and an SVG) for the Name column
python3 - "$FIX/icon-a.png" <<'PY'
import struct, sys, zlib
w = h = 48
rows = b''.join(b'\0' + b''.join(bytes((255, 120, 40, 255)) if (x - 24) ** 2 + (y - 24) ** 2 < 400 else bytes((0, 0, 0, 0)) for x in range(w)) for y in range(h))
chunk = lambda t, d: struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
open(sys.argv[1], 'wb').write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b''))
PY
echo '<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48"><rect x="4" y="4" width="40" height="40" rx="10" fill="#3d7bf0"/></svg>' > "$FIX/icon-b.svg"
cat > "$FIX/devices.json" <<JSON
{"gpus": [{"name": "NVIDIA GeForce RTX 3060", "vendor": "NVIDIA", "busy": 62, "vramUsed": $((58 * G / 10)), "vramTotal": $((12 * G)), "temp": 66}],
 "battery": {"percent": 82, "status": "Discharging", "watts": 7.9, "fullWh": 43, "designWh": 47, "cycles": 212, "hours": 4.17},
 "nets": [{"name": "wlan0", "kind": "Wi-Fi", "speed": 866, "recv": 112500, "send": 5000}, {"name": "tailscale0", "kind": "Tailscale"}],
 "groups": [
  {"name": "Hearth AI Command Center", "pid": 320684, "kind": "app", "iconPath": "$FIX/icon-a.png", "cpu": 8.4, "mem": 1288490188, "shared": 188743680, "swap": 325058560},
  {"name": "Firefox", "pid": 383590, "kind": "app", "iconPath": "$FIX/icon-b.svg", "cpu": 7.2, "mem": 859832320, "shared": 220200960, "swap": 494927872, "gpu": 2, "vram": 440401920, "uses": ["Video decode"]},
  {"name": "Universe at War", "pid": 51240, "kind": "game", "tag": "Game · Proton", "mem": 671088640, "gpu": 58, "vram": 5261334937, "uses": ["3D"]},
  {"name": "Desktop (Mantel shell)", "pid": 1806, "kind": "system", "mem": 450887680, "shared": 62914560, "swap": 41943040, "gpu": 2, "vram": 325058560, "uses": ["3D"]},
  {"name": "Steam", "pid": 4410, "kind": "app", "mem": 325058560, "shared": 94371840, "swap": 125829120}]}
JSON
cat > "$FIX/hw.json" <<'JSON'
{"gameCores": {"can": true, "value": "4-7", "threads": 8, "suggested": "4-7"}, "turbo": {"can": true, "on": true}, "maxPct": {"can": true, "value": 100},
 "throttled": 0, "throttledToday": 0, "onAC": false, "tempLimit": 100, "hibernate": "works", "cpuModel": "Intel Core i3-N305",
 "home": {"type": "btrfs", "commit": "30", "compress": "on"}, "c": {"free": 96000000000, "size": 238000000000},
 "batteries": [{"name": "BAT0", "health": 91, "fullWh": 43, "designWh": 47, "cycles": 212, "capacity": 82, "status": "Discharging", "limit": 80, "canLimit": true, "model": "Lenovo 82XB"}],
 "saved": {"power.plugged": "performance", "apps.cpu": {"Hearth AI Command Center": "50"}, "updates.priority": "low", "disk.scrub": "monthly", "disk.warnPct": "90",
           "disk.commit": "10", "disk.compress": "on", "disk.snapshots": "hourly", "battery.warn": "10,5", "battery.at5": "hibernate", "battery.saver": "20", "battery.dim": "on"}}
JSON
echo '{"mode": {"active": "balanced"}}' > "$FIX/power.json"
echo '{"drives": [{"name": "", "trim": true, "what": ""}], "timer": {"on": true, "last": "Mon 2026-11-10 03:00:01 CST"}}' > "$FIX/drives.json"
H='{"ok": true, "model": "SK hynix BC711", "lifeLeft": 97, "written": 14200000000000, "temp": 41, "hours": 1412}'
echo "{\"drives\": {\"sda\": $H, \"vda\": $H, \"nvme0n1\": $H, \"xvda\": $H, \"sdb\": $H}, \"scrub\": {\"last\": \"Nov 1\", \"ok\": true}, \"snapshots\": 24}" > "$FIX/health.json"
cat > "$FIX/space.json" <<'JSON'
{"rows": [{"id": "games", "label": "Games", "bytes": 61000000000}, {"id": "versions", "label": "Hearth versions kept (to go back)", "bytes": 22000000000},
          {"id": "swap", "label": "Swap file (hibernation)", "bytes": 36000000000}, {"id": "restore", "label": "Restore points", "bytes": 6000000000, "snapshots": 0},
          {"id": "temp", "label": "Temporary files & caches", "bytes": 3100000000, "clean": true}]}
JSON
echo '{"rows": [{"name": "Steam (updating)", "pid": 4410, "read": 2100000, "write": 18000000}, {"name": "Firefox", "pid": 383590, "read": 100000, "write": 400000}, {"name": "zram ↔ drive swap", "pid": 0, "read": 0, "write": 0}]}' > "$FIX/busy.json"
echo '{"name": "Home-5G", "metered": "no", "hotspot": 0}' > "$FIX/net.json"
echo '{"ping": 18, "jitter": 3, "today": 3400000000, "ips": ["192.168.1.42"], "tailscale": "100.84.2.7", "dns": {"choice": "auto", "servers": []}, "powersave": false, "wifi": {"ssid": "Home-5G", "standard": "Wi-Fi 6", "mbps": 866, "bars": 4}}' > "$FIX/netinfo.json"
cat > "$FIX/netapps.json" <<'JSON'
{"apps": [{"name": "Steam", "pid": 4410, "down": 2400000, "up": 40000, "today": 2100000000, "hosts": ["steamcontent.com"], "allow": "limit:5"},
          {"name": "Firefox", "pid": 383590, "down": 90000, "up": 12000, "today": 820000000, "hosts": ["youtube.com", "github.com", "a", "b", "c", "d", "e", "f"], "allow": "allow"},
          {"name": "Hearth AI Command Center", "pid": 320684, "down": 20000, "up": 8000, "today": 210000000, "hosts": ["api.anthropic.com", "chat.groq.com"], "allow": "allow"},
          {"name": "Universe at War", "pid": 51240, "down": 0, "up": 0, "today": 0, "hosts": [], "allow": "block"}]}
JSON
echo '{"Peer": {"a": {}, "b": {}, "c": {}}}' > "$FIX/ts.json"
echo '{"installed": true, "lact": true, "curve": "30:20,50:30,65:45,75:65,90:100", "fanMode": "curve", "powerMin": 100, "powerMax": 170, "powerCap": 145, "canUndervolt": true, "undervolt": -75, "profile": "balanced", "power": 121, "fanPct": 48, "fanRpm": 1620, "fps": 71}' > "$FIX/gpu.json"
echo '{"rows": [{"name": "Firefox", "share": 31, "watts": 2.4, "tip": "a video tab in the background"}, {"name": "Hearth AI Command Center", "share": 22, "watts": 1.7, "tip": "Efficiency mode on battery?"}, {"name": "Screen", "screen": true, "share": 28, "watts": 2.2, "tip": "brightness 70%"}]}' > "$FIX/batt.json"
echo '{"slots": 2, "used": 2, "speed": 4800, "type": "DDR5"}' > "$FIX/board.json"
cat > "$FIX/mem.json" <<JSON
{"settings": {"zram": {"on": true, "gb": 12, "algo": "zstd"}, "keep": {"mode": "50%"}, "swappiness": 200, "swapfile": {"everyday": true},
              "guard": {"level": "gentle", "never": ["desktop", "steam", "game", "aicc"], "extra": [], "checkpoint": []}},
 "limits": {"zramMaxGb": 16, "zramMarkGb": 8, "swapfileMaxGb": 64}, "algos": ["zstd", "lz4", "lzo-rle"],
 "zram": {"active": true, "used": $((31 * G / 10)), "disksize": $((12 * G)), "ratio": 3},
 "swapfile": {"exists": true, "gb": 36, "path": "/var/swap/swapfile", "hibernate": true}, "swaps": [{"path": "/var/swap/swapfile", "used": $((2 * G / 10))}],
 "pressure": {"word": "none"}, "watermarks": {"low": $((2 * G))}, "memory": {"total": $((75 * G / 10))}, "recommended": {"zram": {"gb": 8}},
 "neverLabels": {"desktop": "Desktop", "steam": "Steam", "game": "The game you're playing", "aicc": "AI Command Center"},
 "hibernateChecked": 1762790400, "lastClosed": [], "testFiles": {}, "overridden": {}, "vm": {}}
JSON
# the disk's own name for drives.json's TRIM line
D=$(ls /sys/block | grep -Ev '^(loop|ram|zram|dm-|sr|md)' | head -n1)
sed -i "s/\"name\": \"\"/\"name\": \"$D\"/" "$FIX/drives.json"

export XDG_CONFIG_HOME=/tmp/tm-cfg HEARTH_TM_FIXTURES="$FIX"
run tm-11-cpu-power-settings hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:cpu --tab settings
run tm-12-disk-health-settings hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:disk --tab settings
run tm-13-net-apps-settings hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:net:wlan0 --tab settings
run tm-14-gpu-tuning hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:gpu:0 --tab settings
run tm-15-battery-health-settings hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:battery --tab settings
run tm-16-memory-overview hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:memory
run tm-17-memory-swap-settings hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:memory --tab settings
run tm-21-processes-icons hearth-tasks 0 "$BIN/hearth-task-manager" --page processes
unset HEARTH_TM_FIXTURES
run tm-18-details-columns hearth-tasks 0 "$BIN/hearth-task-manager" --page details --show columns
run tm-19-new-task hearth-tasks 0 "$BIN/hearth-task-manager" --page tasks --show new-task
run tm-20-cpu-settings-real hearth-tasks 0 "$BIN/hearth-task-manager" --page performance:cpu --tab settings
unset XDG_CONFIG_HOME
