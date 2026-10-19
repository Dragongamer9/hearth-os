using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Principal;
using System.Text;
using Microsoft.Win32;
using Nimbus.Host.Mantel;

namespace Nimbus.Host.Services;

/// <summary>
/// Hearth Task Manager on Windows (mantel-tasks.html), the same three pages as Mantel's on Hearth OS:
///   Apps        running apps (a process with a window, plus the processes it started), then background
///               processes grouped by name; CPU and memory; End task (asks the window to close) and Force end
///               (kills the process tree); Windows' and Ember's own critical processes are protected.
///   Performance CPU (total and per core), memory, GPU (Windows' "GPU Engine" counters, like Task Manager),
///               temperatures if Windows can read them without a driver, and the battery.
///   Startup     the apps Windows starts at sign-in (Run keys and Startup folders) with the same on/off switch
///               Task Manager uses (the StartupApproved entries).
/// Process data comes from one NtQuerySystemInformation call (no handle to each process needed), so it works
/// for elevated and protected processes too.
/// </summary>
public sealed class TaskService
{
    private readonly object _lock = new();
    private readonly PerfService _gpu = new();
    private Dictionary<long, long> _prevCpu = new();          // (pid, create time) key → CPU time (100 ns)
    private long _prevStamp;                                   // Stopwatch timestamp of the last sample
    private long[] _prevIdle = Array.Empty<long>(), _prevTotal = Array.Empty<long>();
    private readonly Dictionary<string, string?> _iconByPath = new(StringComparer.OrdinalIgnoreCase);
    private readonly Dictionary<string, string> _nameByPath = new(StringComparer.OrdinalIgnoreCase);
    private readonly Dictionary<long, string?> _pathByProc = new();
    private object? _temps;
    private DateTime _tempsAt = DateTime.MinValue;
    private object? _batteryWmi;
    private DateTime _batteryAt = DateTime.MinValue;

    private sealed record Proc(int Pid, int Parent, string Name, long CpuTime, long Create, long Memory, int Threads, int Handles)
    {
        public long Key => ((long)Pid << 40) ^ Create;
    }

    // Never offered for End task: Windows can't run without them, or they're Ember (and its WebView2 pages).
    private static readonly HashSet<string> Critical = new(StringComparer.OrdinalIgnoreCase)
    {
        "System", "System Idle Process", "Idle", "Registry", "Secure System", "Memory Compression", "smss", "csrss", "wininit",
        "winlogon", "services", "lsass", "lsaiso", "dwm", "fontdrvhost", "svchost", "sihost", "LsaIso", "MsMpEng", "audiodg",
    };

    public static bool Elevated
    {
        get
        {
            try { using var id = WindowsIdentity.GetCurrent(); return new WindowsPrincipal(id).IsInRole(WindowsBuiltInRole.Administrator); }
            catch { return false; }
        }
    }

    // ------------------------------------------------------------------ apps and processes

    /// <summary>Running apps (with a window) and background processes, CPU % since the last call and memory.</summary>
    public object Processes()
    {
        lock (_lock)
        {
            var procs = Snapshot();
            long now = Stopwatch.GetTimestamp();
            double wall100ns = _prevStamp == 0 ? 0 : (now - _prevStamp) * 1e7 / Stopwatch.Frequency;
            int cores = Math.Max(1, Environment.ProcessorCount);
            var cpu = new Dictionary<int, double>();
            var next = new Dictionary<long, long>();
            foreach (var p in procs)
            {
                next[p.Key] = p.CpuTime;
                if (p.Pid == 0) continue;
                if (wall100ns > 0 && _prevCpu.TryGetValue(p.Key, out var before))
                    cpu[p.Pid] = Math.Max(0, (p.CpuTime - before) / (wall100ns * cores) * 100);
            }
            _prevCpu = next;
            _prevStamp = now;

            var byPid = procs.Where(p => p.Pid != 0).GroupBy(p => p.Pid).ToDictionary(g => g.Key, g => g.First());
            var children = new Dictionary<int, List<Proc>>();
            foreach (var p in byPid.Values)
            {
                if (p.Parent == p.Pid || !byPid.TryGetValue(p.Parent, out var parent) || p.Create < parent.Create) continue;   // pid reused
                if (!children.TryGetValue(p.Parent, out var l)) children[p.Parent] = l = new List<Proc>();
                l.Add(p);
            }

            // Ember and everything it started (its WebView2 pages, helpers): one protected entry.
            var ember = new HashSet<int>();
            void Collect(int pid, HashSet<int> into)
            {
                if (!into.Add(pid)) return;
                if (children.TryGetValue(pid, out var kids)) foreach (var k in kids) Collect(k.Pid, into);
            }
            Collect(Environment.ProcessId, ember);

            // Which processes have a taskbar window, and its title.
            var windows = WindowTitles();
            bool explorerIsShell = DesktopMode.ExplorerShellRunning();
            var claimed = new HashSet<int>(ember);
            var apps = new Dictionary<string, Group>(StringComparer.OrdinalIgnoreCase);
            foreach (var (pid, title) in windows)
            {
                if (ember.Contains(pid) || !byPid.TryGetValue(pid, out var root)) continue;
                // A window in a child of another app (a browser's helper): it's part of that app.
                if (HasAppAncestor(root, byPid, windows)) continue;
                var name = root.Name;
                if (!apps.TryGetValue(name, out var g)) apps[name] = g = new Group("app:" + name, name, true) { Title = title };
                var tree = new HashSet<int>();
                Collect(pid, tree);
                foreach (var t in tree) if (claimed.Add(t)) g.Pids.Add(t);
            }

            var background = new Dictionary<string, Group>(StringComparer.OrdinalIgnoreCase);
            foreach (var p in byPid.Values)
            {
                if (claimed.Contains(p.Pid)) continue;
                if (!background.TryGetValue(p.Name, out var g)) background[p.Name] = g = new Group("bg:" + p.Name, p.Name, false);
                g.Pids.Add(p.Pid);
            }
            var emberGroup = new Group("ember", "Ember", false) { Protected = true, Pids = ember.Where(byPid.ContainsKey).ToList() };

            object Shape(Group g)
            {
                var members = g.Pids.Where(byPid.ContainsKey).Select(x => byPid[x]).ToList();
                var first = members.OrderBy(m => m.Create).FirstOrDefault();
                string? path = first is null ? null : PathOf(first);
                bool prot = g.Protected || members.Any(m => m.Pid <= 4 || Critical.Contains(m.Name))
                            || (explorerIsShell && members.Any(m => m.Name.Equals("explorer", StringComparison.OrdinalIgnoreCase)));
                string display = g.Key == "ember" ? "Ember (Hearth OS shell)" : FriendlyName(path, g.Name);
                return new
                {
                    key = g.Key,
                    name = display,
                    exe = g.Name,
                    path,
                    title = g.Title,
                    icon = g.Window || g.Key == "ember" ? IconOf(path) : IconOfCached(path),
                    pids = g.Pids,
                    count = members.Count,
                    cpu = Math.Round(members.Sum(m => cpu.GetValueOrDefault(m.Pid)), 1),
                    memory = members.Sum(m => m.Memory),
                    threads = members.Sum(m => m.Threads),
                    window = g.Window,
                    @protected = prot,
                };
            }

            var appList = apps.Values.Select(Shape).ToList();
            var bgList = background.Values.Select(Shape).ToList();
            if (emberGroup.Pids.Count > 0) bgList.Insert(0, Shape(emberGroup));
            return new
            {
                apps = appList,
                background = bgList,
                processCount = byPid.Count,
                threadCount = byPid.Values.Sum(p => p.Threads),
                handleCount = byPid.Values.Sum(p => (long)p.Handles),
                elevated = Elevated,
            };
        }
    }

    private sealed class Group
    {
        public Group(string key, string name, bool window) { Key = key; Name = name; Window = window; }
        public string Key { get; }
        public string Name { get; }
        public bool Window { get; }
        public string? Title { get; set; }
        public bool Protected { get; set; }
        public List<int> Pids { get; set; } = new();
    }

    private static bool HasAppAncestor(Proc p, Dictionary<int, Proc> byPid, Dictionary<int, string> windows)
    {
        var seen = new HashSet<int>();
        var cur = p;
        while (byPid.TryGetValue(cur.Parent, out var parent) && parent.Create <= cur.Create && seen.Add(parent.Pid))
        {
            if (windows.ContainsKey(parent.Pid) && parent.Name.Equals(p.Name, StringComparison.OrdinalIgnoreCase)) return true;
            cur = parent;
        }
        return false;
    }

    /// <summary>Process id → title of its first window that would be on the taskbar (the same rules as Mantel's).</summary>
    private static Dictionary<int, string> WindowTitles()
    {
        var map = new Dictionary<int, string>();
        Native.EnumWindows((h, _) =>
        {
            if (!Native.IsWindowVisible(h) || Native.GetWindow(h, Native.GW_OWNER) != IntPtr.Zero) return true;
            int ex = Native.GetWindowLong(h, Native.GWL_EXSTYLE);
            if ((ex & Native.WS_EX_TOOLWINDOW) != 0 && (ex & Native.WS_EX_APPWINDOW) == 0) return true;
            if (Native.DwmGetWindowAttribute(h, Native.DWMWA_CLOAKED, out int cloaked, 4) == 0 && cloaked != 0) return true;
            int len = Native.GetWindowTextLength(h);
            if (len == 0) return true;
            var cls = new StringBuilder(256);
            Native.GetClassName(h, cls, cls.Capacity);
            if (cls.ToString() is "Progman" or "WorkerW" or "Shell_TrayWnd" or "Shell_SecondaryTrayWnd") return true;
            Native.GetWindowThreadProcessId(h, out var pid);
            if (map.ContainsKey((int)pid)) return true;
            var sb = new StringBuilder(len + 1);
            Native.GetWindowText(h, sb, sb.Capacity);
            map[(int)pid] = sb.ToString();
            return true;
        }, IntPtr.Zero);
        return map;
    }

    private string? PathOf(Proc p)
    {
        if (_pathByProc.TryGetValue(p.Key, out var known)) return known;
        string? path = null;
        try { path = Native.ProcessPath((uint)p.Pid); } catch { }
        if (_pathByProc.Count > 4000) _pathByProc.Clear();
        return _pathByProc[p.Key] = path;
    }

    /// <summary>The app's own name (its file description, like Task Manager shows), else the exe's name.</summary>
    private string FriendlyName(string? path, string fallback)
    {
        if (path is null) return fallback;
        if (_nameByPath.TryGetValue(path, out var n)) return n;
        string name = fallback;
        try
        {
            var v = FileVersionInfo.GetVersionInfo(path);
            if (!string.IsNullOrWhiteSpace(v.FileDescription)) name = v.FileDescription.Trim();
            else if (!string.IsNullOrWhiteSpace(v.ProductName)) name = v.ProductName.Trim();
        }
        catch { }
        return _nameByPath[path] = name;
    }

    private string? IconOf(string? path)
    {
        if (path is null) return null;
        if (_iconByPath.TryGetValue(path, out var i)) return i;
        return _iconByPath[path] = Native.ShellIcon(path, 32);
    }

    /// <summary>Background processes: icons are worked out a few per refresh so the first one stays quick.</summary>
    private int _iconBudget;
    private string? IconOfCached(string? path)
    {
        if (path is null) return null;
        if (_iconByPath.TryGetValue(path, out var i)) return i;
        if (_iconBudget-- <= 0) return null;
        return IconOf(path);
    }

    /// <summary>Every process, from one NtQuerySystemInformation(SystemProcessInformation) call. Offsets are the
    /// 64-bit SYSTEM_PROCESS_INFORMATION layout (Ember is x64 and ARM64 only).</summary>
    private List<Proc> Snapshot()
    {
        _iconBudget = 12;
        var list = new List<Proc>(400);
        int size = 1 << 20;
        IntPtr buf = IntPtr.Zero;
        try
        {
            for (int attempt = 0; attempt < 6; attempt++)
            {
                buf = Marshal.AllocHGlobal(size);
                int st = NtQuerySystemInformation(5 /* SystemProcessInformation */, buf, size, out int needed);
                if (st == 0) break;
                Marshal.FreeHGlobal(buf); buf = IntPtr.Zero;
                if (st != StatusInfoLengthMismatch) throw new Win32Exception($"Couldn't read the process list (0x{st:X8})");
                size = Math.Max(size * 2, needed + (1 << 16));
            }
            if (buf == IntPtr.Zero) throw new InvalidOperationException("Couldn't read the process list");
            long offset = 0;
            while (true)
            {
                IntPtr p = IntPtr.Add(buf, (int)offset);
                int next = Marshal.ReadInt32(p, 0x00);
                int threads = Marshal.ReadInt32(p, 0x04);
                long privateWs = Marshal.ReadInt64(p, 0x08);
                long create = Marshal.ReadInt64(p, 0x20);
                long user = Marshal.ReadInt64(p, 0x28);
                long kernel = Marshal.ReadInt64(p, 0x30);
                int nameBytes = (ushort)Marshal.ReadInt16(p, 0x38);
                IntPtr nameBuf = Marshal.ReadIntPtr(p, 0x40);
                int pid = (int)Marshal.ReadIntPtr(p, 0x50).ToInt64();
                int parent = (int)Marshal.ReadIntPtr(p, 0x58).ToInt64();
                int handles = Marshal.ReadInt32(p, 0x60);
                long ws = Marshal.ReadIntPtr(p, 0x90).ToInt64();
                string name = pid == 0 ? "System Idle Process"
                    : nameBuf == IntPtr.Zero || nameBytes == 0 ? (pid == 4 ? "System" : "Process " + pid)
                    : Marshal.PtrToStringUni(nameBuf, nameBytes / 2) ?? "";
                if (name.EndsWith(".exe", StringComparison.OrdinalIgnoreCase)) name = name[..^4];
                // Private working set, like Task Manager's Memory column (0 on very old builds: use the working set).
                list.Add(new Proc(pid, parent, name, user + kernel, create, privateWs > 0 ? privateWs : ws, threads, handles));
                if (next == 0) break;
                offset += next;
            }
        }
        finally { if (buf != IntPtr.Zero) Marshal.FreeHGlobal(buf); }
        return list;
    }

    private const int StatusInfoLengthMismatch = unchecked((int)0xC0000004);

    // ------------------------------------------------------------------ end task

    private static void CheckAllowed(int[] pids)
    {
        if (pids.Length == 0) throw new ArgumentException("Nothing to end");
        bool explorerIsShell = DesktopMode.ExplorerShellRunning();
        foreach (var pid in pids)
        {
            if (pid <= 4 || pid == Environment.ProcessId) throw new InvalidOperationException("Ember and Windows' own processes can't be ended here");
            string? name = null;
            try { using var p = Process.GetProcessById(pid); name = p.ProcessName; }
            catch { }   // already gone
            if (name is not null && (Critical.Contains(name) || (explorerIsShell && name.Equals("explorer", StringComparison.OrdinalIgnoreCase))))
                throw new InvalidOperationException($"{name} is part of Windows and can't be ended here");
        }
    }

    /// <summary>End task: asks the app's windows to close (like clicking ×). closing = a window was asked.</summary>
    public object End(int[] pids)
    {
        CheckAllowed(pids);
        var set = pids.ToHashSet();
        int asked = 0;
        Native.EnumWindows((h, _) =>
        {
            if (!Native.IsWindowVisible(h) || Native.GetWindow(h, Native.GW_OWNER) != IntPtr.Zero) return true;
            Native.GetWindowThreadProcessId(h, out var pid);
            if (set.Contains((int)pid)) { Native.PostMessage(h, Native.WM_CLOSE, IntPtr.Zero, IntPtr.Zero); asked++; }
            return true;
        }, IntPtr.Zero);
        return new { closing = asked > 0, alive = Alive(pids) };
    }

    /// <summary>Force end: kills each process and everything it started.</summary>
    public object Kill(int[] pids)
    {
        CheckAllowed(pids);
        string? error = null;
        foreach (var pid in pids)
        {
            try { using var p = Process.GetProcessById(pid); p.Kill(entireProcessTree: true); }
            catch (ArgumentException) { }   // already gone
            catch (Win32Exception) { error = Elevated ? "Windows wouldn't let Ember end it" : "Ending it needs administrator rights (run Ember as administrator)"; }
            catch (InvalidOperationException) { }   // exited meanwhile
            catch (Exception ex) { error = ex.Message; }
        }
        Thread.Sleep(150);
        bool alive = Alive(pids);
        if (alive && error is not null) throw new InvalidOperationException(error);
        return new { alive };
    }

    /// <summary>Is any of them still running?</summary>
    public bool Alive(int[] pids)
    {
        foreach (var pid in pids)
        {
            try { using var p = Process.GetProcessById(pid); if (!p.HasExited) return true; }
            catch { }
        }
        return false;
    }

    // ------------------------------------------------------------------ performance

    /// <summary>A sample for the Performance page (the page keeps the last 60 for its graphs).</summary>
    public object Performance(PowerService power)
    {
        lock (_lock)
        {
            var cores = CoreUse();
            int total = cores.Length == 0 ? 0 : (int)Math.Round(cores.Average());
            var mem = new MEMORYSTATUSEX { dwLength = (uint)Marshal.SizeOf<MEMORYSTATUSEX>() };
            GlobalMemoryStatusEx(ref mem);
            PerfStats? g = null;
            try { g = _gpu.Sample(); } catch { }
            var b = power.Battery();
            var ps = SystemInformation.PowerStatus;
            return new
            {
                cpu = new
                {
                    total,
                    cores = cores.Select(c => (int)Math.Round(c)).ToArray(),
                    name = CpuName,
                    mhz = CpuMhz,
                    threads = Environment.ProcessorCount,
                    uptimeSec = Environment.TickCount64 / 1000,
                },
                memory = new
                {
                    total = (long)mem.ullTotalPhys,
                    used = (long)(mem.ullTotalPhys - mem.ullAvailPhys),
                    available = (long)mem.ullAvailPhys,
                    committed = (long)(mem.ullTotalPageFile - mem.ullAvailPageFile),
                    commitLimit = (long)mem.ullTotalPageFile,
                    load = (int)mem.dwMemoryLoad,
                },
                gpu = new
                {
                    available = g?.Gpu is not null,
                    use = g?.Gpu,
                    vramUsedGb = g?.VramUsedGb,
                    name = GpuName,
                },
                temps = Temps(),
                battery = new
                {
                    present = b.Present,
                    pct = b.Pct,
                    charging = b.Charging,
                    plugged = ps.PowerLineStatus == PowerLineStatus.Online,
                    secondsLeft = ps.BatteryLifeRemaining > 0 ? ps.BatteryLifeRemaining : (int?)null,
                    wmi = b.Present ? BatteryWmi() : null,
                },
            };
        }
    }

    private static string? _cpuName, _gpuName;
    private static int? _cpuMhz;
    private static string CpuName => _cpuName ??= (Registry.GetValue(@"HKEY_LOCAL_MACHINE\HARDWARE\DESCRIPTION\System\CentralProcessor\0", "ProcessorNameString", "") as string ?? "").Trim();
    private static int? CpuMhz => _cpuMhz ??= Registry.GetValue(@"HKEY_LOCAL_MACHINE\HARDWARE\DESCRIPTION\System\CentralProcessor\0", "~MHz", null) is int m ? (int?)m : null;
    private static string GpuName
    {
        get
        {
            if (_gpuName is not null) return _gpuName;
            try
            {
                var info = SystemInfo.Get();
                _gpuName = info.GetType().GetProperty("gpu")?.GetValue(info) as string ?? "";
            }
            catch { _gpuName = ""; }
            return _gpuName;
        }
    }

    /// <summary>Use of each logical processor since the last call, 0–100.</summary>
    private double[] CoreUse()
    {
        int n = Math.Max(1, Environment.ProcessorCount);
        const int Size = 48;   // SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION
        IntPtr buf = Marshal.AllocHGlobal(Size * n);
        try
        {
            if (NtQuerySystemInformation(8 /* SystemProcessorPerformanceInformation */, buf, Size * n, out int got) != 0) return Array.Empty<double>();
            int count = Math.Min(n, got / Size);
            var idle = new long[count]; var tot = new long[count]; var use = new double[count];
            for (int i = 0; i < count; i++)
            {
                IntPtr p = IntPtr.Add(buf, i * Size);
                idle[i] = Marshal.ReadInt64(p, 0);
                tot[i] = Marshal.ReadInt64(p, 8) + Marshal.ReadInt64(p, 16);   // kernel (includes idle) + user
                if (_prevTotal.Length == count)
                {
                    long dt = tot[i] - _prevTotal[i], di = idle[i] - _prevIdle[i];
                    use[i] = dt <= 0 ? 0 : Math.Clamp(100.0 * (dt - di) / dt, 0, 100);
                }
            }
            _prevIdle = idle; _prevTotal = tot;
            return use;
        }
        finally { Marshal.FreeHGlobal(buf); }
    }

    /// <summary>Temperatures from ACPI thermal zones. Windows only gives these to administrators, and many PCs
    /// don't have them at all; null then ("Not available"). Fans aren't readable without a vendor driver.</summary>
    private object? Temps()
    {
        if (DateTime.UtcNow - _tempsAt < TimeSpan.FromSeconds(10)) return _temps;
        _tempsAt = DateTime.UtcNow;
        try
        {
            var zones = new List<object>();
            using var s = new System.Management.ManagementObjectSearcher(@"root\WMI", "SELECT InstanceName, CurrentTemperature FROM MSAcpi_ThermalZoneTemperature");
            s.Options.Timeout = TimeSpan.FromSeconds(2);
            foreach (System.Management.ManagementObject o in s.Get())
            {
                using (o)
                {
                    double c = Convert.ToDouble(o["CurrentTemperature"]) / 10.0 - 273.15;
                    if (c > -20 && c < 150) zones.Add(new { name = (o["InstanceName"]?.ToString() ?? "Thermal zone").Split('\\').Last(), celsius = Math.Round(c, 1) });
                }
            }
            _temps = zones.Count > 0 ? zones : null;
        }
        catch { _temps = null; _tempsAt = DateTime.UtcNow.AddMinutes(5); }   // usually "access denied": don't keep asking
        return _temps;
    }

    /// <summary>Charge rate and capacity from the battery driver (root\WMI BatteryStatus and friends), best effort.</summary>
    private object? BatteryWmi()
    {
        if (DateTime.UtcNow - _batteryAt < TimeSpan.FromSeconds(5)) return _batteryWmi;
        _batteryAt = DateTime.UtcNow;
        try
        {
            long? rate = null, remaining = null, full = null, design = null;
            using (var s = new System.Management.ManagementObjectSearcher(@"root\WMI", "SELECT ChargeRate, DischargeRate, RemainingCapacity, Charging, Discharging FROM BatteryStatus"))
            {
                s.Options.Timeout = TimeSpan.FromSeconds(2);
                foreach (System.Management.ManagementObject o in s.Get())
                {
                    using (o)
                    {
                        long charge = Convert.ToInt64(o["ChargeRate"] ?? 0), discharge = Convert.ToInt64(o["DischargeRate"] ?? 0);
                        bool charging = o["Charging"] is bool c && c, discharging = o["Discharging"] is bool d && d;
                        rate = (rate ?? 0) + (charging ? charge : discharging ? -discharge : 0);
                        remaining = (remaining ?? 0) + Convert.ToInt64(o["RemainingCapacity"] ?? 0);
                    }
                }
            }
            try
            {
                using var s2 = new System.Management.ManagementObjectSearcher(@"root\WMI", "SELECT FullChargedCapacity FROM BatteryFullChargedCapacity");
                s2.Options.Timeout = TimeSpan.FromSeconds(2);
                foreach (System.Management.ManagementObject o in s2.Get()) using (o) full = (full ?? 0) + Convert.ToInt64(o["FullChargedCapacity"] ?? 0);
                using var s3 = new System.Management.ManagementObjectSearcher(@"root\WMI", "SELECT DesignedCapacity FROM BatteryStaticData");
                s3.Options.Timeout = TimeSpan.FromSeconds(2);
                foreach (System.Management.ManagementObject o in s3.Get()) using (o) design = (design ?? 0) + Convert.ToInt64(o["DesignedCapacity"] ?? 0);
            }
            catch { }
            _batteryWmi = rate is null ? null : new
            {
                watts = Math.Round(rate.Value / 1000.0, 1),       // + charging, − discharging
                remainingWh = remaining is > 0 ? Math.Round(remaining.Value / 1000.0, 1) : (double?)null,
                fullWh = full is > 0 ? Math.Round(full.Value / 1000.0, 1) : (double?)null,
                designWh = design is > 0 ? Math.Round(design.Value / 1000.0, 1) : (double?)null,
            };
        }
        catch { _batteryWmi = null; _batteryAt = DateTime.UtcNow.AddMinutes(2); }
        return _batteryWmi;
    }

    // ------------------------------------------------------------------ startup apps

    private const string RunKey = @"Software\Microsoft\Windows\CurrentVersion\Run";
    private const string Run32Key = @"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
    private const string Approved = @"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\";

    /// <summary>The places Windows starts apps from at sign-in, with the StartupApproved key each one's switch
    /// lives under (the same as Task Manager › Startup apps).</summary>
    private static IEnumerable<(string Source, RegistryKey Hive, string? RunPath, string? Folder, string ApprovedKey)> Places() => new (string, RegistryKey, string?, string?, string)[]
    {
        ("hkcu-run", Registry.CurrentUser, RunKey, null, "Run"),
        ("hklm-run", Registry.LocalMachine, RunKey, null, "Run"),
        ("hklm-run32", Registry.LocalMachine, Run32Key, null, "Run32"),
        ("hkcu-folder", Registry.CurrentUser, null, Environment.GetFolderPath(Environment.SpecialFolder.Startup), "StartupFolder"),
        ("hklm-folder", Registry.LocalMachine, null, Environment.GetFolderPath(Environment.SpecialFolder.CommonStartup), "StartupFolder"),
    };

    public object Startup()
    {
        bool admin = Elevated;
        var items = new List<object>();
        foreach (var (source, hive, runPath, folder, approvedKey) in Places())
        {
            bool canToggle = hive == Registry.CurrentUser || admin;
            string location = source switch
            {
                "hkcu-run" => "Your account (registry)",
                "hklm-run" => "All users (registry)",
                "hklm-run32" => "All users (registry, 32-bit)",
                "hkcu-folder" => "Your Startup folder",
                _ => "All users' Startup folder",
            };
            var entries = new List<(string Name, string Command, string? Exe)>();
            try
            {
                if (runPath is not null)
                {
                    using var k = hive.OpenSubKey(runPath);
                    foreach (var name in k?.GetValueNames() ?? Array.Empty<string>())
                    {
                        if (k!.GetValue(name) is not string cmd || string.IsNullOrWhiteSpace(cmd)) continue;
                        var exe = StartupApps.Split(Environment.ExpandEnvironmentVariables(cmd)).File;
                        entries.Add((name, cmd, File.Exists(exe) ? exe : null));
                    }
                }
                else if (folder is not null && Directory.Exists(folder))
                {
                    foreach (var f in Directory.GetFiles(folder))
                    {
                        var name = Path.GetFileName(f);
                        if (name.Equals("desktop.ini", StringComparison.OrdinalIgnoreCase)) continue;
                        entries.Add((name, f, f));
                    }
                }
            }
            catch { }
            foreach (var (name, command, exe) in entries)
            {
                string? publisher = null, display = null;
                if (exe is not null && exe.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
                {
                    try { var v = FileVersionInfo.GetVersionInfo(exe); publisher = v.CompanyName?.Trim(); display = v.FileDescription?.Trim(); } catch { }
                }
                string shown = folder is not null ? Path.GetFileNameWithoutExtension(name) : name;
                items.Add(new
                {
                    id = source + "|" + name,
                    name = shown,
                    description = string.IsNullOrWhiteSpace(display) || display == shown ? null : display,
                    command,
                    publisher = string.IsNullOrWhiteSpace(publisher) ? null : publisher,
                    where = location,
                    enabled = IsEnabled(hive, approvedKey, name),
                    canToggle,
                    icon = exe is null ? null : IconOf(exe),
                });
            }
        }
        return new { items, elevated = admin };
    }

    /// <summary>Task Manager's switch: first byte 2 (or no entry) = on, 3 = off.</summary>
    private static bool IsEnabled(RegistryKey hive, string approvedKey, string name)
    {
        try
        {
            using var k = hive.OpenSubKey(Approved + approvedKey);
            return k?.GetValue(name) is not byte[] b || b.Length == 0 || (b[0] & 1) == 0;
        }
        catch { return true; }
    }

    /// <summary>Turns a startup app on or off the way Task Manager does (12 bytes: 02… on; 03 + when it was turned off).</summary>
    public void SetStartup(string id, bool on)
    {
        int bar = id.IndexOf('|');
        if (bar <= 0) throw new ArgumentException("Unknown startup entry");
        string source = id[..bar], name = id[(bar + 1)..];
        var place = Places().FirstOrDefault(p => p.Source == source);
        if (place.Hive is null) throw new ArgumentException("Unknown startup entry");
        if (place.Hive != Registry.CurrentUser && !Elevated) throw new UnauthorizedAccessException("Apps started for all users can only be changed with administrator rights (run Ember as administrator)");
        var value = new byte[12];
        value[0] = (byte)(on ? 2 : 3);
        if (!on) BitConverter.GetBytes(DateTime.UtcNow.ToFileTimeUtc()).CopyTo(value, 4);
        using var k = place.Hive.CreateSubKey(Approved + place.ApprovedKey, writable: true)
                      ?? throw new InvalidOperationException("Couldn't open the startup settings");
        k.SetValue(name, value, RegistryValueKind.Binary);
    }

    // ------------------------------------------------------------------ native

    [StructLayout(LayoutKind.Sequential)]
    private struct MEMORYSTATUSEX
    {
        public uint dwLength, dwMemoryLoad;
        public ulong ullTotalPhys, ullAvailPhys, ullTotalPageFile, ullAvailPageFile, ullTotalVirtual, ullAvailVirtual, ullAvailExtendedVirtual;
    }
    [DllImport("kernel32.dll")] private static extern bool GlobalMemoryStatusEx(ref MEMORYSTATUSEX m);
    [DllImport("ntdll.dll")] private static extern int NtQuerySystemInformation(int infoClass, IntPtr info, int length, out int returnLength);
}
