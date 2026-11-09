# Device Manager, Run and the Super+X menu

Three of Windows' tools in Mantel (Desktop Mode), in Hearth's look and working with mouse, touch, keyboard and the
controller. All three live in `hosts/linux/desktop/shell/WinTools.qml`, which `shell.qml` loads by file name: a
problem in them only loses these three, never the desktop or Game Mode. Nothing runs until one is opened (no service,
no timer), and opening one writes nothing.

## Device Manager

Opens from the Super+X menu, `devmgmt.msc` in Run, or `quickshell ipc -p <shell> call devices open`.

- **The tree**: the PC's name, then Audio, Bluetooth, Cameras, Controllers, Disk drives, Display adapters, Keyboards and
  mice, Monitors, Network adapters, Processors and USB controllers (and **Other devices** when something fits no
  group). Groups with a problem start open. Which groups are open is only for that window.
- **The details** (right): the driver (its kernel module, and whether it's built into Hearth, Hearth's kernel, or
  added by Hearth like NVIDIA's DKMS driver), firmware, maker, where it's connected (USB 3 port 1-4 (5 Gbps), PCI slot,
  Bluetooth, built in) and the hardware id.
- **Problems** get a yellow warning, and the count is in the toolbar and the status line:
  - **No driver**: a PCI device of a kind that should have one (graphics, network, sound, storage) with none bound,
    or a USB device of a standard kind (sound, HID, storage, camera, Bluetooth, network) whose parts have none.
    USB devices that apps talk to directly (phones over MTP, vendor-specific gadgets) need no driver and aren't
    flagged. Board parts that have no driver on Linux and work fine (SMBus, signal processing) aren't listed.
  - **Missing firmware**: the kernel log of this boot says `firmware: failed to load …` / `Direct firmware load for
    … failed` / `firmware file … not found` for that device, and the file isn't on disk. Not flagged: optional files
    (`.pnvm`, `regulatory.db`), and drivers that try a newer firmware first and found an older one (iwlwifi's
    `-89.ucode` then `-86.ucode`).
  - **Errors**: a driver's `probe … failed with error`, a radio or network chip whose driver loaded but no adapter
    appeared, and a USB port where something didn't start (`device descriptor read`, `device not accepting address`)
    shown as "Unknown USB device (port 1-7)".
  - The kernel log is read with `journalctl -k -b` (people in the `adm` or `systemd-journal` group), else `dmesg`.
    When neither can be read, Firmware says "Can't check" instead of guessing.
- **Actions** (the buttons under the details; X or A on a device goes to them):
  - **Send logs**: the same as Settings › Updates & recovery › Send logs to the developer (`nimbus --send-logs`, or
    `hearth-send-logs`), with the device's name, id, problem and missing files in the note; the code to share shows
    under the details.
  - **Disable device / Enable device**: only Bluetooth adapters and Wi-Fi, as an rfkill soft block (the same as
    airplane mode for that one radio; reversible, no password). Press twice to turn one off. Never storage, never the
    screen or graphics card in use, never anything Hearth runs from: everything else has no Disable button.
  - **Nvidia driver settings**: Settings › Display, where the NVIDIA drawing choice is.
  - The device's Settings page (Sound, Bluetooth, Network, Display, Controller, Keyboard, Mouse & touchpad), **Open
    Disk Management** for drives (when Hearth Disks is installed) and **Performance in Task Manager** for processors.
  - **There's no "Get the firmware"**: Hearth has no way to fetch a single firmware file today (they come with
    updates of the image), so Device Manager says which file is missing and offers Send logs, so it can be added.
- **Scan for hardware changes** (F5) reads everything again.

Keys and the controller: Up / Down move, Right / A open a group or go to the device's actions, Left / B come back,
X (Menu) the actions, B in the list closes.

### How it reads devices

`hosts/linux/desktop/shell/hearth-devmgr list` prints one JSON line. It reads `/sys/bus/pci`, `/sys/bus/usb`,
`/sys/class/{net,bluetooth,sound,video4linux,drm,rfkill,block}`, `/proc/bus/input/devices`, `/proc/cpuinfo`,
`/proc/mounts`, `/proc/asound/cards`, `modules.dep` / `modules.builtin` for where a driver comes from, EDID for a
screen's name, and pci.ids / usb.ids for names (only the ids it needs). Hearth's own virtual controller and keyboard
(uinput, under `/devices/virtual`) aren't listed. `hearth-devmgr disable|enable ID` writes one rfkill change to
`/dev/rfkill` (or runs `rfkill`); it refuses anything that isn't a radio. `HEARTH_DEVMGR_ROOT` and `HEARTH_DEVMGR_LOG`
point it at sample files (hosts/linux/test/test_win_tools.py builds a whole made-up PC).

## Run (Super+R)

Super+R (it used to open the app menu's search; Super+S still does), Run in the Super+X menu, or
`quickshell ipc -p <shell> call run open`. Type, pick a suggestion with Up / Down (Tab fills it in), Enter (A) opens,
Esc (B) closes; Browse… picks a file with Hearth Files' Open dialog.

- **Windows commands** (one data file, `hosts/linux/desktop/shell/win-commands.json`; add a line to add one):
  taskmgr → Task Manager; msconfig → Task Manager › Startup apps; resmon → Resource Monitor; perfmon → Performance; devmgmt.msc → Device
  Manager; diskmgmt.msc → Hearth Disks; control, ms-settings: (and ms-settings:display, :bluetooth, :windowsupdate…)
  → Settings; the .cpl files (ncpa, mmsys, desk, main, powercfg, timedate, sysdm, intl, appwiz) → their Settings page
  or Files' Installed apps; notepad → Text Editor; cmd, powershell, wt → Terminal; explorer → Files; calc →
  Calculator; eventvwr → Logs; osk → the on-screen keyboard; snippingtool → a screenshot of an area; rstrui → System
  Restore (`hearth-restore`); perfmon /rel → Reliability history (`hearth-reliability`). `.exe` and capitals are fine.
- **Folders**: %appdata% → ~/.config, %localappdata% → ~/.local/share, %temp% → ~/.cache, %userprofile% → your home
  folder, with a path after them too (`%appdata%\discord`); `C:\Users\you\Documents`, `D:\Games` (This PC's drive
  letters, `hearth_drive_letters.py` and `drive-paths.js`); `~/…` and `/…`; `\\pc\share` opens the shared folder.
  Folders open in Files, files in their app.
- **Websites** (`https://…`, `www.…`) open in the browser; **apps** by name (their .desktop entries); any other
  installed program by its name.
- A command for an app that isn't installed isn't suggested, and says so when typed.
- **Recent entries** come first in the suggestions. They're state, not a setting: `~/.local/state/nimbus/run-history.json`
  (`{"recent": [...]}`, 12 at most), written through SettingsWriter (atomic, `.bak`) only after something typed opened.
  Opening the box writes nothing.

## The Super+X menu

Super+X, or a right-click on the Start button (the Hearth flame) in any bar layout. Windows' list, each with its
underlined letter: Installed apps (P), Power options (O), Reliability history (L), System (Y), Device Manager (M),
Network connections (W), Disk Management (K), Terminal (I), Task Manager (T), Settings (N), Files (E), Search (S),
Run (R), Shut down or sign out › (U: Sign out, Lock, Sleep, Shut down, Restart, Game Mode), Desktop (D).

- Entries whose app isn't installed (Reliability history, Disk Management) are hidden.
- **Terminal (Admin) is left out**: Hearth has no admin terminal; things that need it ask for your password themselves.
- Reliability history's letter is L, not Windows' V: the underlined letter has to be in the name.
- Keys and the controller: the letter opens its entry, Up / Down and Enter (A), Right opens Shut down or sign out,
  Left / Esc (B) go back, then close. A click outside closes it.
- The entries are the `winx` list in `win-commands.json`.

## Checks

- `hosts/linux/test/test_win_tools.py`: device grouping and problems from a made-up PC's sysfs and kernel log, Disable
  only for radios, the Run map with node (commands, %folders%, C:\ paths, apps that aren't installed), the menu's
  hidden entries, Super+R / Super+X in labwc, the Loader, and that opening writes nothing.
- `.github/workflows/desktop-test.yml` opens all three on the virtual screen, takes pictures (`shots/win-tools/`) and
  fails if their QML reports errors or if opening them changed a file.
