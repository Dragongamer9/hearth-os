# Resource Monitor (approved Nov 11–12, 2026)

Mockups: next-update/resmon-1-overview.png, next-update/resmon-2-processor.png, next-update/resmon-3-memory.png, next-update/resmon-4-disk.png, next-update/resmon-5-network.png, next-update/resmon-6-graphics.png
Release: 1.12

## resmon-1 Overview
- [x] Processor, Memory, Disk, Network, Graphics sections with chips and top programs — hosts/linux/desktop/shell/ResourceMonitorView.qml: "Graphics"
- [ ] "Pause ⏸ · Open Task Manager" on top
- [ ] Memory chip: compressed amount and pressure
- [ ] Tick programs: "Filtered to …" shown above every section

## resmon-2 Processor
- [x] Priority for each program — hosts/linux/desktop/shell/ResourceMonitorView.qml: "Priority"
- [ ] Services of the picked program (name, what it is)
- [ ] Open files with their C:\ names
- [ ] Libraries (modules) of the picked program

## resmon-3 Memory
- [x] Compressed column for programs — hosts/linux/desktop/shell/ResourceMonitorView.qml: "Compressed"
- [ ] Physical memory bar: Kept for hardware, In use, Compressed (zram real size), Cached, Free
- [ ] Private column
- [ ] Swapping in / out
- [ ] Waits / s (memory pressure)

## resmon-4 Disk
- [x] Drives: Drive, Type, Busy, Response, Free, Total, Full, Temp — hosts/linux/desktop/shell/ResourceMonitorView.qml: "Files being read and written"
- [ ] Programs using the disk (btrfs and zram included)
- [ ] File names shown as C:\ and G:\ paths

## resmon-5 Network
- [x] Listening ports with who can reach them — hosts/linux/desktop/shell/ResourceMonitorView.qml: "listening ports and open connections"
- [ ] Programs using the network (Down, Up, Talking to)
- [ ] Adapters: Wi-Fi name with speed and link, Tailscale with devices

## resmon-6 Graphics
- [x] 3D, Video decode, Video memory columns — hosts/linux/desktop/shell/ResourceMonitorView.qml: "Video decode"
- [ ] Programs using the graphics card with what they draw with (Vulkan, OpenGL, VA-API video)
- [ ] Shared memory
- [ ] Screens (each screen, its mode)
- [ ] Games running (with Vulkan · OpenGL)
