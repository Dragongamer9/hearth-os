# Resource Monitor (approved Nov 11–12, 2026)

Mockups: next-update/resmon-1-overview.png, next-update/resmon-2-processor.png, next-update/resmon-3-memory.png, next-update/resmon-4-disk.png, next-update/resmon-5-network.png, next-update/resmon-6-graphics.png
Release: 1.12

## resmon-1 Overview
- [x] Processor, Memory, Disk, Network, Graphics sections with chips and top programs — hosts/linux/desktop/shell/ResourceMonitorView.qml: "Graphics"
- [x] "Pause ⏸ · Open Task Manager" on top — hosts/linux/desktop/shell/ResourceMonitorView.qml: "text: \"Open Task Manager\""
- [x] Memory chip: compressed amount and pressure — hosts/linux/desktop/shell/ResourceMonitorView.qml: "\" compressed\", \"\"]"
- [x] Tick programs: "Filtered to …" shown above every section — hosts/linux/desktop/shell/ResourceMonitorView.qml: "\"Filtered to <b>"

## resmon-2 Processor
- [x] Priority for each program — hosts/linux/desktop/shell/ResourceMonitorView.qml: "Priority"
- [x] Services of the picked program (name, what it is) — hosts/linux/desktop/shell/ResourceMonitorView.qml: "key: \"cpu:services\", title: \"Services\""
- [x] Open files with their C:\ names — hosts/linux/desktop/shell/ResourceMonitorView.qml: "rows: openFiles.slice(0, 40).map(x => [f.name, String(x.pid), x.win || x.path])"
- [x] Libraries (modules) of the picked program — hosts/linux/desktop/shell/ResourceMonitorView.qml: "title: \"Libraries (modules)\""

## resmon-3 Memory
- [x] Compressed column for programs — hosts/linux/desktop/shell/ResourceMonitorView.qml: "Compressed"
- [x] Physical memory bar: Kept for hardware, In use, Compressed (zram real size), Cached, Free — hosts/linux/desktop/shell/ResourceMonitorView.qml: "[\"Kept for hardware\", size(mb.reserved || 0)]"
- [x] Private column — hosts/linux/desktop/shell/ResourceMonitorView.qml: "memory: [\"In use\", \"Private\", \"Shared\", \"Compressed\", \"On drive\", \"Waits / s\"]"
- [x] Swapping in / out — hosts/linux/desktop/shell/ResourceMonitorView.qml: "[\"Swapping in / out\""
- [x] Waits / s (memory pressure) — hosts/linux/desktop/shell/hearth-resmon: "g['waits']"

## resmon-4 Disk
- [x] Drives: Drive, Type, Busy, Response, Free, Total, Full, Temp — hosts/linux/desktop/shell/ResourceMonitorView.qml: "Files being read and written"
- [x] Programs using the disk (btrfs and zram included) — hosts/linux/desktop/shell/ResourceMonitorView.qml: "name: \"btrfs (system)\""
- [x] File names shown as C:\ and G:\ paths — hosts/linux/desktop/shell/ResourceMonitorView.qml: "(x.writing ? \"\u270e \" : \"\") + (x.win || x.path)"

## resmon-5 Network
- [x] Listening ports with who can reach them — hosts/linux/desktop/shell/ResourceMonitorView.qml: "listening ports and open connections"
- [x] Programs using the network (Down, Up, Talking to) — hosts/linux/desktop/shell/ResourceMonitorView.qml: "progs.title = \"Programs using the network\""
- [x] Adapters: Wi-Fi name with speed and link, Tailscale with devices — hosts/linux/desktop/shell/ResourceMonitorView.qml: "\" devices\" : \"\") : n.speed ? \" \u00b7 \" + n.speed + \" Mbps link\""

## resmon-6 Graphics
- [x] 3D, Video decode, Video memory columns — hosts/linux/desktop/shell/ResourceMonitorView.qml: "Video decode"
- [x] Programs using the graphics card with what they draw with (Vulkan, OpenGL, VA-API video) — hosts/linux/desktop/shell/hearth-resmon: "def graphics_api(maps):"
- [x] Shared memory — hosts/linux/desktop/shell/ResourceMonitorView.qml: "[\"Shared memory\""
- [x] Screens (each screen, its mode) — hosts/linux/desktop/shell/ResourceMonitorView.qml: "[\"Screens\""
- [x] Games running (with Vulkan · OpenGL) — hosts/linux/desktop/shell/ResourceMonitorView.qml: "[\"Games running\""

## Also in the pictures
- [x] Processor: Cores column, and the chips "% used · threads · Up …" — hosts/linux/desktop/shell/ResourceMonitorView.qml: "[\"Up \" + upTime(s.uptime || 0), \"\"]"
- [x] Processor: every core's graph with its speed, and the temperature with its limit — hosts/linux/desktop/shell/ResourceMonitorView.qml: "\" · limit \" + v.tempLimit + \"°C\""
- [x] Memory: Used memory, Compressed (zram), Memory pressure and Swapping graphs — hosts/linux/desktop/shell/ResourceMonitorView.qml: "[\"Compressed (zram)\", s.zram ? v.size(s.zram.orig) : \"off\""
- [x] Disk: Priority column, each drive's busy graph, Health in Drives — hosts/linux/desktop/shell/ResourceMonitorView.qml: "disk: [\"Read\", \"Write\", \"Total\", \"Priority\"]"
- [x] Network: Connections with Talking to (by name), Ping and State; Listening ports with What it is and the firewall chip — hosts/linux/desktop/shell/ResourceMonitorView.qml: "[\"State\", 1]]"
- [x] Graphics: Video encode and Through columns; Driver with Mesa, Clock, Power, Vulkan · OpenGL — hosts/linux/desktop/shell/ResourceMonitorView.qml: "card.mesa ? \"Mesa \" + card.mesa"

## Fedora edition (Rust)

The Fedora edition's Resource Monitor (Mantel runs only on Debian): hearth-resmon-app, hosts/hearth-ui/apps/resmon,
with Mantel's helper (desktop/shell/hearth-resmon) and files. Pictured by hearth-apps-rust.yml (.github/scripts/
hearth-apps-rust-shots.d/resmon.sh, the mockups' numbers from resmon-fixture.py) and set beside each mockup.

- [x] resmon-1: tabs Your PC, Overview, Processor, Memory, Disk, Network, Graphics (picture resmon-1-overview) — hosts/hearth-ui/apps/resmon/src/main.rs: "Tab::Pc => \"Your PC\""
- [x] resmon-1: Live · every second, Pause and Open Task Manager (the Fedora edition's hearth-task-manager) — hosts/hearth-ui/apps/resmon/src/main.rs: "command -v hearth-task-manager"
- [x] resmon-1: Processor, Memory, Disk, Network, Graphics sections with chips and top programs, folded by a click (resmon-1-overview, resmon-10-overview-unfolded) — hosts/hearth-ui/apps/resmon/src/view.rs: "(Tab::Gpu, \"Graphics\")"
- [x] resmon-1: Memory chip with compressed amount and pressure — hosts/hearth-ui/apps/resmon/src/view.rs: "\"{} compressed\""
- [x] resmon-1: ticking programs: "Filtered to …" and the hint under the sections — hosts/hearth-ui/apps/resmon/src/view.rs: "Tick programs to filter every section to just them"
- [x] resmon-1: graphs Processor, Memory, Disk C:, Network, Graphics — hosts/hearth-ui/apps/resmon/src/view.rs: "gs(\"Graphics\", pct(last(h, \"gpu\")"
- [x] resmon-2: Programs with CPU, Threads, Priority, Cores and the chips % used · threads · Up (resmon-2-processor) — hosts/hearth-ui/apps/resmon/src/view.rs: "cols(&[\"CPU\", \"Threads\", \"Priority\", \"Cores\"])"
- [x] resmon-2: Filtered to the picked or ticked program — hosts/hearth-ui/apps/resmon/src/view.rs: "\" · showing its services, open files and libraries\""
- [x] resmon-2: Services of the picked program (name, PID, status, CPU, what it is) — hosts/hearth-ui/apps/resmon/src/view.rs: "\"cpu:services\", \"Services\""
- [x] resmon-2: Open files with their C:\ names and the search box — hosts/hearth-ui/apps/resmon/src/view.rs: "t1(\"search:\", 13.0"
- [x] resmon-2: Libraries (modules), folded as in the mockup (resmon-11-libraries open) — hosts/hearth-ui/apps/resmon/src/view.rs: "\"Libraries (modules)\""
- [x] resmon-2: every core's graph with its speed, and the temperature with its limit — hosts/hearth-ui/apps/resmon/src/view.rs: "\" · limit {}°C\""
- [x] resmon-3: Programs with In use, Private, Shared, Compressed, On drive, Waits / s (resmon-3-memory) — hosts/hearth-ui/apps/resmon/src/view.rs: "\"In use\", \"Private\", \"Shared\", \"Compressed\", \"On drive\", \"Waits / s\""
- [x] resmon-3: Physical memory bar (Kept, In use, Compr., Cached, Free) and its facts — hosts/hearth-ui/apps/resmon/src/view.rs: "(\"Kept for hardware\".to_string()"
- [x] resmon-3: graphs Used memory, Compressed (zram), Memory pressure, Swapping in / out — hosts/hearth-ui/apps/resmon/src/view.rs: "gs(\"Compressed (zram)\""
- [x] resmon-4: Programs using the disk with Read, Write, Total, Priority, btrfs (system) included (resmon-4-disk) — hosts/hearth-ui/apps/resmon/src/view.rs: "\"{fs} (system)\""
- [x] resmon-4: Files being read and written, as C:\ and G:\ paths — hosts/hearth-ui/apps/resmon/src/view.rs: "\"Files being read and written\""
- [x] resmon-4: Drives: Drive, Type, Busy, Response, Free, Total, Health, Temp — hosts/hearth-ui/apps/resmon/src/view.rs: "c(\"Health\", 1.0, true)"
- [x] resmon-4: each drive's busy graph and Response time — hosts/hearth-ui/apps/resmon/src/view.rs: "gs(\"Response time\""
- [x] resmon-5: Programs using the network (Down, Up, Total) (resmon-5-network) — hosts/hearth-ui/apps/resmon/src/view.rs: "\"Programs using the network\""
- [x] resmon-5: Connections (TCP) with Talking to by name, Port, Ping, State — hosts/hearth-ui/apps/resmon/src/view.rs: "\"Connections (TCP)\""
- [x] resmon-5: Listening ports with Open to, What it is and the firewall chip — hosts/hearth-ui/apps/resmon/src/view.rs: "format!(\"firewall: {fw}\")"
- [x] resmon-5: graphs Wi-Fi (name, speed, link), Tailscale (devices), Ping — hosts/hearth-ui/apps/resmon/src/view.rs: "\" · {} devices\""
- [x] resmon-6: Programs using the graphics card with 3D, Video decode, Video encode, Video memory, Through (resmon-6-graphics) — hosts/hearth-ui/apps/resmon/src/view.rs: "\"3D\", \"Video decode\", \"Video encode\", \"Video memory\", \"Through\""
- [x] resmon-6: Graphics card: Name, Driver (Mesa), Shared memory, Clock, Temperature, Power, Screens, Vulkan · OpenGL, Games running — hosts/hearth-ui/apps/resmon/src/view.rs: "(\"Vulkan · OpenGL\".into()"
- [x] resmon-6: graphs 3D, Video decode, Video memory, Temperature — hosts/hearth-ui/apps/resmon/src/view.rs: "gs(\"Video decode\", pct(last(h, \"gdec\")"
- [x] Right-click a program: End task (asks first: resmon-9-end-asks), Efficiency mode, Open file location, Search online (resmon-8-menu), through hearth-resmon end / eco (Task Manager's hearth-tasks) — hosts/hearth-ui/apps/resmon/src/view.rs: "It closes now. Anything you haven't saved in it may be lost."
- [x] Every table sorts by a click on its column (again for the other way) — hosts/hearth-ui/apps/resmon/src/view.rs: "Msg::Sort(tb.key.clone(), i)"
- [x] Keyboard and controller: LB / RB tabs, Up / Down, Space ticks, X menu, Delete ends (asks), B closes — hosts/hearth-ui/apps/resmon/src/main.rs: "Some(Named::PageDown) => return step(a, 1)"
- [x] Live only while the window is seen: minimized, the sampler and the timers stand still (CI: resmon-rest) — hosts/hearth-ui/apps/resmon/src/feed.rs: "Two samples with no frame drawn for 2.5 s"
- [x] Your PC: the board drawn from hearth-resmon pc, the part's details, tips and button (resmon-7, -12, -13, -14) — hosts/hearth-ui/apps/resmon/src/board.rs: "TIPS FOR THIS PC"
- [x] Plasma's menu: Resource Monitor (hearth-resmon.desktop) opens it on the Fedora edition — os-bases/anatase/build.sh: "Exec=/usr/bin/hearth-resmon-app"
