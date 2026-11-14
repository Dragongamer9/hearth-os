# Task Manager (approved Nov 11–12, 2026)

Mockups: next-update/tm-1-processes-with-pid.png, next-update/tm-2-details.png, next-update/tm-3-cpu-power-settings.png, next-update/tm-4-disk-health-settings.png, next-update/tm-5-network-apps-settings.png, next-update/tm-6-gpu-tuning.png, next-update/tm-7-battery-health-settings.png, next-update/tm-8-startup-services-tasks.png, next-update/task-manager-memory.png, next-update/task-manager-swap-settings.png
Release: 1.12

Started in 1.11 with parts left out; finished in 1.12. Every part of every picture, as shown.

## tm-1 Processes
- [x] PID column — hosts/linux/desktop/shell/TaskManagerView.qml: "title: \"PID\""
- [x] Compressed column — hosts/linux/desktop/shell/TaskManagerView.qml: "title: \"Compressed\""
- [x] Power column — hosts/linux/desktop/shell/TaskManagerView.qml: "title: \"Power\""
- [x] Apps, Hearth and Background processes sections — hosts/linux/desktop/shell/TaskManagerView.qml: "label: \"Hearth (\""
- [x] Right-click: End task, End process tree, Efficiency mode, Set priority — hosts/linux/desktop/shell/TaskManagerView.qml: "label: \"End process tree\""
- [x] Right-click: Run on these cores… — hosts/linux/desktop/shell/TaskManagerView.qml: "label: \"Run on these cores…\""
- [x] Right-click: Limit CPU / memory… — hosts/linux/desktop/shell/TaskManagerView.qml: "label: \"Limit CPU / memory…\""
- [x] Right-click: Never close when memory runs out — hosts/linux/desktop/shell/TaskManagerView.qml: "label: \"Never close when memory runs out\""
- [x] Right-click: Keep this running (checkpoint): frozen and its memory moved to the drive instead of closed, remembered — hosts/linux/desktop/shell/TaskManagerView.qml: "label: \"Keep this running (checkpoint)\""
- [x] The memory guard freezes a program on the Keep running list instead of closing it — os/system_files/usr/libexec/nimbus/hearth-memory: "freeze(v, procs)"
- [x] Right-click: Resume for a paused program; Status "Paused to save memory ⏸" — hosts/linux/desktop/shell/TaskManagerView.qml: "\"Paused to save memory \u23f8\""
- [x] NEW tags on the menu's new items — hosts/linux/desktop/shell/TaskManagerView.qml: "text: \"NEW\"; color: Theme.ember"
- [x] Right-click: Go to details shows the PID on the right — hosts/linux/desktop/shell/TaskManagerView.qml: "label: \"Go to details\", icon: \"list\", key: \"PID \" + pid"
- [x] Apps & groups / All processes switch at the top — hosts/linux/desktop/shell/TaskManagerView.qml: "tabs: [[\"groups\", \"Apps & groups\"], [\"all\", \"All processes\"]]"
- [x] Efficiency mode, Open file location and End task buttons at the top — hosts/linux/desktop/shell/TaskManagerView.qml: "{ id: \"where\", label: \"Open file location\", icon: \"folder\", enabled: pp.selGroup"
- [x] Tags on rows: AI (an AI job), bridge, Game · Proton, Hearth — hosts/linux/desktop/shell/TaskManagerView.qml: "tag: c.ai ? \"AI\" : c.bridge ? \"bridge\" : \"\""
- [x] Status: Quick Resume ⏸ for a suspended game — hosts/linux/desktop/shell/TaskManagerView.qml: "\"Quick Resume \u23f8\""
- [x] Search box: "Type a name, publisher or PID to search" (finds a PID) — hosts/linux/desktop/shell/TaskManagerView.qml: "Type a name, publisher or PID to search"

## tm-2 Details
- [x] Parent, Swapped, Threads, Priority columns — hosts/linux/desktop/shell/TaskManagerView.qml: "title: \"Swapped\""
- [x] Subtitle "every process with its number, the Windows way", Columns… and End task buttons — hosts/linux/desktop/shell/TaskManagerView.qml: "subtitle: \"every process with its number, the Windows way\""
- [x] CPU time and Cores columns — hosts/linux/desktop/shell/TaskManagerView.qml: "{ key: \"cputime\", title: \"CPU time\""
- [x] Right-click Set priority: Realtime, High, Above normal, Normal, Below normal, Low — hosts/linux/desktop/shell/TaskManagerView.qml: "{ id: \"realtime\", label: \"Realtime\" }, { id: \"high\", label: \"High\""
- [x] Right-click: Set affinity — hosts/linux/desktop/shell/TaskManagerView.qml: "Set affinity"
- [x] Right-click: Suspend / Resume — hosts/linux/desktop/shell/TaskManagerView.qml: "\"Resume\" : \"Suspend\""
- [x] Right-click: Create dump file — hosts/linux/desktop/shell/TaskManagerView.qml: "label: \"Create dump file\""
- [x] Right-click: Open file location and End process tree — hosts/linux/desktop/shell/TaskManagerView.qml: "{ id: \"tree\", label: \"End process tree\", icon: \"close\", enabled: r.mine"

## tm-3 CPU › Power & settings
- [x] The cores strip on top (each core's use, core 0 with its speed) and "name · N cores" on the right — hosts/linux/desktop/shell/DeviceSettings.qml: "\"0 \u00b7 \" + (ds.sample.cpu.mhz / 1000).toFixed(1) + \" GHz\""
- [x] Power mode: Battery saver / Balanced / Performance — hosts/linux/desktop/shell/DeviceSettings.qml: "[\"power-saver\", \"Battery saver\"]"
- [x] Power mode: When plugged in (switches by itself when you plug in or unplug) — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"When plugged in\"; sub: \"switches by itself when you plug in or unplug\""
- [x] Turbo boost — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Turbo boost\""
- [x] Max processor speed as a slider with its percent — hosts/linux/desktop/shell/DeviceSettings.qml: "Slide { from: 30; to: 100; stepSize: 5; value: ds.hw ? ds.hw.maxPct.value : 100"
- [x] Temperature now · limit — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Temperature now \u00b7 limit\""
- [x] Throttling: "none today" (counted per day) — hosts/linux/desktop/shell/DeviceSettings.qml: "\"none today\""
- [x] Game cores: Reserve for games, Cores for the game, Background work — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Reserve for games\""
- [x] Per-app limits card: a program with Max %, Updates and downloads at Low priority, + Add a program (remembered) — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Per-app limits\"; isNew: true; sub: \"Stop one program from taking the whole processor.\""

## tm-4 Disk › Health & settings
- [x] Numbers on top: Health (Good · % life left), Written so far (of ~TB rated), Temperature, Free on C:, Power-on hours — hosts/linux/desktop/shell/DeviceSettings.qml: "[\"Power-on hours\", h.hours ? thousands(h.hours) : \"\u2013\", \"\"]"
- [x] Size · type · maker on the right — hosts/linux/desktop/shell/DeviceSettings.qml: "if (kind === \"disk\") { const d = dev && dev.disk ? dev.disk : {};"
- [x] Optimize: TRIM with Optimize now and when it last ran — hosts/linux/desktop/shell/DeviceSettings.qml: "text: \"Optimize now\""
- [x] Optimize: Check the file system (btrfs scrub), Monthly, when it last ran — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Check the file system (btrfs scrub)\""
- [x] Optimize: Warn when C: is over (90% full) — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Warn when C: is over\""
- [x] What's using space: Games, Hearth versions kept, Swap file, Restore points, Temporary files & caches with Clean up — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"What's using space\"; isNew: true; sub: \"C: at a glance, biggest first.\""
- [x] Writes & safety: Save to the drive every 5 s / 10 s / 30 s — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Save to the drive every\"; sub: \"shorter = less lost if the power cuts\""
- [x] Writes & safety: Compress files on C: (zstd) — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Compress files on C: (zstd)\"; sub: \"saves space, a little CPU\""
- [x] Writes & safety: Snapshots of your files (Previous versions in Hearth Files), Every hour — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Snapshots of your files\"; sub: \"Previous versions in Hearth Files\""
- [x] Busiest programs: Program, PID, Read, Write (with zram ↔ drive swap) — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Busiest programs\""

## tm-5 Network › Apps & settings
- [x] The tab is called "Apps & settings" — hosts/linux/desktop/shell/TaskManagerView.qml: "net: \"Apps & settings\""
- [x] Numbers on top: Ping (to the internet) with jitter, Used today, IP · Tailscale IP, DNS — hosts/linux/desktop/shell/DeviceSettings.qml: "[\"Ping (to the internet)\""
- [x] Network name · Wi-Fi standard · link speed · signal on the right — hosts/linux/desktop/shell/DeviceSettings.qml: "\"signal \" + \"\u2582\u2584\u2586\u2588\".slice(0, w.bars)"
- [x] Which programs use the network: Program, PID, Down, Up, Today, Talking to, Allow (Allowed / Limit to … / Blocked) — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Which programs use the network\"; isNew: true"
- [x] Connection: Metered connection — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Metered connection\""
- [x] Connection: DNS (Automatic and others) — hosts/linux/desktop/shell/DeviceSettings.qml: "[[\"auto\", \"Automatic\"], [\"cloudflare\", \"Cloudflare (1.1.1.1)\"]"
- [x] Connection: Wi-Fi power saving (off = lower ping in games) — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Wi-Fi power saving\"; sub: \"off = lower ping in games\""
- [x] Tools: Speed test (Run) — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Speed test\""
- [x] Tools: Fix my connection (resets Wi-Fi and DNS, tells you what it found) — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Fix my connection\""
- [x] Tools: Share this connection (hotspot) as a switch — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Share this connection (hotspot)\""
- [x] A Tailscale entry in the left list (N devices) — hosts/linux/desktop/shell/TaskManagerView.qml: "x.kind === \"Tailscale\" ? (v.tsPeers >= 0 ? v.plural(v.tsPeers, \"device\")"

## tm-6 GPU › Tuning
- [x] Numbers on top: Busy, Video memory, Temperature, Power (of max), Fan (% and rpm), Game FPS — hosts/linux/desktop/shell/DeviceSettings.qml: "[\"Game FPS\", t.fps ? String(Math.round(t.fps)) : \"\u2013\", \"\"]"
- [x] Fan curve: a graph from 30 °C to 90 °C with points you drag; Mode Automatic / My curve / Quiet — hosts/linux/desktop/shell/DeviceSettings.qml: "options: [[\"auto\", \"Automatic\"], [\"curve\", \"My curve\"], [\"quiet\", \"Quiet\"]]"
- [x] Power & clocks: Power limit slider, Undervolt, Profile Quiet / Balanced / Max, Back to default (through LACT, off until you change something) — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Power & clocks\"; sub: \"Off until you change something. Every change remembered and undoable (LACT).\""
- [x] Programs on the graphics card: Program (with Proton · DXVK), PID, GPU, Video memory, Uses — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Programs on the graphics card\""

## tm-7 Battery › Health & settings
- [x] Numbers on top: Charge (on battery), Time left, Using now, Health (Wh of Wh), Charge cycles; model · Wh on the right — hosts/linux/desktop/shell/DeviceSettings.qml: "[\"Charge cycles\", hb.cycles ? String(hb.cycles) : \"\u2013\", \"\"]"
- [x] When the battery runs low: Warn at (10% and 5%) — hosts/linux/desktop/shell/DeviceSettings.qml: "[[\"10,5\", \"10% and 5%\"]"
- [x] When the battery runs low: At 5% Hibernate / Shut down / Sleep (hibernate only once proven) — hosts/linux/desktop/shell/DeviceSettings.qml: "sub: \"hibernate is offered only once it's proven to work on this PC\""
- [x] When the battery runs low: Hibernation on this laptop (tested · works) — hosts/linux/desktop/shell/DeviceSettings.qml: "\"tested \u00b7 works\""
- [x] Look after the battery: Stop charging at — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Stop charging at\""
- [x] Look after the battery: Battery saver from (20%) — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Battery saver from\""
- [x] Look after the battery: Dim the screen on battery — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Dim the screen on battery\""
- [x] What used the battery today: Program, PID, Share, Power now, Tip (the screen too) — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"What used the battery today\""

## tm-8 Startup apps, Services, Scheduled tasks
- [x] Startup apps: "Last start: N s (desktop ready)" — hosts/linux/desktop/shell/TaskManagerView.qml: "\" s (desktop ready)\""
- [x] Startup apps: Publisher, Time added to start and Starts (at sign-in, minimized, system) columns — hosts/linux/desktop/shell/TaskManagerView.qml: "{ key: \"added\", title: \"Time added to start\""
- [x] Services: Starts Automatic / When needed / Off, with what each one is for in plain words — hosts/linux/desktop/shell/TaskManagerView.qml: "{ key: \"starts\", title: \"Starts\", width: 120 }"
- [x] Scheduled tasks page — hosts/linux/desktop/shell/TaskManagerView.qml: "title: \"Scheduled tasks\""
- [x] Scheduled tasks: + New task (a program or script at a time, at sign-in, when idle or on battery) — hosts/linux/desktop/shell/TaskManagerView.qml: "{ id: \"new\", label: \"+ New task\", icon: \"plus\", primary: true }"

## Memory (task-manager-memory, task-manager-swap-settings)
- [x] Overview: programs' memory graph with compressed (zram) dashed — hosts/linux/desktop/shell/MemoryOverview.qml: "Memory in use (programs) · 60 seconds"
- [x] Overview: Memory composition bar and key — hosts/linux/desktop/shell/MemoryOverview.qml: "Memory composition"
- [x] Overview: In use, Available, Cached, Free, Compressed (in · ratio), Committed — hosts/linux/desktop/shell/MemoryOverview.qml: "[\"Committed\""
- [x] Overview: Swap card (zram and drive swap file bars) — hosts/linux/desktop/shell/MemoryOverview.qml: "Drive swap file (slow)"
- [x] Overview: Memory pressure card (now, 10 s · minute, a minute of bars) — hosts/linux/desktop/shell/MemoryOverview.qml: "How often programs waited for memory. High = stutter."
- [x] Overview: Memory setting card with Change in Settings — hosts/linux/desktop/shell/MemoryOverview.qml: "title: \"Memory setting\""
- [x] Overview: Biggest users (In use, Shared, Compressed) — hosts/linux/desktop/shell/MemoryOverview.qml: "BIGGEST USERS"
- [x] Overview: the On drive column in Biggest users — hosts/linux/desktop/shell/MemoryOverview.qml: "\"On drive\""
- [x] Overview: memory speed and slots on the right — hosts/linux/desktop/shell/MemoryOverview.qml: "\" MT/s\""
- [x] Swap & settings — hosts/linux/desktop/shell/TaskManagerView.qml: "memory: \"Swap & settings\""
- [x] Swap & settings: "Starts freeing below" with "then frees up to the target" — hosts/linux/desktop/shell/MemorySettings.qml: "title: \"Starts freeing below\"; sub: \"then frees up to the target\""
- [x] Swap & settings: Target with Custom — hosts/linux/desktop/shell/MemorySettings.qml: "[\"custom\", \"Custom\"]"
- [x] Swap & settings: Drive swap file Size "on C: (/var/swap) · hibernation needs at least your memory size" — hosts/linux/desktop/shell/MemorySettings.qml: "on C: (/var/swap) · hibernation needs at least your memory size"
- [x] Swap & settings: Hibernation (works on this laptop · last checked, ready) — hosts/linux/desktop/shell/MemorySettings.qml: "works on this laptop · last checked "
- [x] Swap & settings: Never close with + Add — hosts/linux/desktop/shell/MemorySettings.qml: "// + Add (1.12, mockup task-manager-swap-settings): any program, by its name"
