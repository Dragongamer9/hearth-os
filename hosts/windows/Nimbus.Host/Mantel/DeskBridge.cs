using System.Text.Json;

namespace Nimbus.Host.Mantel;

/// <summary>
/// Bridge calls for the newer parts of Mantel on Windows (SystemBridge hands over anything starting with one of
/// <see cref="Prefixes"/>): the taskbar's and desktop's saved state and the desktop icons (hearth-desk on Hearth OS),
/// the notification history, virtual desktops, Hearth Settings' Date &amp; time / Keyboard / Default apps / Users /
/// Printers / Clipboard pages, the clipboard history picker, and Apps (winget).
/// Called on the UI thread (WinRT's clipboard needs it); slow work goes to the thread pool inside each service.
/// </summary>
internal static class DeskBridge
{
    private static readonly string[] Prefixes = { "desk.", "notify.", "vdesk.", "time.", "keyboard.", "defaults.", "users.", "printers.", "clipboard.", "apps." };
    public static bool Handles(string method) => Prefixes.Any(method.StartsWith);

    public static async Task<object?> DispatchAsync(string method, JsonElement a, Action<string, object> raise)
    {
        switch (method)
        {
            // ------------------------------------------------ the taskbar's and desktop's state (DeskState.qml)
            case "desk.get": return State();
            case "desk.set":
            {
                var key = Req(a, "key");
                bool on = Bool(a, "value");
                MantelState.Update(d =>
                {
                    switch (key)
                    {
                        case "desktopIcons": d.DesktopIcons = on; break;
                        case "dnd": d.Dnd = on; break;
                        case "clock24": d.Clock24 = on; break;
                        default: throw new ArgumentException("Unknown setting: " + key);
                    }
                });
                return State();
            }
            case "desk.icons": return await Task.Run(() => DesktopIcons.List(Int(a, "cols", 1), Int(a, "rows", 1)));
            case "desk.place": return await Task.Run(() => DesktopIcons.Place(Req(a, "name"), Int(a, "col"), Int(a, "row"), Int(a, "cols", 1), Int(a, "rows", 1)));
            case "desk.arrange": return await Task.Run(() => DesktopIcons.Arrange(Int(a, "cols", 1), Int(a, "rows", 1)));
            case "desk.newFolder": return await Task.Run(() => DesktopIcons.NewFolder(Str(a, "name")));

            // ------------------------------------------------ notifications (the clock's panel)
            case "notify.list": return Notes();
            case "notify.forget": Notifications.Forget(Int(a, "key")); return Notes();
            case "notify.clear": Notifications.Clear(); return Notes();
            case "notify.seen": Notifications.Seen(); return Notes();

            // ------------------------------------------------ virtual desktops (DeskSwitcher.qml)
            case "vdesk.get": return new { desks = VirtualDesktops.Get() };
            case "vdesk.switch": VirtualDesktops.Switch(Int(a, "index")); await Task.Delay(350); return new { desks = VirtualDesktops.Get() };
            case "vdesk.move":
            {
                var ok = VirtualDesktops.MoveWindow(new IntPtr(Long(a, "window")), Req(a, "desk"), out var error);
                return new { ok, error };
            }

            // ------------------------------------------------ Settings › Date & time
            case "time.get": return WinSystem.TimeGet();
            case "time.zones": return await Task.Run(WinSystem.TimeZones);
            case "time.setZone": await WinSystem.TimeSetZone(Req(a, "zone")); return WinSystem.TimeGet();
            case "time.setNtp": await WinSystem.TimeSetNtp(Bool(a, "on")); return WinSystem.TimeGet();
            case "time.set": await WinSystem.TimeSet(Req(a, "when")); return WinSystem.TimeGet();

            // ------------------------------------------------ Settings › Keyboard
            case "keyboard.get": return await WinSystem.KeyboardGet();
            case "keyboard.layouts": return await Task.Run(WinSystem.KeyboardLayouts);
            case "keyboard.shortcuts": return WinSystem.Shortcuts();
            case "keyboard.set":
                return await WinSystem.KeyboardSet(Strs(a, "layouts"), a.ValueKind == JsonValueKind.Object && a.TryGetProperty("switch", out var sw) && sw.ValueKind == JsonValueKind.String ? sw.GetString() : null,
                    Int(a, "delay", 500), Int(a, "rate", 30));

            // ------------------------------------------------ Settings › Default apps, Users, Printers
            case "defaults.get": return await Task.Run(WinSystem.Defaults);
            case "users.get": return await Task.Run(WinSystem.Users);
            case "users.remove": await WinSystem.UserRemove(Req(a, "name"), Bool(a, "deleteFiles")); return await Task.Run(WinSystem.Users);
            case "printers.get": return await Task.Run(WinSystem.Printers);
            case "printers.default": await Task.Run(() => WinSystem.PrinterDefault(Req(a, "name"))); return await Task.Run(WinSystem.Printers);
            case "printers.test": await Task.Run(() => WinSystem.PrinterTest(Req(a, "name"))); return null;
            case "printers.remove": await Task.Run(() => WinSystem.PrinterRemove(Req(a, "name"))); return await Task.Run(WinSystem.Printers);

            // ------------------------------------------------ Settings › Clipboard and the history picker
            case "clipboard.status": return ClipboardHistory.Status();
            case "clipboard.set": return ClipboardHistory.SetEnabled(Bool(a, "on"));
            case "clipboard.list": return await ClipboardHistory.ListAsync();
            case "clipboard.delete": ClipboardHistory.Delete(Req(a, "id")); return null;
            case "clipboard.clear": ClipboardHistory.Clear(); return null;

            // ------------------------------------------------ Apps (winget)
            case "apps.status": return await Winget.StatusAsync();
            case "apps.browse": return await Winget.BrowseAsync(Req(a, "what"));
            case "apps.search": return await Winget.SearchAsync(Str(a, "q") ?? "");
            case "apps.installed": return await Winget.InstalledAsync();
            case "apps.updates": return await Winget.UpdatesAsync();
            case "apps.app": return await Winget.AppAsync(Req(a, "id"));
            case "apps.install": case "apps.update": case "apps.remove":
            {
                var id = Req(a, "id");
                void Progress(double p, string line) => raise("apps.progress", new { id, progress = p, line });
                if (method == "apps.install") await Winget.InstallAsync(id, Progress);
                else if (method == "apps.update") await Winget.UpdateAsync(id, Progress);
                else await Winget.RemoveAsync(id, Progress);
                return new { ok = true };
            }
        }
        throw new NotSupportedException($"Unknown method: {method}");
    }

    public static object State()
    {
        var d = MantelState.Load();
        return new { pins = d.Pins, desktopIcons = d.DesktopIcons, dnd = d.Dnd, clock24 = d.Clock24 };
    }

    public static object Notes() => new { history = Notifications.All, unseen = Notifications.Unseen, dnd = MantelState.Load().Dnd };

    // ------------------------------------------------------------------ args
    private static string? Str(JsonElement a, string k) =>
        a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() : null;
    private static string Req(JsonElement a, string k) => Str(a, k) ?? throw new ArgumentException($"Missing '{k}'");
    private static bool Bool(JsonElement a, string k) => a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.True;
    private static int Int(JsonElement a, string k, int fallback = 0) =>
        a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Number ? (int)Math.Round(v.GetDouble()) : fallback;
    private static long Long(JsonElement a, string k) =>
        a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Number ? (long)v.GetDouble() : 0;
    private static string[] Strs(JsonElement a, string k) =>
        a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.Array
            ? v.EnumerateArray().Where(x => x.ValueKind == JsonValueKind.String).Select(x => x.GetString() ?? "").Where(x => x.Length > 0).ToArray()
            : Array.Empty<string>();
}

/// <summary>
/// The desktop's icons in full Mantel (Desktop.qml + hearth-desk on Hearth OS): what's in your Desktop folder and the
/// Public desktop, each with a place on a grid that's remembered (mantel.json, "icons"). New files take the first
/// free place (down the first column, then the next); a saved place outside today's grid is kept for a bigger screen.
/// </summary>
internal static class DesktopIcons
{
    public static string UserDir => Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory);
    public static string PublicDir => Environment.GetFolderPath(Environment.SpecialFolder.CommonDesktopDirectory);

    private sealed record Item(string Name, string Label, string Path, bool Dir, bool Launcher, string Ext, long Size, bool Public);

    private static List<Item> Items()
    {
        var outp = new List<Item>();
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var (dir, isPublic) in new[] { (UserDir, false), (PublicDir, true) })
        {
            if (string.IsNullOrEmpty(dir) || !Directory.Exists(dir)) continue;
            IEnumerable<FileSystemInfo> list;
            try { list = new DirectoryInfo(dir).EnumerateFileSystemInfos(); } catch { continue; }
            foreach (var fi in list)
            {
                try
                {
                    if ((fi.Attributes & (FileAttributes.Hidden | FileAttributes.System)) != 0) continue;
                    if (fi.Name.Equals("desktop.ini", StringComparison.OrdinalIgnoreCase)) continue;
                    if (!seen.Add(fi.Name)) continue;
                    bool isDir = (fi.Attributes & FileAttributes.Directory) != 0;
                    var ext = isDir ? "" : fi.Extension.ToLowerInvariant();
                    bool launcher = ext is ".lnk" or ".url" or ".appref-ms";
                    // Windows hides shortcuts' extensions; other files keep their full name, like Hearth OS shows them.
                    var label = launcher ? System.IO.Path.GetFileNameWithoutExtension(fi.Name) : fi.Name;
                    outp.Add(new Item(fi.Name, label, fi.FullName, isDir, launcher, ext, isDir ? 0 : ((FileInfo)fi).Length, isPublic));
                }
                catch { }
            }
        }
        outp.Sort((x, y) =>
        {
            if (x.Dir != y.Dir) return x.Dir ? -1 : 1;
            int c = string.Compare(x.Label, y.Label, StringComparison.CurrentCultureIgnoreCase);
            return c != 0 ? c : string.CompareOrdinal(x.Name, y.Name);
        });
        return outp;
    }

    /// <summary>Places every name on the grid (hearth-desk's layout()): saved places inside the grid are kept (the first
    /// name wins a clash), the rest take the first free places; more icons than places stack past the last column.</summary>
    private static (Dictionary<string, (int C, int R)> Placed, List<string> Fresh) Layout(List<string> names, Dictionary<string, int[]> saved, int cols, int rows)
    {
        cols = Math.Max(1, cols); rows = Math.Max(1, rows);
        var taken = new HashSet<(int, int)>();
        var placed = new Dictionary<string, (int C, int R)>();
        var fresh = new List<string>();
        foreach (var n in names)
            if (saved.TryGetValue(n, out var cell) && cell.Length == 2 && cell[0] >= 0 && cell[0] < cols && cell[1] >= 0 && cell[1] < rows && taken.Add((cell[0], cell[1])))
                placed[n] = (cell[0], cell[1]);
        var free = new Queue<(int, int)>();
        for (int c = 0; c < cols; c++) for (int r = 0; r < rows; r++) if (!taken.Contains((c, r))) free.Enqueue((c, r));
        int overflow = 0;
        foreach (var n in names)
        {
            if (placed.ContainsKey(n)) continue;
            if (free.Count > 0) placed[n] = free.Dequeue();
            else { placed[n] = (cols + overflow / rows, overflow % rows); overflow++; }
            if (!saved.ContainsKey(n)) fresh.Add(n);
        }
        return (placed, fresh);
    }

    public static object List(int cols, int rows)
    {
        var items = Items();
        var d = MantelState.Load();
        var names = items.Select(i => i.Name).ToList();
        var (placed, fresh) = Layout(names, d.Icons, cols, rows);
        var gone = d.Icons.Keys.Where(k => !placed.ContainsKey(k)).ToList();
        if (fresh.Count > 0 || gone.Count > 0)
        {
            foreach (var n in gone) d.Icons.Remove(n);
            foreach (var n in fresh) d.Icons[n] = new[] { placed[n].C, placed[n].R };
            MantelState.Save(d, notify: false);
        }
        return new
        {
            dir = UserDir,
            exists = Directory.Exists(UserDir),
            entries = items.Select(i => new { name = i.Name, label = i.Label, path = i.Path, dir = i.Dir, launcher = i.Launcher, ext = i.Ext, size = i.Size, @public = i.Public, col = placed[i.Name].C, row = placed[i.Name].R }).ToList(),
        };
    }

    public static object Place(string name, int col, int row, int cols, int rows)
    {
        var names = Items().Select(i => i.Name).ToList();
        if (!names.Contains(name)) throw new InvalidOperationException("“" + name + "” isn't on the desktop any more");
        col = Math.Clamp(col, 0, Math.Max(1, cols) - 1);
        row = Math.Clamp(row, 0, Math.Max(1, rows) - 1);
        var d = MantelState.Load();
        var (placed, _) = Layout(names, d.Icons, cols, rows);
        var old = placed[name];
        foreach (var (other, cell) in placed)
            if (other != name && cell == (col, row)) { d.Icons[other] = new[] { old.C, old.R }; break; }   // swap
        d.Icons[name] = new[] { col, row };
        MantelState.Save(d, notify: false);
        return List(cols, rows);
    }

    public static object Arrange(int cols, int rows)
    {
        var d = MantelState.Load();
        d.Icons.Clear();
        MantelState.Save(d, notify: false);
        return List(cols, rows);
    }

    public static object NewFolder(string? name)
    {
        var dir = UserDir;
        Directory.CreateDirectory(dir);
        var baseName = string.IsNullOrWhiteSpace(name) ? "New folder" : name.Trim();
        if (baseName.IndexOfAny(System.IO.Path.GetInvalidFileNameChars()) >= 0 || baseName is "." or "..") throw new ArgumentException("A name can't contain \\ / : * ? \" < > |");
        var target = System.IO.Path.Combine(dir, baseName);
        for (int n = 2; Directory.Exists(target) || File.Exists(target); n++) target = System.IO.Path.Combine(dir, $"{baseName} {n}");
        Directory.CreateDirectory(target);
        return new { ok = true, path = target, name = System.IO.Path.GetFileName(target) };
    }
}
