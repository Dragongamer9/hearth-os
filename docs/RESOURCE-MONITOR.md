# Resource Monitor

Hearth's Resource Monitor (1.10) shows what's inside this PC, drawn the way it really is, and what each program uses,
live. The look follows the approved mockups (Main, Memory, Gpu, GameOverlay, BoardReal, PcView): Hearth's dark blue,
Inter, and the board view's part colours.

## What it has

- **Your PC** (the first tab): the board or device drawn from what this machine reports. Each part (memory slots, PCIe
  and M.2 slots, SATA ports, rear USB ports, fan headers, the sound chip, the chipset, the CMOS battery, the BIOS chip,
  the Super I/O chip…) is shown as busy, connected or empty. Wires run from each part to what it really talks to (the
  processor, the chipset, the Super I/O chip, the sound chip, a handheld's controls chip), with one line per lane (16
  lanes are drawn as 10). Busy links get moving flow and a lightning bolt; both hold still with Settings ›
  Accessibility › Reduce motion. Click a part (or move to it with the D-pad and press A) for its details: status, live
  numbers, the full spec list, what it's wired to, which programs use it right now, and a button that goes somewhere
  useful (Disk Management, Device Manager, the volume mixer, a Settings page). A button only shows when that place
  exists on this system.
- **Tips**, only when the data shows they're true: Hearth on a hard drive, an empty M.2 slot (when the board lists its
  slots), memory sticks all on one channel, a single stick, a low CMOS battery (when the board reports its voltage), no
  Wi-Fi.
- **Overview, Processor, Memory, Disk, Network, Graphics**: per-program tables with tags (Game · Proton, Game, Steam,
  Hearth, Home server, System, Windows app), and four live graphs on the right (processor, disk, network, and memory;
  graphics on the Graphics tab).
  - Memory: a bar split into reserved, in use, being saved, cache and free (from /proc/meminfo), explained in plain
    words, and each program's memory in use, shared and swapped.
  - Graphics: per card: busy, temperature, power, clock, fan and video memory. Per program: graphics use and video
    memory, from nvidia-smi pmon on NVIDIA and DRM fdinfo on amdgpu, Intel and others.
  - Network: per program, its open connections and one address it's talking to. Linux doesn't count network bytes
    per program without root or eBPF, so there's no per-program speed.
- **Right-click a program** (or press X): End task, Efficiency mode, Open file location, Search online. These are Task
  Manager's own actions, through hearth-tasks, with the same checks: only your own programs, and never a part of
  Hearth.
- **Controller**: LB / RB switch tabs, the D-pad moves, A selects, X opens actions (on Your PC: the part's button), and
  B closes the menu and then the window.

Open it from the app menu (Resource Monitor), Task Manager › Performance › Open Resource Monitor, Settings › System ›
About › Resource Monitor, or `hearth-resmon` (the launcher name for the Run box and Super+X). `hearth-resmon memory`
opens a tab directly.

## How it stays light and safe

- Nothing runs until the window opens. `hearth-resmon serve` samples once a second, only while the window is open, and
  ends when it closes. Each tab only asks for what it shows: the process list always, sockets only on Network
  (every 3 s), nvidia-smi only on Graphics and Your PC.
- **No setting is written.** Opening it and using it change no settings file. The only file it writes is a cache:
  `~/.cache/hearth/resmon-hwinfo.json`, the memory and slot tables, so the root helper isn't run every time.
- It's loaded by file name through Loaders (shell.qml → ResourceMonitor.qml → ResourceMonitorView.qml →
  ResmonBoard.qml). A problem in any of them only loses Resource Monitor, never Mantel or Game Mode. There is no
  always-on service.

### The privileged helper

Memory sticks' maker, part number, speed and slot (SMBIOS type 17), the slots (type 9) and ports (type 8) are in the
firmware's table `/sys/firmware/dmi/tables/DMI`, which only root can read.
`/usr/libexec/nimbus/hearth-hwinfo` reads it and prints only those tables. It never prints serial numbers, asset tags,
UUIDs or anything else. It takes no arguments, writes nothing and changes nothing.

Resource Monitor runs it through `pkexec` once, when it first opens on a machine, then caches the answer. The cache is
keyed on the board, the product, the BIOS version and the installed memory, so new memory gets read again. "Read them
now" on Your PC reads it again.

**Polkit:** `/usr/share/polkit-1/rules.d/49-hearth-hwinfo.rules` allows this one helper **without a password for the
person at the device** (a local, active session), like 49-hearth-network does for Wi-Fi. It only reads. When it can't
run, Your PC still draws, with the memory from what Linux knows without root (EDAC where present, else the total from
the memory blocks).

## How the drawing is chosen

`hearth-resmon pc` detects the machine and picks one of three families. The choice is in `layout_choice()`:

1. **Handheld**, by DMI name first, because some handhelds report a desktop chassis:
   - ROG Ally (RC71L), ROG Ally X (RC72LA), ROG Xbox Ally (RC73YA), ROG Xbox Ally X (RC73XA)
   - Steam Deck (Jupiter), Steam Deck OLED (Galileo)
   - Legion Go (83E1), Legion Go S, Legion Go 2

   Each has built-in facts: screen, ports, SSD size, microSD, trackpads. The live parts come from the device. Other
   handhelds (Claw, AYANEO, OneXPlayer, GPD, chassis type 11) get the generic handheld.
2. **Laptop**: the chassis type says laptop, notebook, convertible or detachable. Soldered memory is drawn as chips,
   SO-DIMM slots as slots (empty ones too). Also drawn: the SSD, Wi-Fi, fan, battery with its health and cycles, the
   ports, and a separate graphics chip if there is one.
3. **Desktop**: a board template laid out from what's there. ATX, mATX or ITX comes from the board's name (B760M →
   mATX, -I → ITX), else from its slots. The pieces:
   - **Memory slots**: from DMI type 17, empty ones included.
   - **PCIe and M.2 slots**: from DMI type 9, matched to the device in each by PCI address. Without type 9 they come
     from what's plugged in, plus one x1 and one x4 slot drawn as unknown.
   - **SATA ports**: from `/sys/class/ata_port`, and the drive on each from its sysfs path (`ataN`). DVD drives too.
   - **Rear USB ports**: from ACPI's `physical_location` (back or front; a USB 3 port's two halves counted once), each
     device on the port it's really in. Without that, an estimate, said so in the subtitle.
   - **Lanes and wiring**: from each device's `current_link_speed` / `current_link_width` ("PCIe 5.0 x16"), and whether
     its root port belongs to the processor or the chipset (Intel's PCH ports, AMD's Promontory and FCH).
   - **Fan headers, voltages and the Super I/O chip**: from hwmon (nct67xx, it87…).
   - **The chipset name**: from the board's name, else the LPC bridge's.

   When the board says almost nothing (no DMI board name, no PCI), a clean generic board is drawn with what's known.

If drawing fails for any reason, the generic layout is used with the data cleaned up, and the processor alone as a last
resort. It's never a broken picture. The tests check that every part is on the stage, has a title, a status and a
style the board can paint, and is wired to something that exists.

## Game Mode: Quick Access › Resources

- **Steam Game Mode**: a Resources section at the top of the Hearth Sync Decky plugin's panel
  (`hosts/decky-hearth-sync/src/resources.tsx`). The backend loads hearth-resmon's sampler in-process, asked once a
  second only while the panel is open, and drops it a minute later.
- **Hearth Game Mode** (ui/, classic side panel): a Resources tab after Performance. It asks `resources.sample`
  (hearth-core `resources.rs` → hearth-resmon) once a second while the tab shows. perf.stats stands in where the
  helper isn't.

Both show graphics, processor, memory and network meters with temperatures, and what else is running.

**Pause downloads and backups while I play** uses existing pauses only:
- Steam's own download pause (Steam Game Mode), or Hearth's downloads.pauseAll (Hearth Game Mode).
- The home server's runtime flag: `hearth-resmon pause on|off` runs `hearth-home-server resume off|on`. With Light
  mode on, backups and big jobs wait while you play.

**It changes no setting.** The pause lasts this session: Steam's ends with Steam, the home server's flag is gone after
a restart, and a Hearth download paused from here stays paused until you resume it, as with Downloads › Pause all.
When Light mode is off, a separate button, **Always wait while I play**, turns it on (`pause light-on`), and only
because you pressed it.

## Files

- `hosts/linux/desktop/shell/hearth-resmon`: detection, the drawing, the sampler, Game Mode's brief, the pause
- `hosts/linux/desktop/shell/ResourceMonitor.qml`: the window and its IPC (`resmon`)
- `hosts/linux/desktop/shell/ResourceMonitorView.qml`: the page
- `hosts/linux/desktop/shell/ResmonBoard.qml`: the board painter
- `os/system_files/usr/libexec/nimbus/hearth-hwinfo`, `os/system_files/usr/share/polkit-1/rules.d/49-hearth-hwinfo.rules`
- `os/system_files/usr/bin/hearth-resmon`, `os/system_files/usr/share/applications/hearth-resmon.desktop`
- Tests: `hosts/linux/test/test_resource_monitor.py`, using sample machines in `hosts/linux/test/fixtures/resmon/` (the
  owner's i7-14700F / B760 / RTX 5070 / 2 × 16 GB DDR5; dad's ASRock FM2A88X Extreme6+ / A10 Richland / 4 × 8 GB DDR3 /
  R9 390 / hard drive; a laptop; a ROG Ally). Desktop Mode's CI draws the board from each (desktop-test.yml,
  `ipc resmon fixture <file>`).
