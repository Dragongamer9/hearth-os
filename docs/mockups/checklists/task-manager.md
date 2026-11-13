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
- [ ] Right-click: Keep this running (checkpoint)
- [ ] Right-click: Go to details shows the PID on the right
- [ ] Apps & groups / All processes switch at the top
- [ ] Efficiency mode, Open file location and End task buttons at the top
- [ ] Tags on rows: AI (an AI job), bridge, Game · Proton, Hearth
- [ ] Status: Quick Resume ⏸ for a suspended game
- [ ] Search box: "Type a name, publisher or PID to search" (finds a PID)

## tm-2 Details
- [x] Parent, Swapped, Threads, Priority columns — hosts/linux/desktop/shell/TaskManagerView.qml: "title: \"Swapped\""
- [ ] Subtitle "every process with its number, the Windows way", Columns… and End task buttons
- [ ] CPU time and Cores columns
- [ ] Right-click Set priority: Realtime, High, Above normal, Normal, Below normal, Low
- [x] Right-click: Set affinity — hosts/linux/desktop/shell/TaskManagerView.qml: "Set affinity"
- [ ] Right-click: Suspend / Resume
- [ ] Right-click: Create dump file
- [ ] Right-click: Open file location and End process tree

## tm-3 CPU › Power & settings
- [ ] The cores strip on top (each core's use, core 0 with its speed) and "name · N cores" on the right
- [x] Power mode: Battery saver / Balanced / Performance — hosts/linux/desktop/shell/DeviceSettings.qml: "[\"power-saver\", \"Battery saver\"]"
- [ ] Power mode: When plugged in (switches by itself when you plug in or unplug)
- [x] Turbo boost — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Turbo boost\""
- [ ] Max processor speed as a slider with its percent
- [ ] Temperature now · limit
- [ ] Throttling: "none today" (counted per day)
- [x] Game cores: Reserve for games, Cores for the game, Background work — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Reserve for games\""
- [ ] Per-app limits card: a program with Max %, Updates and downloads at Low priority, + Add a program (remembered)

## tm-4 Disk › Health & settings
- [ ] Numbers on top: Health (Good · % life left), Written so far (of ~TB rated), Temperature, Free on C:, Power-on hours
- [ ] Size · type · maker on the right
- [x] Optimize: TRIM with Optimize now and when it last ran — hosts/linux/desktop/shell/DeviceSettings.qml: "text: \"Optimize now\""
- [ ] Optimize: Check the file system (btrfs scrub), Monthly, when it last ran
- [ ] Optimize: Warn when C: is over (90% full)
- [ ] What's using space: Games, Hearth versions kept, Swap file, Restore points, Temporary files & caches with Clean up
- [ ] Writes & safety: Save to the drive every 5 s / 10 s / 30 s
- [ ] Writes & safety: Compress files on C: (zstd)
- [ ] Writes & safety: Snapshots of your files (Previous versions in Hearth Files), Every hour
- [ ] Busiest programs: Program, PID, Read, Write (with zram ↔ drive swap)

## tm-5 Network › Apps & settings
- [ ] The tab is called "Apps & settings"
- [ ] Numbers on top: Ping (to the internet) with jitter, Used today, IP · Tailscale IP, DNS
- [ ] Network name · Wi-Fi standard · link speed · signal on the right
- [ ] Which programs use the network: Program, PID, Down, Up, Today, Talking to, Allow (Allowed / Limit to … / Blocked)
- [x] Connection: Metered connection — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Metered connection\""
- [ ] Connection: DNS (Automatic and others)
- [ ] Connection: Wi-Fi power saving (off = lower ping in games)
- [ ] Tools: Speed test (Run)
- [ ] Tools: Fix my connection (resets Wi-Fi and DNS, tells you what it found)
- [ ] Tools: Share this connection (hotspot) as a switch
- [ ] A Tailscale entry in the left list (N devices)

## tm-6 GPU › Tuning
- [ ] Numbers on top: Busy, Video memory, Temperature, Power (of max), Fan (% and rpm), Game FPS
- [ ] Fan curve: a graph from 30 °C to 90 °C with points you drag; Mode Automatic / My curve / Quiet
- [ ] Power & clocks: Power limit slider, Undervolt, Profile Quiet / Balanced / Max, Back to default (through LACT, off until you change something)
- [ ] Programs on the graphics card: Program (with Proton · DXVK), PID, GPU, Video memory, Uses

## tm-7 Battery › Health & settings
- [ ] Numbers on top: Charge (on battery), Time left, Using now, Health (Wh of Wh), Charge cycles; model · Wh on the right
- [ ] When the battery runs low: Warn at (10% and 5%)
- [ ] When the battery runs low: At 5% Hibernate / Shut down / Sleep (hibernate only once proven)
- [ ] When the battery runs low: Hibernation on this laptop (tested · works)
- [x] Look after the battery: Stop charging at — hosts/linux/desktop/shell/DeviceSettings.qml: "title: \"Stop charging at\""
- [ ] Look after the battery: Battery saver from (20%)
- [ ] Look after the battery: Dim the screen on battery
- [ ] What used the battery today: Program, PID, Share, Power now, Tip (the screen too)

## tm-8 Startup apps, Services, Scheduled tasks
- [ ] Startup apps: "Last start: N s (desktop ready)"
- [ ] Startup apps: Publisher, Time added to start and Starts (at sign-in, minimized, system) columns
- [ ] Services: Starts Automatic / When needed / Off, with what each one is for in plain words
- [x] Scheduled tasks page — hosts/linux/desktop/shell/TaskManagerView.qml: "title: \"Scheduled tasks\""
- [ ] Scheduled tasks: + New task (a program or script at a time, at sign-in, when idle or on battery)

## Memory (task-manager-memory, task-manager-swap-settings)
- [x] Overview: programs' memory graph with compressed (zram) dashed — hosts/linux/desktop/shell/MemoryOverview.qml: "Memory in use (programs) · 60 seconds"
- [x] Overview: Memory composition bar and key — hosts/linux/desktop/shell/MemoryOverview.qml: "Memory composition"
- [x] Overview: In use, Available, Cached, Free, Compressed (in · ratio), Committed — hosts/linux/desktop/shell/MemoryOverview.qml: "[\"Committed\""
- [x] Overview: Swap card (zram and drive swap file bars) — hosts/linux/desktop/shell/MemoryOverview.qml: "Drive swap file (slow)"
- [x] Overview: Memory pressure card (now, 10 s · minute, a minute of bars) — hosts/linux/desktop/shell/MemoryOverview.qml: "How often programs waited for memory. High = stutter."
- [x] Overview: Memory setting card with Change in Settings — hosts/linux/desktop/shell/MemoryOverview.qml: "title: \"Memory setting\""
- [x] Overview: Biggest users (In use, Shared, Compressed) — hosts/linux/desktop/shell/MemoryOverview.qml: "BIGGEST USERS"
- [ ] Overview: the On drive column in Biggest users
- [ ] Overview: memory speed and slots on the right
- [x] Swap & settings — hosts/linux/desktop/shell/TaskManagerView.qml: "memory: \"Swap & settings\""
- [ ] Swap & settings: "Starts freeing below" with "then frees up to the target"
