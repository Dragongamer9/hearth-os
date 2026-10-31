# One app of each kind per desktop

**The rule:** each desktop shows exactly one app of each kind (one file manager, one terminal, one text editor...), and
opens files in that app.

| Desktop | `XDG_CURRENT_DESKTOP` | Shows |
|---|---|---|
| Hearth Classic (labwc + Mantel) and Game Mode | `Nimbus` | Hearth's own classic apps: Files, Terminal, Text Editor, Calculator, Documents, Task Manager (screenshots: Mantel's own) |
| Hearth Plasma | `KDE` | the KDE-based ones: Files (Dolphin), Terminal (Konsole), Text Editor (Kate), Reader (Okular), Capture (Spectacle), Calculator (KCalc), Plasma's System Monitor |
| GNOME, COSMIC, Deepin | `GNOME`, `COSMIC`, `Deepin` / `DDE` | their own apps, once they register them (below) |

Apps with no other app of their kind (Photos, Player, Office, Maps, Contacts, Calendar, Paint, Camera, Recorder, the
browser, Bazaar...) show on every desktop, unless a desktop registers its own app of that kind: then that one is used
there and Hearth's is hidden there. Stock apps that duplicate one (KWrite, Gwenview, Elisa, Haruna, Discover,
Firefox's own entry...) are hidden. Game Mode is `Nimbus` too, so it opens files like Hearth Classic.

## How it works

- **The table**: `os/system_files/usr/share/hearth/app-kinds.json` (in the image: `/usr/share/hearth/app-kinds.json`).
- **The tool**: `os/system_files/usr/libexec/hearth-app-kinds`, run by both images' build.sh (the "app kinds" block,
  after Hearth's default apps) once every app is installed. It:
  1. writes a copy of each affected menu entry with `OnlyShowIn=` / `NotShowIn=` (or `NoDisplay=true`) into
     `/usr/share/hearth/app-kinds/applications`. That folder is an XDG data folder that `nimbus-session` puts first in
     `XDG_DATA_DIRS` for every desktop it starts, so the copies win over the packages' files (same ids). Package files
     are never edited, so package updates don't undo it; on Peppermint an apt hook
     (`/etc/apt/apt.conf.d/80-hearth-app-kinds`) writes the copies again after every apt run (`--refresh`: it never
     fails apt and never hides an app you installed yourself);
  2. writes each desktop's default apps, the XDG way: `/etc/xdg/<desktop>-mimeapps.list` (lower case: `nimbus-`,
     `kde-`, `gnome-`, `cosmic-`, `deepin-`, `dde-`) and, for terminals, `/etc/xdg/<desktop>-xdg-terminals.list`.
     Lines it doesn't own are kept. Your own choices (Settings › Default apps, `~/.config/mimeapps.list`) still win;
  3. **stops the build** if a desktop that's in the image would show two apps of one kind, or none where the table
     says one is needed, or would open a kind's files in another app.
- Hearth's own menus follow the same entries: Mantel's launcher (hides what `hearth-app-kinds hidden` lists for its
  desktop), the Hearth Plasma launcher (`hearth-plasma apps`: OnlyShowIn/NotShowIn), and Hearth Files' "Open with"
  (`hearth-files`: OnlyShowIn/NotShowIn for the current desktop). Game Mode has no app menu.
- Checks: `hosts/linux/test/test_app_kinds.py` (run before each image build) and the image check in
  `base-anatase.yml` / `base-peppermint.yml` (`hearth-app-kinds check`, plus GLib's `gio mime` as a warning).

Commands: `hearth-app-kinds apply [--root DIR] [--refresh]`, `check [--root DIR]`, `hidden [DESKTOP]`,
`resolve DESKTOP`.

## The table

```json
{
  "desktops": {
    "GNOME": {"label": "Hearth GNOME", "names": ["GNOME"], "session": ["/usr/bin/gnome-shell"]}
  },
  "kinds": {
    "files": {
      "label": "File manager",
      "required": ["Nimbus", "KDE"],
      "categories": ["FileManager"],
      "mimes": ["inode/directory", "x-directory/normal"],
      "apps": {
        "Nimbus": ["hearth-files.desktop"],
        "KDE": ["hearth-files-kde.desktop", "org.kde.dolphin.desktop", "hearth-files.desktop"]
      },
      "everywhere": [],
      "others": []
    }
  }
}
```

- `desktops.<D>`: `names` are the `XDG_CURRENT_DESKTOP` names that desktop sets (written into `OnlyShowIn`/`NotShowIn`
  and used for `<name>-mimeapps.list`); `session` are programs whose presence means the desktop is in the image (a
  desktop that isn't in the image is never required to have anything).
- `kinds.<K>.apps.<D>`: that desktop's app for the kind, in order of preference: the first one installed (and whose
  `TryExec` is found) is used, the rest are fallbacks and are hidden. An empty list means "none on this desktop"
  (Hearth Classic's screenshots are Mantel's own).
- `everywhere`: the app for every desktop not listed in `apps` (also shown in desktops Hearth doesn't know).
- `others`: other apps of this kind to hide wherever the kind has an app (stock duplicates).
- `categories`: any other installed app with one of these `Categories=` counts as this kind too, and is hidden
  wherever the kind has an app (a desktop with none for the kind keeps it).
- `mimes`: the types this kind opens; `image/*` style patterns take every matching type the chosen app lists. The
  chosen app becomes the default for each type it lists in its own `MimeType=`. Two kinds may not share a type.
- `required`: desktops that must show one (the build fails otherwise, when that desktop is in the image).
- `terminal: true`: also writes `<desktop>-xdg-terminals.list`.

## Adding a desktop and its native apps

1. Add it under `desktops` with the `XDG_CURRENT_DESKTOP` names its session really sets and a `session` program.
2. For each kind it has its own app for, add `"<Desktop>": ["<its .desktop id>"]` under that kind's `apps`, e.g.
   `"GNOME": ["org.gnome.Nautilus.desktop"]` under `files`, `["org.gnome.Console.desktop"]` under `terminal`,
   `["org.gnome.TextEditor.desktop"]` under `text-editor`, `["org.gnome.Loupe.desktop"]` under `image-viewer`. Its
   apps are then shown only there, and Hearth's app of that kind is hidden there. Add a new kind if needed (with its
   `mimes`).
3. Add it to `required` for the kinds it must always have.
4. Start its session from `nimbus-session` (so `XDG_DATA_DIRS` has the copies first) with its own
   `XDG_CURRENT_DESKTOP`.
5. Run `python3 -m unittest discover -s hosts/linux/test -p test_app_kinds.py` (a test, "a desktop registers its
   apps", shows the shape), then let the image build's check prove it.
