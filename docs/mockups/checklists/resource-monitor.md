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
