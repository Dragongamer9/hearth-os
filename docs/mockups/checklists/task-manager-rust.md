# Task Manager in pure Rust (the approved Task Manager mockups, ported from Mantel)

Mockups: next-update/tm-1-processes-with-pid.png, next-update/tm-2-details.png, next-update/tm-3-cpu-power-settings.png, next-update/tm-4-disk-health-settings.png, next-update/tm-5-network-apps-settings.png, next-update/tm-6-gpu-tuning.png, next-update/tm-7-battery-health-settings.png, next-update/tm-8-startup-services-tasks.png, next-update/task-manager-memory.png, next-update/task-manager-swap-settings.png
Release: next

The pure-Rust Task Manager (hosts/hearth-ui/apps/task-manager, docs/APPS-ANATASE.md) for Hearth OS, Fedora edition, where
Mantel's QML Task Manager can't run. The same mockups as task-manager.md; every part has to be in before it replaces
anything. Open parts are listed as open (red flag 11).

## tm-1 Processes
- [x] PID column — hosts/hearth-ui/apps/task-manager/src/pages.rs: "col(\"pid\", \"PID\", 76.0, true)"
- [x] Compressed column — hosts/hearth-ui/apps/task-manager/src/pages.rs: "col(\"swap\", \"Compressed\", 104.0, true)"
- [x] Power column — hosts/hearth-ui/apps/task-manager/src/pages.rs: "col(\"power\", \"Power\", 90.0, true)"
- [x] Apps, Hearth and Background processes sections — hosts/hearth-ui/apps/task-manager/src/pages.rs: "(\"Background processes\", bg)"
- [x] Totals over CPU, Memory, Disk, Network and GPU — hosts/hearth-ui/apps/task-manager/src/pages.rs: "tot(\"cpu\", \"CPU\", 80.0)"
- [x] Right-click: End task, End process tree, Efficiency mode, Set priority — hosts/hearth-ui/apps/task-manager/src/main.rs: "MenuItem::new(\"End process tree\", Act::Tree)"
- [x] Right-click: Run on these cores… — hosts/hearth-ui/apps/task-manager/src/main.rs: "MenuItem::new(\"Run on these cores…\", Act::Cores)"
- [x] Right-click: Limit CPU / memory… — hosts/hearth-ui/apps/task-manager/src/main.rs: "MenuItem::new(\"Limit CPU / memory…\", Act::Limit)"
- [x] Right-click: Never close when memory runs out — hosts/hearth-ui/apps/task-manager/src/main.rs: "MenuItem::new(\"Never close when memory runs out\", Act::Never)"
- [x] Right-click: Keep this running (checkpoint) — hosts/hearth-ui/apps/task-manager/src/main.rs: "MenuItem::new(\"Keep this running (checkpoint)\", Act::Keep(!g.keep_running))"
- [x] Right-click: Resume for a paused program — hosts/hearth-ui/apps/task-manager/src/main.rs: "MenuItem::new(\"Resume\", Act::Resume)"
- [x] Status "Paused to save memory ⏸" for a program the memory guard froze — hosts/hearth-ui/apps/task-manager/src/pages.rs: "if g.frozen { \"Paused to save memory ⏸\".into() }"
- [x] NEW tags on the menu's new items — hosts/hearth-ui/apps/task-manager/src/pages.rs: "chip(\"NEW\""
- [x] Right-click: Go to details shows the PID on the right — hosts/hearth-ui/apps/task-manager/src/main.rs: "MenuItem::new(\"Go to details\", Act::Details).key(format!(\"PID {pid}\"))"
- [x] Apps & groups / All processes switch at the top — hosts/hearth-ui/apps/task-manager/src/pages.rs: "(\"groups\", \"Apps & groups\"), (\"all\", \"All processes\")"
- [x] Efficiency mode, Open file location and End task buttons at the top — hosts/hearth-ui/apps/task-manager/src/pages.rs: "btn(&p, \"Open file location\""
- [x] Tags on rows: AI, bridge, Game · Proton, Hearth — hosts/hearth-ui/apps/task-manager/src/apps.rs: "\"Game · Proton\""
- [x] Status: Quick Resume ⏸ for a suspended game — hosts/hearth-ui/apps/task-manager/src/pages.rs: "\"Quick Resume ⏸\""
- [x] Search box: "Type a name, publisher or PID to search" (finds a PID) — hosts/hearth-ui/apps/task-manager/src/pages.rs: "Type a name, publisher or PID to search"
- [ ] App icons in the Name column

## tm-2 Details
- [x] Parent, Swapped, Threads, Priority columns — hosts/hearth-ui/apps/task-manager/src/pages.rs: "col(\"swap\", \"Swapped\", 90.0, true)"
- [x] Subtitle "every process with its number, the Windows way" and End task — hosts/hearth-ui/apps/task-manager/src/pages.rs: "\"every process with its number, the Windows way\""
- [x] Columns… picks any column but Name (remembered) — hosts/hearth-ui/apps/task-manager/src/main.rs: "MenuItem::new(c.title, Act::Col(c.key))"
- [x] CPU time and Cores columns — hosts/hearth-ui/apps/task-manager/src/pages.rs: "col(\"cputime\", \"CPU time\", 86.0, true)"
- [x] Right-click Set priority: Realtime, High, Above normal, Normal, Below normal, Low — hosts/hearth-ui/apps/task-manager/src/main.rs: "MenuItem::new(\"Realtime\", Act::Priority(\"realtime\"))"
- [x] Right-click: Set affinity — hosts/hearth-ui/apps/task-manager/src/main.rs: "MenuItem::new(\"Set affinity\", Act::Cores)"
- [x] Right-click: Suspend / Resume — hosts/hearth-ui/apps/task-manager/src/main.rs: "Act::Suspend(!suspended)"
- [x] Right-click: Create dump file — hosts/hearth-ui/apps/task-manager/src/main.rs: "MenuItem::new(\"Create dump file\", Act::Dump)"
- [x] Right-click: Open file location and End process tree — hosts/hearth-ui/apps/task-manager/src/main.rs: "MenuItem::new(\"End task\", Act::End)"

## tm-3 CPU › Power & settings
- [x] The cores strip on top (each core's use, core 0 with its speed) and "name · N cores" on the right — hosts/hearth-ui/apps/task-manager/src/devset.rs: "format!(\"0 · {:.1} GHz\""
- [x] The CPU page's Power & settings tab — hosts/hearth-ui/apps/task-manager/src/pages.rs: "\"cpu\" => \"Power & settings\""
- [x] Power mode: Battery saver / Balanced / Performance — hosts/hearth-ui/apps/task-manager/src/devset.rs: "(\"power-saver\", \"Battery saver\"), (\"balanced\", \"Balanced\"), (\"performance\", \"Performance\")"
- [x] Power mode: When plugged in — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"switches by itself when you plug in or unplug\""
- [x] Turbo boost — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"Turbo boost\", \"faster bursts, more heat\""
- [x] Max processor speed slider with its percent — hosts/hearth-ui/apps/task-manager/src/devset.rs: "w::slide(\"cpu.maxPct\", 30.0, 100.0, 5.0"
- [x] Temperature now · limit — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"Temperature now · limit\""
- [x] Throttling: "none today" — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"none today\".into()"
- [x] Game cores: Reserve for games, Cores for the game, Background work — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"Background work (AI, updates, builds)\""
- [x] Per-app limits card: a program with Max %, Updates and downloads, + Add a program — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"+ Add a program\""

## tm-4 Disk › Health & settings
- [x] The Disk page's Health & settings tab: Health, Written so far, Temperature, Free on C:, Power-on hours — hosts/hearth-ui/apps/task-manager/src/devset.rs: "x(\"Power-on hours\")"
- [x] Optimize: TRIM, Check the file system (btrfs scrub), Warn when C: is over — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"Check the file system (btrfs scrub)\""
- [x] What's using space with Clean up — hosts/hearth-ui/apps/task-manager/src/devset.rs: "small_btn(a, \"Clean up\", \"cleanup\")"
- [x] Writes & safety: Save to the drive every, Compress files on C: (zstd), Snapshots of your files — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"Snapshots of your files\""
- [x] Busiest programs: Program, PID, Read, Write — hosts/hearth-ui/apps/task-manager/src/devset.rs: "(\"Program\", 0.0, false), (\"PID\", 80.0, true), (\"Read\", 90.0, true), (\"Write\", 90.0, true)"

## tm-5 Network › Apps & settings
- [x] The Network page's tab called "Apps & settings": Ping with jitter, Used today, IP · Tailscale IP, DNS — hosts/hearth-ui/apps/task-manager/src/devset.rs: "x(\"Ping (to the internet)\")"
- [x] Network name · Wi-Fi standard · link speed · signal on the right — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"▂▄▆█\""
- [x] Which programs use the network: Program, PID, Down, Up, Today, Talking to, Allow — hosts/hearth-ui/apps/task-manager/src/devset.rs: "(\"Talking to\", 260.0, false), (\"Allow\", 200.0, false)"
- [x] Connection: Metered connection, DNS, Wi-Fi power saving — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"Wi-Fi power saving\", \"off = lower ping in games\""
- [x] Tools: Speed test, Fix my connection, Share this connection (hotspot) — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"Share this connection (hotspot)\""
- [x] A Tailscale entry in the left list (N devices) — hosts/hearth-ui/apps/task-manager/src/pages.rs: "format!(\"{} device{}\", a.ts_peers"

## tm-6 GPU › Tuning
- [x] Numbers on top: Busy, Video memory, Temperature, Power (of max), Fan (% and rpm), Game FPS — hosts/hearth-ui/apps/task-manager/src/devset.rs: "x(\"Game FPS\")"
- [x] Fan curve: a graph from 30 °C to 90 °C with points you drag; Mode Automatic / My curve / Quiet — hosts/hearth-ui/apps/task-manager/src/devset.rs: "(\"auto\", \"Automatic\"), (\"curve\", \"My curve\"), (\"quiet\", \"Quiet\")"
- [x] Power & clocks: Power limit slider, Undervolt, Profile, Back to default (LACT) — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"Back to default\""
- [x] Programs on the graphics card: Program, PID, GPU, Video memory, Uses — hosts/hearth-ui/apps/task-manager/src/devset.rs: "(\"Video memory\", 120.0, true), (\"Uses\", 160.0, false)"

## tm-7 Battery › Health & settings
- [x] Numbers on top: Charge, Time left, Using now, Health, Charge cycles; model · Wh — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"Full in\" } else { \"Time left\" }"
- [x] When the battery runs low: Warn at, At 5%, Hibernation on this laptop — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"Hibernation on this laptop\""
- [x] Look after the battery: Stop charging at, Battery saver from, Dim the screen on battery — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"Dim the screen on battery\""
- [x] What used the battery today: Program, PID, Share, Power now, Tip — hosts/hearth-ui/apps/task-manager/src/devset.rs: "\"What used the battery today\""

## tm-8 Startup apps, Services, Scheduled tasks
- [x] Startup apps: "Last start: N s (desktop ready)" — hosts/hearth-ui/apps/task-manager/src/pages.rs: "\"Last start: {} s (desktop ready)\""
- [x] Startup apps: Publisher, Time added to start and Starts columns — hosts/hearth-ui/apps/task-manager/src/pages.rs: "col(\"added\", \"Time added to start\", 160.0, true)"
- [x] Services: Starts Automatic / When needed / Off, with what each one is for in plain words — hosts/hearth-ui/apps/task-manager/src/pages.rs: "col(\"starts\", \"Starts\", 120.0, false)"
- [x] Scheduled tasks page — hosts/hearth-ui/apps/task-manager/src/main.rs: "Page::Tasks => \"Scheduled tasks\""
- [x] Scheduled tasks: + New task (a program or script at a time, at sign-in, when idle or on battery) — hosts/hearth-ui/apps/task-manager/src/main.rs: "(\"battery\", \"When it goes on battery\")"

## Memory (task-manager-memory, task-manager-swap-settings)
- [x] Overview: programs' memory graph with compressed (zram) dashed — hosts/hearth-ui/apps/task-manager/src/memset.rs: "\"- - compressed (zram)\""
- [x] Overview: Memory composition bar and key — hosts/hearth-ui/apps/task-manager/src/memset.rs: "\"Cached: given back when needed\""
- [x] Overview: In use, Available, Cached, Free, Compressed (in · ratio), Committed — hosts/hearth-ui/apps/task-manager/src/memset.rs: "num(\"Committed\""
- [x] Overview: Swap card (zram and drive swap file bars) — hosts/hearth-ui/apps/task-manager/src/memset.rs: "\"Drive swap file (slow)\""
- [x] Overview: Memory pressure card (now, 10 s · minute, a minute of bars) — hosts/hearth-ui/apps/task-manager/src/memset.rs: "\"How often programs waited for memory. High = stutter.\""
- [x] Overview: Memory setting card with Change in Swap & settings — hosts/hearth-ui/apps/task-manager/src/memset.rs: "\"Change in Swap & settings ›\""
- [x] Overview: Biggest users (In use, Shared, Compressed, On drive) — hosts/hearth-ui/apps/task-manager/src/memset.rs: "rcell(\"On drive\".into(), 100.0, p.muted)"
- [x] Overview: memory speed and slots on the right — hosts/hearth-ui/apps/task-manager/src/memset.rs: " MT/s"
- [x] Swap & settings tab: Compressed memory (zram), Keep memory free (Target with Custom, Starts freeing below, Swappiness), Drive swap file (Size, Hibernation), When memory runs out (Never close with + Add), Save — hosts/hearth-ui/apps/task-manager/src/memset.rs: "\"Starts freeing below\", \"then frees up to the target\""
