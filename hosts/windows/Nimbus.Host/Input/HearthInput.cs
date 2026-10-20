using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Nimbus.Host.Input;

// Hearth Input on Windows: controller layouts that make games without controller support playable. While a game
// with a layout is in front, the controller (read through XInput by ControllerBridge) acts as a keyboard and mouse
// (SendInput). The same shared tables as Hearth (hosts/linux/desktop/shell/input-layouts.json, built into Ember),
// the same per-game choices and checks as hearth-core's input_map.rs, and the same mapping engine as
// desktop/shell/hearth-input (ported line for line: taps, hold, turbo, the second set, radial menu, stick curves).
//
// Differences from Hearth, because of Windows:
//   * The controller can't be taken away from the game (Linux grabs the device). A game that reads controllers itself
//     sees both the controller and the keys, so layouts are for games without controller support (the default is
//     Off for every game). HidHide (a driver) could hide it; Ember says when it's installed but doesn't set it up.
//   * XInput has no back paddles (L4/R4/L5/R5) and no motion sensors, so those controls and gyro aiming do nothing.
//   * Keys are sent as scan codes (the key's place on the keyboard, like Linux's KEY_* names), which DirectInput and
//     raw-input games read too. Nothing is sent while one of Ember's own windows is in front, except to try a layout
//     on its own page (Try it), which catches the keys itself.

/// <summary>A layout that isn't allowed (the reason in Hearth's words).</summary>
public sealed class LayoutException : Exception { public LayoutException(string m) : base(m) { } }

/// <summary>The shared tables: keys, controls, limits, templates.</summary>
public static class InputTables
{
    private static readonly Lazy<JsonObject> RootLazy = new(() =>
    {
        using var s = typeof(InputTables).Assembly.GetManifestResourceStream("input-layouts.json")
                      ?? throw new InvalidOperationException("input-layouts.json isn't built into Ember");
        var root = JsonNode.Parse(s)!.AsObject();
        // Read by several threads (the controller's and the bridge's): every node made now, so later reads only look.
        static void Walk(JsonNode? n)
        {
            if (n is JsonObject o) foreach (var (_, v) in o) Walk(v);
            else if (n is JsonArray a) foreach (var v in a) Walk(v);
            else n?.GetValueKind();
        }
        Walk(root);
        return root;
    });
    public static JsonObject Root => RootLazy.Value;

    private static readonly Lazy<HashSet<string>> KeysLazy = new(() =>
        Root["keys"]!.AsArray().SelectMany(g => g!["keys"]!.AsArray().Select(k => k![0]!.GetValue<string>())).ToHashSet());
    public static HashSet<string> Keys => KeysLazy.Value;

    public static string[] Ids(string list) => Root[list]!.AsArray().Select(x => x![0]!.GetValue<string>()).ToArray();
    public static string Label(string list, string id) => Root[list]!.AsArray().FirstOrDefault(x => x![0]!.GetValue<string>() == id)?[1]?.GetValue<string>() ?? id;
    public static string KeyLabel(string key) =>
        Root["keys"]!.AsArray().SelectMany(g => g!["keys"]!.AsArray()).FirstOrDefault(k => k![0]!.GetValue<string>() == key)?[1]?.GetValue<string>() ?? key;

    public static (double Lo, double Hi) Limit(string name) { var l = Root["limits"]![name]!.AsArray(); return (l[0]!.GetValue<double>(), l[1]!.GetValue<double>()); }
    public static int LimitN(string name) => Root["limits"]![name]!.GetValue<int>();

    public static JsonObject? Template(string id) =>
        Root["templates"]!.AsArray().FirstOrDefault(t => t!["id"]!.GetValue<string>() == id)?["layout"]?.AsObject();
}

/// <summary>One thing a control does. Kind: none | keys | mouse | scroll | shift | gyro | radial.</summary>
internal sealed class Act
{
    public string Kind { get; }
    public string[] Codes { get; }
    public int V { get; }
    public int H { get; }
    public string? Arg { get; }
    public string Raw { get; }
    public Act(string kind, string[]? codes = null, int v = 0, int h = 0, string? arg = null, string raw = "none")
    { Kind = kind; Codes = codes ?? Array.Empty<string>(); V = v; H = h; Arg = arg; Raw = raw; }
    public static readonly Act None = new("none");
    public string Label => InputLayouts.ActionLabel(Raw);
}

internal sealed class Binding { public Act Press = Act.None; public Act? Hold; public bool Turbo; public double Threshold = 0.5; }

internal sealed class StickBinding
{
    public string Mode = "none";
    public Dictionary<string, Act> Dirs = new();
    public double Threshold = 0.5, Sensitivity = 1.0, Deadzone = 0.12, Outer = 0.95, Curve = 1.8;
    public bool InvertY;
}

/// <summary>A layout checked and normalized: what the engine runs (hearth-input's validate_layout).</summary>
internal sealed class Layout
{
    public string Name = "Custom", Base = "";
    public Dictionary<string, Binding> Buttons = new();
    public Dictionary<string, StickBinding> Sticks = new();
    public Dictionary<string, Binding> ShiftButtons = new();
    public Dictionary<string, StickBinding> ShiftSticks = new();
    public double GyroSensitivity = 1.0, GyroDeadzone = 1.0;
    public bool GyroInvertX, GyroInvertY, GyroStartOn;
    public List<Act> Radial = new();
    public int HoldMs = 300;
    public double TurboHz = 10;
}

/// <summary>Checking layouts, each game's choice (kept per game) and the UI's input.* calls. Port of input_map.rs
/// and hearth-input's validate_layout / validate_prefs.</summary>
public static class InputLayouts
{
    private static readonly string[] Triggers = { "lt", "rt" };
    private static readonly string[] Dpad = { "up", "down", "left", "right" };
    private static readonly Dictionary<string, string> MouseCodes = new()
    { ["left"] = "BTN_LEFT", ["right"] = "BTN_RIGHT", ["middle"] = "BTN_MIDDLE", ["back"] = "BTN_SIDE", ["forward"] = "BTN_EXTRA" };
    private static readonly Dictionary<string, (int V, int H)> ScrollDirs = new()
    { ["up"] = (120, 0), ["down"] = (-120, 0), ["left"] = (0, -120), ["right"] = (0, 120) };
    private static readonly Dictionary<string, HashSet<string>> Allow = new()
    {
        ["button"] = new() { "none", "keys", "mouse", "scroll", "shift", "gyro", "radial" },
        ["shift_button"] = new() { "none", "keys", "mouse", "scroll", "gyro" },
        ["simple"] = new() { "none", "keys", "mouse", "scroll" },
        ["radial"] = new() { "keys", "mouse", "scroll" },
    };
    private static readonly Dictionary<string, string[]> StickFields = new()
    {
        ["none"] = Array.Empty<string>(),
        ["keys"] = new[] { "up", "down", "left", "right", "threshold" },
        ["mouse"] = new[] { "sensitivity", "deadzone", "outer", "curve", "invert_y" },
        ["scroll"] = new[] { "sensitivity", "deadzone" },
    };

    private static string Short(string s, int n) => s.Length > n ? s[..n] : s;
    private static JsonValueKind Kind(JsonNode? n) => n is null ? JsonValueKind.Null : n.GetValueKind();

    internal static Act ParseAction(JsonNode? node, string where, string path)
    {
        string s = Kind(node) == JsonValueKind.String ? node!.GetValue<string>() : "";
        if (s.Length == 0 || s.Length > 80) throw new LayoutException($"{path}: should be an action like \"key:SPACE\"");
        Act a;
        if (s == "none") a = Act.None;
        else if (s == "shift") a = new Act("shift", raw: s);
        else if (s == "radial") a = new Act("radial", raw: s);
        else if (s is "gyro:toggle" or "gyro:hold") a = new Act("gyro", arg: s[5..], raw: s);
        else if (s.StartsWith("key:"))
        {
            var names = s[4..].Split('+');
            int max = InputTables.LimitN("combo_keys");
            if (names.Length < 1 || names.Length > max) throw new LayoutException($"{path}: a key combination has 1 to {max} keys");
            if (names.Distinct().Count() != names.Length) throw new LayoutException($"{path}: the same key twice");
            foreach (var n in names) if (!InputTables.Keys.Contains(n)) throw new LayoutException($"{path}: unknown key \"{Short(n, 20)}\"");
            a = new Act("keys", names.Select(n => "KEY_" + n).ToArray(), raw: s);
        }
        else if (s.StartsWith("mouse:") && MouseCodes.TryGetValue(s[6..], out var mc)) a = new Act("mouse", new[] { mc }, raw: s);
        else if (s.StartsWith("scroll:") && ScrollDirs.TryGetValue(s[7..], out var sd)) a = new Act("scroll", v: sd.V, h: sd.H, raw: s);
        else throw new LayoutException($"{path}: unknown action \"{Short(s, 30)}\"");
        if (!Allow[where].Contains(a.Kind)) throw new LayoutException($"{path}: \"{s}\" can't be used here");
        return a;
    }

    public static string ActionLabel(string raw)
    {
        if (raw == "none") return "Nothing";
        if (raw == "shift") return "Second set (hold)";
        if (raw == "radial") return "Radial menu";
        if (raw == "gyro:toggle") return "Gyro aiming on/off";
        if (raw == "gyro:hold") return "Gyro aiming (hold)";
        if (raw.StartsWith("key:")) return string.Join(" + ", raw[4..].Split('+').Select(InputTables.KeyLabel));
        if (raw.StartsWith("mouse:")) return InputTables.Label("mouse", raw[6..]);
        if (raw.StartsWith("scroll:")) return InputTables.Label("scroll", raw[7..]);
        return raw;
    }

    private static double Num(JsonNode? v, string lim, string path, bool integer = false)
    {
        var (lo, hi) = InputTables.Limit(lim);
        if (Kind(v) != JsonValueKind.Number) throw new LayoutException($"{path}: should be a number");
        double x = v!.GetValue<double>();
        if (double.IsNaN(x) || double.IsInfinity(x)) throw new LayoutException($"{path}: should be a number");
        if (integer && Math.Floor(x) != x) throw new LayoutException($"{path}: should be a whole number");
        if (x < lo || x > hi) throw new LayoutException($"{path}: should be between {lo} and {hi}");
        return x;
    }

    private static bool Bool(JsonNode? v, string path) => Kind(v) switch
    {
        JsonValueKind.True => true,
        JsonValueKind.False => false,
        _ => throw new LayoutException($"{path}: should be true or false"),
    };

    private static JsonObject Obj(JsonNode? v, string path, IEnumerable<string> allowed)
    {
        if (v is not JsonObject o) throw new LayoutException($"{path}: should be an object");
        var ok = allowed as ICollection<string> ?? allowed.ToList();
        foreach (var (k, _) in o) if (!ok.Contains(k)) throw new LayoutException($"{path}: unknown setting \"{Short(k, 30)}\"");
        return o;
    }

    private static Binding ParseButton(string ctl, JsonNode? v, string layer, string path)
    {
        var where = layer == "base" ? "button" : "shift_button";
        if (Kind(v) == JsonValueKind.String) v = new JsonObject { ["press"] = v!.GetValue<string>() };
        var allowed = new List<string> { "press", "hold", "turbo" };
        if (Triggers.Contains(ctl)) allowed.Add("threshold");
        var o = Obj(v, path, allowed);
        if (!o.ContainsKey("press")) throw new LayoutException($"{path}: needs \"press\"");
        var b = new Binding { Press = ParseAction(o["press"], where, path + ".press") };
        if (o.TryGetPropertyValue("hold", out var hold) && hold is not null)
        {
            b.Hold = ParseAction(hold, "simple", path + ".hold");
            if (!Allow["simple"].Contains(b.Press.Kind)) throw new LayoutException($"{path}: tap and hold work with keys, clicks and scrolling");
        }
        if (o.TryGetPropertyValue("turbo", out var turbo))
        {
            b.Turbo = Bool(turbo, path + ".turbo");
            if (b.Turbo && (b.Hold is not null || b.Press.Kind is not ("keys" or "mouse")))
                throw new LayoutException($"{path}: turbo repeats a key or a click (and can't be used with hold)");
        }
        if (o.TryGetPropertyValue("threshold", out var th)) b.Threshold = Num(th, "threshold", path + ".threshold");
        return b;
    }

    private static StickBinding ParseStick(JsonNode? v, string path)
    {
        string? mode = v is JsonObject vo && Kind(vo["mode"]) == JsonValueKind.String ? vo["mode"]!.GetValue<string>() : null;
        if (mode is null || !StickFields.ContainsKey(mode)) throw new LayoutException($"{path}: needs a mode: none, keys, mouse or scroll");
        var o = Obj(v, path, StickFields[mode].Append("mode").ToList());
        var s = new StickBinding { Mode = mode };
        JsonNode? F(string k) => o.TryGetPropertyValue(k, out var n) ? n : null;
        bool Has(string k) => o.ContainsKey(k);
        if (mode == "keys")
        {
            foreach (var d in Dpad) s.Dirs[d] = Has(d) ? ParseAction(F(d), "simple", $"{path}.{d}") : Act.None;
            s.Threshold = Has("threshold") ? Num(F("threshold"), "stick_threshold", path + ".threshold") : 0.5;
        }
        else if (mode is "mouse" or "scroll")
        {
            s.Sensitivity = Has("sensitivity") ? Num(F("sensitivity"), "sensitivity", path + ".sensitivity") : 1.0;
            s.Deadzone = Has("deadzone") ? Num(F("deadzone"), "deadzone", path + ".deadzone") : 0.12;
            if (mode == "mouse")
            {
                s.Outer = Has("outer") ? Num(F("outer"), "outer", path + ".outer") : 0.95;
                s.Curve = Has("curve") ? Num(F("curve"), "curve", path + ".curve") : 1.8;
                s.InvertY = Has("invert_y") && Bool(F("invert_y"), path + ".invert_y");
                if (s.Outer < s.Deadzone + 0.1) throw new LayoutException($"{path}: the outer dead zone must be well past the dead zone");
            }
        }
        return s;
    }

    /// <summary>A layout as Hearth stores it → the form the engine runs. Throws LayoutException with the reason.</summary>
    internal static Layout Parse(JsonNode? node)
    {
        var o = Obj(node, "layout", new[] { "version", "name", "base", "buttons", "sticks", "gyro", "shift", "radial", "hold_ms", "turbo_hz" });
        if (o.TryGetPropertyValue("version", out var ver) && !(Kind(ver) == JsonValueKind.Number && ver!.GetValue<double>() == 1))
            throw new LayoutException("layout.version: only version 1 is known");
        var L = new Layout();
        int nameLen = InputTables.LimitN("name_len");
        if (o.TryGetPropertyValue("name", out var name))
        {
            string? nm = Kind(name) == JsonValueKind.String ? name!.GetValue<string>() : null;
            if (nm is null || nm.Trim().Length == 0 || nm.Length > nameLen || nm.Any(c => c < 32))
                throw new LayoutException($"layout.name: up to {nameLen} characters");
            L.Name = nm;
        }
        if (o.TryGetPropertyValue("base", out var bas))
        {
            string? bs = Kind(bas) == JsonValueKind.String ? bas!.GetValue<string>() : null;
            if (bs is null || (bs.Length > 0 && InputTables.Template(bs) is null))
                throw new LayoutException("layout.base: unknown template");
            L.Base = bs;
        }
        var buttonIds = InputTables.Ids("buttons");
        var stickIds = InputTables.Ids("sticks");
        var buttons = o.ContainsKey("buttons") ? Obj(o["buttons"], "buttons", buttonIds) : new JsonObject();
        foreach (var ctl in buttonIds)
            L.Buttons[ctl] = buttons.ContainsKey(ctl) ? ParseButton(ctl, buttons[ctl], "base", "buttons." + ctl) : new Binding();
        var sticks = o.ContainsKey("sticks") ? Obj(o["sticks"], "sticks", stickIds) : new JsonObject();
        foreach (var st in stickIds)
            L.Sticks[st] = sticks.ContainsKey(st) ? ParseStick(sticks[st], "sticks." + st) : new StickBinding();
        if (o.ContainsKey("shift"))
        {
            var sh = Obj(o["shift"], "shift", new[] { "buttons", "sticks" });
            if (sh.ContainsKey("buttons"))
                foreach (var (ctl, v) in Obj(sh["buttons"], "shift.buttons", buttonIds)) L.ShiftButtons[ctl] = ParseButton(ctl, v, "shift", "shift.buttons." + ctl);
            if (sh.ContainsKey("sticks"))
                foreach (var (st, v) in Obj(sh["sticks"], "shift.sticks", stickIds)) L.ShiftSticks[st] = ParseStick(v, "shift.sticks." + st);
        }
        if (o.ContainsKey("gyro"))
        {
            var g = Obj(o["gyro"], "gyro", new[] { "sensitivity", "deadzone", "invert_x", "invert_y", "start_on" });
            if (g.ContainsKey("sensitivity")) L.GyroSensitivity = Num(g["sensitivity"], "gyro_sensitivity", "gyro.sensitivity");
            if (g.ContainsKey("deadzone")) L.GyroDeadzone = Num(g["deadzone"], "gyro_deadzone", "gyro.deadzone");
            if (g.ContainsKey("invert_x")) L.GyroInvertX = Bool(g["invert_x"], "gyro.invert_x");
            if (g.ContainsKey("invert_y")) L.GyroInvertY = Bool(g["invert_y"], "gyro.invert_y");
            if (g.ContainsKey("start_on")) L.GyroStartOn = Bool(g["start_on"], "gyro.start_on");
        }
        if (o.ContainsKey("radial"))
        {
            int max = InputTables.LimitN("radial_items");
            if (o["radial"] is not JsonArray r || r.Count > max) throw new LayoutException($"radial: up to {max} actions");
            for (int i = 0; i < r.Count; i++) L.Radial.Add(ParseAction(r[i], "radial", $"radial.{i}"));
        }
        if (o.ContainsKey("hold_ms")) L.HoldMs = (int)Num(o["hold_ms"], "hold_ms", "hold_ms", integer: true);
        if (o.ContainsKey("turbo_hz")) L.TurboHz = Num(o["turbo_hz"], "turbo_hz", "turbo_hz");
        return L;
    }

    /// <summary>Is this a layout Hearth Input can run? Throws LayoutException with the reason.</summary>
    public static void Validate(JsonNode? layout) => Parse(layout);

    /// <summary>Quick Access's per-game choices: {sensitivity, gyro}, with defaults.</summary>
    public static JsonObject CheckPrefs(JsonNode? p)
    {
        if (p is null) return new JsonObject { ["sensitivity"] = 1.0, ["gyro"] = false };
        var o = Obj(p, "prefs", new[] { "sensitivity", "gyro" });
        double s = o.ContainsKey("sensitivity") ? Num(o["sensitivity"], "pref_sensitivity", "prefs.sensitivity") : 1.0;
        bool g = o.ContainsKey("gyro") && Bool(o["gyro"], "prefs.gyro");
        return new JsonObject { ["sensitivity"] = s, ["gyro"] = g };
    }

    // ------------------------------------------------------------------ each game's choice

    public static string Dir => Path.Combine(Stores.HelperTools.DataDir, "input");

    /// <summary>A game id as a file name: letters, digits, - and _ as they are (and dots, but not first); anything
    /// else %XX, so no id can name another folder. Same as input_map.rs.</summary>
    public static string FileName(string id)
    {
        if (id.Length == 0 || id.Length > 200) throw new InvalidOperationException("Unknown game");
        var sb = new System.Text.StringBuilder();
        var bytes = System.Text.Encoding.UTF8.GetBytes(id);
        for (int i = 0; i < bytes.Length; i++)
        {
            byte b = bytes[i];
            bool ok = (b >= '0' && b <= '9') || (b >= 'a' && b <= 'z') || (b >= 'A' && b <= 'Z') || b == '-' || b == '_' || (b == '.' && i > 0);
            if (ok) sb.Append((char)b); else sb.Append('%').Append(b.ToString("X2"));
        }
        return sb + ".json";
    }

    /// <summary>Off, or "Strategy (RTS)" for Beacon's strategy games.</summary>
    public static string DefaultLayout(string id) => Library.Beacon.RtsByDefault(id) && InputTables.Template("rts") is not null ? "rts" : "off";

    /// <summary>A game's saved choice, checked: {layout, custom, prefs}. No file: the game's default. Broken files are Off.</summary>
    public static JsonObject Load(string id)
    {
        JsonObject Off(string layout = "off") => new() { ["layout"] = layout, ["custom"] = null, ["prefs"] = CheckPrefs(null) };
        string file;
        try { file = Path.Combine(Dir, FileName(id)); } catch { return Off(); }
        if (!File.Exists(file)) return Off(DefaultLayout(id));
        JsonObject v;
        try { v = JsonNode.Parse(File.ReadAllText(file)) as JsonObject ?? throw new LayoutException("not an object"); }
        catch { return Off(); }
        JsonNode? custom = null;
        if (v["custom"] is JsonObject c) { try { Validate(c); custom = c.DeepClone(); } catch (LayoutException) { } }
        JsonObject prefs;
        try { prefs = CheckPrefs(v["prefs"]); } catch (LayoutException) { prefs = CheckPrefs(null); }
        var want = Kind(v["layout"]) == JsonValueKind.String ? v["layout"]!.GetValue<string>() : "off";
        var layout = want == "custom" && custom is not null ? "custom" : want != "custom" && InputTables.Template(want) is not null ? want : "off";
        return new JsonObject { ["layout"] = layout, ["custom"] = custom, ["prefs"] = prefs };
    }

    /// <summary>Saves a game's choice ("off", a template id or "custom", with the custom layout or the one already
    /// saved); prefs are merged into what's saved. Returns the saved choice.</summary>
    public static JsonObject Save(string id, string? layout, JsonNode? custom, JsonNode? prefs)
    {
        var name = FileName(id);
        var cur = Load(id);
        JsonNode? keep;
        if (custom is not null) { Validate(custom); keep = custom.DeepClone(); }
        else keep = cur["custom"]?.DeepClone();
        var lay = layout ?? cur["layout"]!.GetValue<string>();
        if (lay == "custom" && keep is null) throw new InvalidOperationException("There's no custom layout to use yet");
        if (lay != "off" && lay != "custom" && InputTables.Template(lay) is null) throw new InvalidOperationException("Unknown layout");
        var p = cur["prefs"]!.AsObject().DeepClone().AsObject();
        if (prefs is not null)
            foreach (var (k, v) in Obj(prefs, "prefs", new[] { "sensitivity", "gyro" })) p[k] = v?.DeepClone();
        var checkedPrefs = CheckPrefs(p);
        var doc = new JsonObject { ["version"] = 1, ["layout"] = lay, ["custom"] = keep, ["prefs"] = checkedPrefs };
        Directory.CreateDirectory(Dir);
        var file = Path.Combine(Dir, name);
        File.WriteAllText(file + ".tmp", doc.ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
        File.Move(file + ".tmp", file, overwrite: true);
        return Load(id);
    }

    /// <summary>The layout a choice runs (null: Off).</summary>
    public static JsonObject? Resolve(JsonObject choice)
    {
        var l = choice["layout"]?.GetValue<string>();
        return l switch
        {
            null or "off" => null,
            "custom" => choice["custom"] as JsonObject,
            _ => InputTables.Template(l),
        };
    }

    // ------------------------------------------------------------------ the UI's calls

    private static JsonNode? Node(JsonElement a, string k) =>
        a.ValueKind == JsonValueKind.Object && a.TryGetProperty(k, out var v) && v.ValueKind != JsonValueKind.Null ? JsonNode.Parse(v.GetRawText()) : null;

    /// <summary>input.tables, input.get, input.set, input.test, input.status. running: the game playing now.</summary>
    public static object Call(string method, JsonElement a, string? running, HearthInput mapper)
    {
        string GameArg()
        {
            var g = a.ValueKind == JsonValueKind.Object && a.TryGetProperty("game", out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() : null;
            return !string.IsNullOrEmpty(g) ? g : running ?? throw new InvalidOperationException("No game");
        }
        try
        {
            switch (method)
            {
                case "input.tables":
                {
                    var t = InputTables.Root.DeepClone().AsObject();
                    t.Remove("_about");
                    return t;
                }
                case "input.get":
                {
                    var id = GameArg();
                    var c = Load(id);
                    c["game"] = id;
                    c["running"] = running == id;
                    c["resolved"] = Resolve(c)?.DeepClone();
                    return c;
                }
                case "input.set":
                {
                    var id = GameArg();
                    var layout = a.ValueKind == JsonValueKind.Object && a.TryGetProperty("layout", out var lv) && lv.ValueKind == JsonValueKind.String ? lv.GetString() : null;
                    Save(id, layout, Node(a, "custom"), Node(a, "prefs"));
                    mapper.Forget();
                    var c = Load(id);
                    c["game"] = id; c["running"] = running == id; c["resolved"] = Resolve(c)?.DeepClone();
                    return c;
                }
                case "input.test":
                {
                    if (a.ValueKind == JsonValueKind.Object && a.TryGetProperty("stop", out var st) && st.ValueKind == JsonValueKind.True) { mapper.StopTest(); return new { ok = true }; }
                    var layout = Node(a, "layout") as JsonObject ?? throw new InvalidOperationException("No layout to try");
                    var parsed = Parse(layout);
                    int secs = a.ValueKind == JsonValueKind.Object && a.TryGetProperty("seconds", out var sv) && sv.ValueKind == JsonValueKind.Number ? (int)Math.Round(sv.GetDouble()) : 20;
                    secs = Math.Clamp(secs, 1, 60);
                    mapper.StartTest(parsed, secs);
                    return new { ok = true, seconds = secs };
                }
                case "input.status": return mapper.Status();
                default: throw new InvalidOperationException($"{method} isn't known");
            }
        }
        catch (LayoutException e) { throw new InvalidOperationException(e.Message); }
    }
}

/// <summary>Where the engine's presses go.</summary>
internal interface IInputOut
{
    void Key(string code, bool down);
    void Move(int dx, int dy);
    void Wheel(int v, int h);
    void Osd(string text);
}

/// <summary>Runs one layout: hearth-input's Engine, ported. Time is passed in (seconds).</summary>
internal sealed class Engine
{
    private const double Pulse = 0.06, ScrollFirst = 0.35, ScrollEvery = 0.09;
    private const double MouseSpeed = 1800.0, ScrollSpeed = 10.0, GyroScale = 12.0, TriggerHyst = 0.05, StickHyst = 0.8;
    private static readonly double Diag = Math.Sin(22.5 * Math.PI / 180);
    private static readonly string[] Triggers = { "lt", "rt" };
    private static readonly string[] Dpad = { "up", "down", "left", "right" };
    private static readonly Dictionary<string, (int X, int Y)> DpadVectors = new() { ["up"] = (0, -1), ["right"] = (1, 0), ["down"] = (0, 1), ["left"] = (-1, 0) };

    private sealed class Held { public string Kind = ""; public double T; public Binding? B; public string Phase = ""; public Act? A; public bool On; public double Next; }
    private sealed class RadialState { public string By = ""; public int Sel = -1; }

    private readonly IInputOut _out;
    public Layout L { get; }
    public double Sensitivity { get; private set; }
    public bool PrefGyro { get; private set; }
    private readonly Dictionary<string, HashSet<string>> _refs = new();
    private readonly Dictionary<string, List<string>> _srcCodes = new();
    private readonly Dictionary<string, Held> _held = new();
    private HashSet<string> _ignored = new();
    private readonly List<(double T, string Src)> _pulses = new();
    private readonly Dictionary<string, (double T, Act A)> _scrollNext = new();
    private readonly HashSet<string> _shiftBy = new();
    private bool _gyroOn;
    private readonly HashSet<string> _gyroHold = new();
    private RadialState? _radial;
    private double _lx, _ly, _rx, _ry, _lt, _rt;
    private readonly Dictionary<string, bool> _trig = new() { ["lt"] = false, ["rt"] = false };
    private readonly Dictionary<(string S, string D), Act> _stickHeld = new();
    private readonly double[] _acc = new double[4];

    public Engine(IInputOut output, Layout layout, double sensitivity, bool gyro)
    {
        _out = output; L = layout; Sensitivity = sensitivity; PrefGyro = gyro;
        _gyroOn = L.GyroStartOn || gyro;
    }

    // ------------------------------------------------------------------ outputs
    private void Down(string src, string[] codes)
    {
        foreach (var c in codes)
        {
            if (!_refs.TryGetValue(c, out var s)) _refs[c] = s = new HashSet<string>();
            if (s.Count == 0) _out.Key(c, true);
            s.Add(src);
        }
        if (!_srcCodes.TryGetValue(src, out var l)) _srcCodes[src] = l = new List<string>();
        l.AddRange(codes);
    }

    private void Up(string src)
    {
        if (_srcCodes.Remove(src, out var codes))
            for (int i = codes.Count - 1; i >= 0; i--)
            {
                var c = codes[i];
                if (_refs.TryGetValue(c, out var s) && s.Remove(src) && s.Count == 0) _out.Key(c, false);
            }
        _scrollNext.Remove(src);
    }

    private void Press(string src, Act act, double now)
    {
        if (act.Kind is "keys" or "mouse") Down(src, act.Codes);
        else if (act.Kind == "scroll") { _out.Wheel(act.V, act.H); _scrollNext[src] = (now + ScrollFirst, act); }
    }

    private void Tap(string src, Act act, double now)
    {
        Press(src, act, now);
        _scrollNext.Remove(src);
        _pulses.Add((now + Pulse, src));
    }

    // ------------------------------------------------------------------ layers
    private bool Shifted => _shiftBy.Count > 0;
    private Binding BindingOf(string ctl) => Shifted && L.ShiftButtons.TryGetValue(ctl, out var b) ? b : L.Buttons[ctl];
    private StickBinding StickOf(string s) => Shifted && L.ShiftSticks.TryGetValue(s, out var b) ? b : L.Sticks[s];
    public bool GyroActive => _gyroOn || _gyroHold.Count > 0;

    // ------------------------------------------------------------------ input
    public void Button(string name, bool down, double now)
    {
        if (!L.Buttons.ContainsKey(name)) return;
        if (down)
        {
            if (_ignored.Contains(name) || _held.ContainsKey(name)) return;
            if (_radial is not null && Dpad.Contains(name))
            {
                var (x, y) = DpadVectors[name];
                RadialPick(x, y);
                _held[name] = new Held { Kind = "radialnav" };
                return;
            }
            var b = BindingOf(name);
            var act = b.Press;
            if (act.Kind == "shift") { _shiftBy.Add(name); _held[name] = new Held { Kind = "shift" }; }
            else if (act.Kind == "gyro")
            {
                if (act.Arg == "toggle") { _gyroOn = !_gyroOn; _out.Osd("Gyro aiming " + (GyroActive ? "on" : "off")); }
                else _gyroHold.Add(name);
                _held[name] = new Held { Kind = "gyro" };
            }
            else if (act.Kind == "radial")
            {
                _held[name] = new Held { Kind = "radial" };
                if (L.Radial.Count > 0 && _radial is null)
                {
                    _radial = new RadialState { By = name, Sel = -1 };
                    ReleaseStick("rs");
                    _out.Osd("Radial menu: " + string.Join(" · ", L.Radial.Select(a => a.Label)));
                }
            }
            else if (b.Hold is not null) _held[name] = new Held { Kind = "holdtap", T = now, B = b, Phase = "wait" };
            else if (b.Turbo)
            {
                Press(name, act, now);
                _held[name] = new Held { Kind = "turbo", A = act, On = true, Next = now + TurboHalf };
            }
            else { Press(name, act, now); _held[name] = new Held { Kind = "plain" }; }
            return;
        }
        if (_ignored.Remove(name)) return;
        if (!_held.Remove(name, out var h)) return;
        switch (h.Kind)
        {
            case "shift": _shiftBy.Remove(name); break;
            case "gyro": _gyroHold.Remove(name); break;
            case "radial":
                var r = _radial;
                if (r is not null && r.By == name)
                {
                    _radial = null;
                    if (r.Sel >= 0)
                    {
                        var act = L.Radial[r.Sel];
                        Tap("radial", act, now);
                        _out.Osd(act.Label);
                    }
                }
                break;
            case "holdtap":
                if (h.Phase == "wait") Tap(name + "#tap", h.B!.Press, now);
                else Up(name + "#hold");
                break;
            case "turbo" or "plain": Up(name); break;
        }
    }

    private double TurboHalf => 0.5 / L.TurboHz;

    private static int RadialIndex(double x, double y, int n)
    {
        var ang = Math.Atan2(x, -y) % (2 * Math.PI);
        if (ang < 0) ang += 2 * Math.PI;
        return (int)Math.Round(ang / (2 * Math.PI / n)) % n;
    }

    private void RadialPick(double x, double y)
    {
        var r = _radial!;
        int sel = RadialIndex(x, y, L.Radial.Count);
        if (sel != r.Sel) { r.Sel = sel; _out.Osd("Radial menu: " + L.Radial[sel].Label); }
    }

    public void SetAxes(double now, double lx, double ly, double rx, double ry, double lt, double rt)
    {
        _lx = lx; _ly = ly; _rx = rx; _ry = ry; _lt = lt; _rt = rt;
        foreach (var t in Triggers)
        {
            double thr = BindingOf(t).Threshold;
            double v = t == "lt" ? _lt : _rt;
            bool pressed = _trig[t] ? v > thr - TriggerHyst : v >= thr;
            if (pressed != _trig[t]) { _trig[t] = pressed; Button(t, pressed, now); }
        }
    }

    private void ReleaseStick(string s)
    {
        foreach (var key in _stickHeld.Keys.Where(k => k.S == s).ToList())
        {
            Up($"{key.S}:{key.D}");
            _stickHeld.Remove(key);
        }
    }

    private static HashSet<string> StickDirections(double x, double y, double threshold, bool held)
    {
        var mag = Math.Sqrt(x * x + y * y);
        var limit = held ? threshold * StickHyst : threshold;
        var outSet = new HashSet<string>();
        if (mag < limit || mag == 0) return outSet;
        double nx = x / mag, ny = y / mag;
        if (nx > Diag) outSet.Add("right"); else if (nx < -Diag) outSet.Add("left");
        if (ny > Diag) outSet.Add("down"); else if (ny < -Diag) outSet.Add("up");
        return outSet;
    }

    private static double StickCurve(double mag, double deadzone, double outer, double exponent)
    {
        if (mag <= deadzone) return 0.0;
        double t = Math.Min(1.0, (mag - deadzone) / Math.Max(1e-6, outer - deadzone));
        return Math.Pow(t, exponent);
    }

    public void Tick(double now, double dt)
    {
        foreach (var p in _pulses.Where(p => p.T <= now).ToList()) { _pulses.Remove(p); Up(p.Src); }
        foreach (var (name, h) in _held.ToList())
        {
            if (h.Kind == "holdtap" && h.Phase == "wait" && now - h.T >= L.HoldMs / 1000.0)
            {
                h.Phase = "held";
                Press(name + "#hold", h.B!.Hold!, now);
            }
            else if (h.Kind == "turbo" && now >= h.Next)
            {
                h.On = !h.On;
                h.Next = now + TurboHalf;
                if (h.On) Press(name, h.A!, now); else Up(name);
            }
        }
        foreach (var (src, (t, act)) in _scrollNext.ToList())
            if (now >= t) { _out.Wheel(act.V, act.H); _scrollNext[src] = (now + ScrollEvery, act); }

        double sens = Sensitivity;
        foreach (var (s, x, y) in new[] { ("ls", _lx, _ly), ("rs", _rx, _ry) })
        {
            if (_radial is not null && s == "rs")
            {
                if (Math.Sqrt(x * x + y * y) > 0.5) RadialPick(x, y);
                continue;
            }
            var b = StickOf(s);
            if (b.Mode != "keys") ReleaseStick(s);
            if (b.Mode == "keys")
            {
                bool anyHeld = _stickHeld.Keys.Any(k => k.S == s);
                var want = StickDirections(x, y, b.Threshold, anyHeld);
                foreach (var d in Dpad)
                {
                    Act? act = want.Contains(d) ? b.Dirs[d] : null;
                    Act? cur = _stickHeld.TryGetValue((s, d), out var c0) ? c0 : null;
                    if (cur is not null && (act is null || cur.Raw != act.Raw))
                    {
                        Up($"{s}:{d}");
                        _stickHeld.Remove((s, d));
                        cur = null;
                    }
                    if (act is not null && cur is null)
                    {
                        _stickHeld[(s, d)] = act;
                        Press($"{s}:{d}", act, now);
                    }
                }
            }
            else if (b.Mode == "mouse")
            {
                double mag = Math.Sqrt(x * x + y * y);
                double f = StickCurve(mag, b.Deadzone, b.Outer, b.Curve);
                if (f > 0)
                {
                    double speed = MouseSpeed * b.Sensitivity * sens * f * dt;
                    _acc[0] += x / mag * speed;
                    _acc[1] += y / mag * speed * (b.InvertY ? -1 : 1);
                }
            }
            else if (b.Mode == "scroll")
            {
                double mag = Math.Sqrt(x * x + y * y);
                double f = StickCurve(mag, b.Deadzone, 1.0, 1.5);
                if (f > 0)
                {
                    double rate = ScrollSpeed * 120 * b.Sensitivity * f * dt;
                    _acc[2] += -y / mag * rate;
                    _acc[3] += x / mag * rate;
                }
            }
        }
        // (No motion sensors through XInput: gyro aiming has nothing to read on Windows.)
        int dx = (int)_acc[0], dy = (int)_acc[1];
        _acc[0] -= dx; _acc[1] -= dy;
        if (dx != 0 || dy != 0) _out.Move(dx, dy);
        int v = (int)_acc[2], hw = (int)_acc[3];
        _acc[2] -= v; _acc[3] -= hw;
        if (v != 0 || hw != 0) _out.Wheel(v, hw);
    }

    // ------------------------------------------------------------------ starting and stopping
    /// <summary>Every key and button up, nothing half-done (no tap fires).</summary>
    public void ReleaseAll()
    {
        foreach (var src in _srcCodes.Keys.ToList()) Up(src);
        _held.Clear(); _pulses.Clear(); _scrollNext.Clear(); _stickHeld.Clear(); _shiftBy.Clear(); _gyroHold.Clear();
        _radial = null;
        _trig["lt"] = false; _trig["rt"] = false;
        Array.Clear(_acc);
    }

    /// <summary>The layout takes over: buttons already down (the A that started the game) do nothing until let go.</summary>
    public void Start(IEnumerable<string> heldButtons, double lt, double rt)
    {
        ReleaseAll();
        _ignored = new HashSet<string>(heldButtons);
        if (lt >= 0.5) { _ignored.Add("lt"); _trig["lt"] = true; }
        if (rt >= 0.5) { _ignored.Add("rt"); _trig["rt"] = true; }
    }

    public void SetPrefs(double sensitivity, bool gyro)
    {
        Sensitivity = sensitivity;
        if (gyro != PrefGyro) _gyroOn = gyro;
        PrefGyro = gyro;
    }
}

/// <summary>Keys and the mouse through SendInput. Nothing but key releases goes to Ember's own windows (except
/// while trying a layout on its page).</summary>
internal sealed class SendInputOut : IInputOut
{
    public volatile bool AllowEmber;
    public event Action<string>? OsdText;

    // Set 1 scan codes (the key's place on a US keyboard, like Linux's KEY_* names); e0 = extended.
    private static readonly Dictionary<string, (ushort Scan, bool Ext)> Scan = BuildScan();
    private static Dictionary<string, (ushort, bool)> BuildScan()
    {
        var d = new Dictionary<string, (ushort, bool)>();
        void S(string k, ushort sc, bool e = false) => d["KEY_" + k] = (sc, e);
        S("ESC", 0x01); S("1", 0x02); S("2", 0x03); S("3", 0x04); S("4", 0x05); S("5", 0x06); S("6", 0x07); S("7", 0x08); S("8", 0x09); S("9", 0x0A); S("0", 0x0B);
        S("MINUS", 0x0C); S("EQUAL", 0x0D); S("BACKSPACE", 0x0E); S("TAB", 0x0F);
        S("Q", 0x10); S("W", 0x11); S("E", 0x12); S("R", 0x13); S("T", 0x14); S("Y", 0x15); S("U", 0x16); S("I", 0x17); S("O", 0x18); S("P", 0x19);
        S("LEFTBRACE", 0x1A); S("RIGHTBRACE", 0x1B); S("ENTER", 0x1C); S("LEFTCTRL", 0x1D);
        S("A", 0x1E); S("S", 0x1F); S("D", 0x20); S("F", 0x21); S("G", 0x22); S("H", 0x23); S("J", 0x24); S("K", 0x25); S("L", 0x26);
        S("SEMICOLON", 0x27); S("APOSTROPHE", 0x28); S("GRAVE", 0x29); S("LEFTSHIFT", 0x2A); S("BACKSLASH", 0x2B);
        S("Z", 0x2C); S("X", 0x2D); S("C", 0x2E); S("V", 0x2F); S("B", 0x30); S("N", 0x31); S("M", 0x32);
        S("COMMA", 0x33); S("DOT", 0x34); S("SLASH", 0x35); S("RIGHTSHIFT", 0x36); S("KPASTERISK", 0x37); S("LEFTALT", 0x38); S("SPACE", 0x39); S("CAPSLOCK", 0x3A);
        S("F1", 0x3B); S("F2", 0x3C); S("F3", 0x3D); S("F4", 0x3E); S("F5", 0x3F); S("F6", 0x40); S("F7", 0x41); S("F8", 0x42); S("F9", 0x43); S("F10", 0x44);
        S("SCROLLLOCK", 0x46);
        S("KP7", 0x47); S("KP8", 0x48); S("KP9", 0x49); S("KPMINUS", 0x4A); S("KP4", 0x4B); S("KP5", 0x4C); S("KP6", 0x4D); S("KPPLUS", 0x4E);
        S("KP1", 0x4F); S("KP2", 0x50); S("KP3", 0x51); S("KP0", 0x52); S("KPDOT", 0x53); S("F11", 0x57); S("F12", 0x58);
        S("KPENTER", 0x1C, true); S("RIGHTCTRL", 0x1D, true); S("KPSLASH", 0x35, true); S("SYSRQ", 0x37, true); S("RIGHTALT", 0x38, true);
        S("HOME", 0x47, true); S("UP", 0x48, true); S("PAGEUP", 0x49, true); S("LEFT", 0x4B, true); S("RIGHT", 0x4D, true);
        S("END", 0x4F, true); S("DOWN", 0x50, true); S("PAGEDOWN", 0x51, true); S("INSERT", 0x52, true); S("DELETE", 0x53, true);
        S("LEFTMETA", 0x5B, true); S("COMPOSE", 0x5D, true);
        return d;
    }
    // Keys whose scan codes Windows reads differently: sent as virtual keys.
    private static readonly Dictionary<string, (ushort Vk, bool Ext)> ByVk = new() { ["KEY_PAUSE"] = (0x13, false), ["KEY_NUMLOCK"] = (0x90, true) };

    private bool EmberInFront()
    {
        var fg = GetForegroundWindow();
        if (fg == IntPtr.Zero) return false;
        GetWindowThreadProcessId(fg, out var pid);
        return pid == (uint)Environment.ProcessId;
    }
    private bool Blocked() => !AllowEmber && EmberInFront();

    public void Key(string code, bool down)
    {
        if (down && Blocked()) return;   // releases always go out, so nothing stays held
        var input = new INPUT[1];
        if (code.StartsWith("BTN_"))
        {
            uint flags; uint data = 0;
            switch (code)
            {
                case "BTN_LEFT": flags = down ? 0x0002u : 0x0004u; break;
                case "BTN_RIGHT": flags = down ? 0x0008u : 0x0010u; break;
                case "BTN_MIDDLE": flags = down ? 0x0020u : 0x0040u; break;
                case "BTN_SIDE": flags = down ? 0x0080u : 0x0100u; data = 0x0001; break;    // XBUTTON1: back
                case "BTN_EXTRA": flags = down ? 0x0080u : 0x0100u; data = 0x0002; break;   // XBUTTON2: forward
                default: return;
            }
            input[0].type = INPUT_MOUSE;
            input[0].u.mi = new MOUSEINPUT { dwFlags = flags, mouseData = data, dwExtraInfo = Marker };
        }
        else if (ByVk.TryGetValue(code, out var vk))
        {
            input[0].type = INPUT_KEYBOARD;
            input[0].u.ki = new KEYBDINPUT { wVk = vk.Vk, dwFlags = (vk.Ext ? KEYEVENTF_EXTENDEDKEY : 0) | (down ? 0 : KEYEVENTF_KEYUP), dwExtraInfo = Marker };
        }
        else if (Scan.TryGetValue(code, out var sc))
        {
            input[0].type = INPUT_KEYBOARD;
            input[0].u.ki = new KEYBDINPUT { wScan = sc.Scan, dwFlags = KEYEVENTF_SCANCODE | (sc.Ext ? KEYEVENTF_EXTENDEDKEY : 0) | (down ? 0 : KEYEVENTF_KEYUP), dwExtraInfo = Marker };
        }
        else return;
        SendInput(1, input, Marshal.SizeOf<INPUT>());
    }

    public void Move(int dx, int dy)
    {
        if (Blocked()) return;
        var input = new INPUT[1];
        input[0].type = INPUT_MOUSE;
        input[0].u.mi = new MOUSEINPUT { dx = dx, dy = dy, dwFlags = 0x0001 /* MOVE (relative) */, dwExtraInfo = Marker };
        SendInput(1, input, Marshal.SizeOf<INPUT>());
    }

    public void Wheel(int v, int h)
    {
        if (Blocked()) return;
        var list = new List<INPUT>();
        // Windows takes partial notches (120 = one), positive = up / right, like Linux's hi-res wheel.
        if (v != 0) list.Add(new INPUT { type = INPUT_MOUSE, u = new InputUnion { mi = new MOUSEINPUT { dwFlags = 0x0800 /* WHEEL */, mouseData = unchecked((uint)v), dwExtraInfo = Marker } } });
        if (h != 0) list.Add(new INPUT { type = INPUT_MOUSE, u = new InputUnion { mi = new MOUSEINPUT { dwFlags = 0x1000 /* HWHEEL */, mouseData = unchecked((uint)h), dwExtraInfo = Marker } } });
        if (list.Count > 0) SendInput((uint)list.Count, list.ToArray(), Marshal.SizeOf<INPUT>());
    }

    public void Osd(string text) => OsdText?.Invoke(text);

    /// <summary>On every event Ember sends, so they can be told apart ("HRTH").</summary>
    private static readonly IntPtr Marker = new(0x48525448);
    private const uint INPUT_MOUSE = 0, INPUT_KEYBOARD = 1;
    private const uint KEYEVENTF_EXTENDEDKEY = 0x0001, KEYEVENTF_KEYUP = 0x0002, KEYEVENTF_SCANCODE = 0x0008;

    [StructLayout(LayoutKind.Sequential)]
    private struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Sequential)]
    private struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Sequential)]
    private struct HARDWAREINPUT { public uint uMsg; public ushort wParamL; public ushort wParamH; }
    [StructLayout(LayoutKind.Explicit)]
    private struct InputUnion { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; [FieldOffset(0)] public HARDWAREINPUT hi; }
    [StructLayout(LayoutKind.Sequential)]
    private struct INPUT { public uint type; public InputUnion u; }

    [DllImport("user32.dll", SetLastError = true)] private static extern uint SendInput(uint nInputs, INPUT[] pInputs, int cbSize);
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
}

/// <summary>
/// Hearth Input's runtime on Windows (hearth-input's Daemon and Policy): which layout runs right now, fed with the
/// controller every 10 ms by ControllerBridge. A layout runs while the running game's window is in front and it has
/// one (or while one is being tried); it lets go, every key released, the moment that stops, the overlay or Ember
/// comes in front, or the Guide button is pressed (then for 1.5 s, while Quick Access opens).
/// </summary>
public sealed class HearthInput
{
    private const double GuideSuspend = 1.5;
    private readonly object _lock = new();
    private readonly Stopwatch _clock = Stopwatch.StartNew();
    private readonly SendInputOut _out = new();

    /// <summary>The game playing now, and its folders (set by SystemBridge).</summary>
    public Func<string?> RunningId { get; set; } = () => null;
    public Func<string[]> RunningDirs { get; set; } = Array.Empty<string>;
    /// <summary>Connected XInput controllers.</summary>
    public Func<int> Pads { get; set; } = () => 0;
    /// <summary>A short message for the screen (gyro, the radial menu), raised on the controller thread.</summary>
    public event Action<string>? OsdText { add => _out.OsdText += value; remove => _out.OsdText -= value; }

    private Engine? _engine;
    private Layout? _running;
    private string _runningName = "";
    private Layout? _test;
    private double _testUntil;
    private double _suspendUntil;
    private double _last;
    private HashSet<string> _pressed = new();
    private bool _lastFront, _lastOverlay;
    private string? _error;

    // The running game's layout, read from its file at most every second (the same layout stays the same object).
    private string? _cacheGame;
    private string _cacheRaw = "";
    private Layout? _cacheLayout;
    private (double S, bool G) _cachePrefs = (1.0, false);
    private double _cacheAt = -10;

    // Is the window in front the game's? Asked at most every half second per window.
    private IntPtr _fgWin;
    private bool _fgIsGame;
    private double _fgAt = -10;

    private double Now => _clock.Elapsed.TotalSeconds;

    /// <summary>input.set changed a game's choice: read it again now.</summary>
    public void Forget() { lock (_lock) _cacheAt = -10; }

    public void StartTest(object layout, int seconds)
    {
        if (layout is not Layout l) throw new ArgumentException("No layout to try");
        lock (_lock) { _test = l; _testUntil = Now + Math.Clamp(seconds, 1, 60); }
    }
    internal void StartTest(Layout l, int seconds) => StartTest((object)l, seconds);
    public void StopTest() { lock (_lock) { _test = null; _testUntil = 0; } }

    /// <summary>Is this window one of the running game's (its process's exe is in the game's folders)?</summary>
    public bool IsGameWindow(IntPtr fg)
    {
        lock (_lock)
        {
            double now = Now;
            if (fg == _fgWin && now - _fgAt < 0.5) return _fgIsGame;
            _fgWin = fg; _fgAt = now;
            _fgIsGame = false;
            if (fg == IntPtr.Zero) return false;
            GetWindowThreadProcessId(fg, out var pid);
            if (pid == 0 || pid == (uint)Environment.ProcessId) return false;
            _fgIsGame = Library.GameMonitor.IsUnder((int)pid, RunningDirs());
            return _fgIsGame;
        }
    }

    private void Refresh(string? game, double now)
    {
        if (game == _cacheGame && now - _cacheAt < 1.0) return;
        _cacheAt = now;
        if (game is null) { _cacheGame = null; _cacheLayout = null; _cacheRaw = ""; return; }
        try
        {
            var choice = InputLayouts.Load(game);
            var resolved = InputLayouts.Resolve(choice);
            var raw = resolved?.ToJsonString() ?? "";
            var p = choice["prefs"]!;
            _cachePrefs = (p["sensitivity"]!.GetValue<double>(), p["gyro"]!.GetValue<bool>());
            if (game != _cacheGame || raw != _cacheRaw)
            {
                _cacheGame = game; _cacheRaw = raw;
                _cacheLayout = resolved is null ? null : InputLayouts.Parse(resolved);
                _error = null;
            }
        }
        catch (Exception ex) { _cacheGame = game; _cacheRaw = ""; _cacheLayout = null; _error = ex.Message; }
    }

    private static readonly (ushort Bit, string Name)[] Bits =
    {
        (0x1000, "a"), (0x2000, "b"), (0x4000, "x"), (0x8000, "y"), (0x0100, "lb"), (0x0200, "rb"), (0x0020, "view"), (0x0010, "menu"),
        (0x0040, "l3"), (0x0080, "r3"), (0x0001, "up"), (0x0002, "down"), (0x0004, "left"), (0x0008, "right"),
    };

    /// <summary>One moment of the controller (all XInput pads merged). gameFront: the running game's window is in
    /// front; emberFront: Ember's main window is; overlayFront: the overlay is. Returns true while a layout runs (the
    /// controller is the layout's: the bridge does nothing else with it).</summary>
    public bool Feed(ushort buttons, byte lt, byte rt, short lx, short ly, short rx, short ry, bool gameFront, bool emberFront, bool overlayFront, bool guidePressed)
    {
        lock (_lock)
        {
            double now = Now;
            if (guidePressed && (_engine is not null || _test is not null)) { _suspendUntil = now + GuideSuspend; _test = null; }
            _lastFront = gameFront; _lastOverlay = overlayFront;

            Layout? want = null;
            (double S, bool G) prefs = (1.0, false);
            bool testing = false;
            if (!overlayFront && now >= _suspendUntil)
            {
                if (_test is not null && now < _testUntil) { want = _test; testing = true; }
                else
                {
                    if (_test is not null) _test = null;
                    string? game = RunningId();
                    Refresh(game, now);
                    if (gameFront && game is not null && _cacheLayout is not null) { want = _cacheLayout; prefs = _cachePrefs; }
                }
            }
            _out.AllowEmber = testing && emberFront;

            if (!ReferenceEquals(want, _running) || (want is not null && _engine is null))
            {
                if (_engine is not null) { _engine.ReleaseAll(); _engine = null; SystemBridge.Log("input: layout off"); }
                _running = want;
                if (want is not null)
                {
                    _engine = new Engine(_out, want, prefs.S, prefs.G);
                    var held = Bits.Where(b => (buttons & b.Bit) != 0).Select(b => b.Name).ToList();
                    _engine.Start(held, lt / 255.0, rt / 255.0);
                    _pressed = new HashSet<string>(held);
                    _last = now;
                    _runningName = want.Name;
                    SystemBridge.Log($"input: layout on: {want.Name}{(testing ? " (trying it)" : "")}");
                }
            }
            else if (_engine is not null && !testing && (_engine.Sensitivity != prefs.S || _engine.PrefGyro != prefs.G))
                _engine.SetPrefs(prefs.S, prefs.G);

            if (_engine is null) { _pressed.Clear(); _last = now; return false; }

            var state = new HashSet<string>(Bits.Where(b => (buttons & b.Bit) != 0).Select(b => b.Name));
            foreach (var b in state.Union(_pressed).Where(n => state.Contains(n) != _pressed.Contains(n)).OrderBy(n => n, StringComparer.Ordinal).ToList())
                _engine.Button(b, state.Contains(b), now);
            _pressed = state;
            static double N(short v) => Math.Clamp(v / 32767.0, -1.0, 1.0);
            // XInput's sticks point up for positive Y; the engine (like evdev) uses down.
            _engine.SetAxes(now, N(lx), -N(ly), N(rx), -N(ry), lt / 255.0, rt / 255.0);
            _engine.Tick(now, Math.Min(0.05, now - _last));
            _last = now;
            return true;
        }
    }

    /// <summary>Lets everything go (Ember closing).</summary>
    public void Release() { lock (_lock) { _engine?.ReleaseAll(); _engine = null; _running = null; } }

    public static bool HidHideInstalled()
    {
        try
        {
            if (File.Exists(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), "Nefarius Software Solutions", "HidHide", "x64", "HidHideCLI.exe"))) return true;
            using var k = Microsoft.Win32.Registry.LocalMachine.OpenSubKey(@"SYSTEM\CurrentControlSet\Services\HidHide");
            return k is not null;
        }
        catch { return false; }
    }

    /// <summary>input.status: like hearth-input's status, plus what Windows can't do.</summary>
    public object Status()
    {
        lock (_lock)
        {
            double now = Now;
            int pads = Pads();
            bool hid = HidHideInstalled();
            return new
            {
                ok = true,
                active = _engine is not null,
                layout = _engine is not null ? _runningName : null,
                game = RunningId(),
                front = _lastFront,
                overlay = _lastOverlay,
                test = _test is not null && now < _testUntil ? (int)Math.Round(_testUntil - now) : 0,
                connected = true,
                devices = Enumerable.Range(0, pads).Select(i => new { name = $"Controller {i + 1} (XInput)", grabbed = false, busy = false }).ToList(),
                gyro = new { available = false, device = (string?)null, on = false },
                error = _error,
                hidhide = hid,
                caveat = hid
                    ? "HidHide is installed: in HidHide's settings, hide the controller and allow Ember, so games with a layout see only the keyboard and mouse."
                    : "Windows can't hide the controller from the game, so a game that reads controllers sees both. Use layouts for games without controller support.",
            };
        }
    }

    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
}
