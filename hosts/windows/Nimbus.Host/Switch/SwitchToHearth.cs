using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text.Json;
using Microsoft.Win32;

namespace Nimbus.Host.Switch;

/// <summary>
/// Settings › Switch to Hearth OS (Ember): moving this PC from Windows to Hearth OS, step by step.
///   1. Check the PC: UEFI, Fast Startup, room on C:, charger, the games Ember knows about.
///   2. Make room: Windows shrinks its own drive (UAC prompt) and Fast Startup is turned off.
///   3. Make the installer USB: the Hearth ISO (downloaded, or one you already have) written to a USB drive.
///   4. Restart into it: Windows' "Use a device" start-up menu, where you pick the USB drive.
/// The live USB then installs Hearth into the free space ("Install next to Windows"), and Hearth offers to
/// bring the games over on its first start. Admin steps run in <see cref="AdminTasks"/>.
/// </summary>
public sealed class SwitchToHearth
{
    /// <summary>Where the installer can be downloaded from, once it's hosted (empty: pick a file instead).</summary>
    public const string IsoUrlAmd = "";
    public const string IsoUrlNvidia = "";
    public const string IsoPage = "https://dragongamer9.github.io/hearth-os/#download";

    private readonly Action<string, object> _event;
    private readonly Library.LibraryService _games;
    private object? _job;          // the running admin step (for the page)
    private string? _iso;

    public SwitchToHearth(Action<string, object> evt, Library.LibraryService games) { _event = evt; _games = games; }

    // ------------------------------------------------------------------ 1. checks

    public async Task<object> StatusAsync()
    {
        var c = new DriveInfo("C");
        bool uefi = GetFirmwareType(out var ft) && ft == 2;
        bool fastStartup = false;
        try
        {
            using var k = Registry.LocalMachine.OpenSubKey(@"SYSTEM\CurrentControlSet\Control\Session Manager\Power");
            fastStartup = Convert.ToInt32(k?.GetValue("HiberbootEnabled") ?? 0) == 1;
        }
        catch { }
        var ps = SystemInformation.PowerStatus;
        bool hasBattery = ps.BatteryChargeStatus != BatteryChargeStatus.NoSystemBattery;
        int games = 0; long gameBytes = 0;
        try
        {
            var lib = await _games.GetAsync(false);
            foreach (var g in lib.Games) if (g.Installed) { games++; gameBytes += g.SizeBytes ?? 0; }
        }
        catch { }
        string gpu = "";
        try
        {
            using var s = new System.Management.ManagementObjectSearcher("SELECT Name FROM Win32_VideoController");
            gpu = string.Join(", ", s.Get().Cast<System.Management.ManagementObject>().Select(o => o["Name"]?.ToString() ?? ""));
        }
        catch { }
        bool nvidia = gpu.Contains("NVIDIA", StringComparison.OrdinalIgnoreCase) && !gpu.Contains("AMD", StringComparison.OrdinalIgnoreCase) && !gpu.Contains("Radeon", StringComparison.OrdinalIgnoreCase) && !gpu.Contains("Intel", StringComparison.OrdinalIgnoreCase);
        return new
        {
            uefi,
            fastStartup,
            free = c.IsReady ? c.AvailableFreeSpace : 0,
            size = c.IsReady ? c.TotalSize : 0,
            plugged = !hasBattery || ps.PowerLineStatus == PowerLineStatus.Online,
            battery = hasBattery ? (int)(ps.BatteryLifePercent * 100) : (int?)null,
            games, gameBytes,
            gpu, edition = nvidia ? "nvidia" : "amd-intel",
            isoUrl = nvidia ? IsoUrlNvidia : IsoUrlAmd, isoPage = IsoPage,
            iso = _iso, isoSize = _iso is not null && File.Exists(_iso) ? new FileInfo(_iso).Length : 0,
            usb = UsbDrives(),
            job = _job,
        };
    }

    public static List<object> UsbDrives()
    {
        var list = new List<object>();
        try
        {
            using var s = new System.Management.ManagementObjectSearcher("SELECT Index, Model, Size, InterfaceType, MediaType FROM Win32_DiskDrive");
            foreach (System.Management.ManagementObject d in s.Get())
            {
                var iface = d["InterfaceType"]?.ToString() ?? "";
                var media = d["MediaType"]?.ToString() ?? "";
                if (!iface.Equals("USB", StringComparison.OrdinalIgnoreCase) && !media.Contains("Removable", StringComparison.OrdinalIgnoreCase)) continue;
                long size = Convert.ToInt64(d["Size"] ?? 0L);
                list.Add(new { disk = Convert.ToInt32(d["Index"]), name = (d["Model"]?.ToString() ?? "USB drive").Trim(), size, bigEnough = size >= 15_000_000_000L });
            }
        }
        catch (Exception ex) { SystemBridge.Log("switch: usb list: " + ex.Message); }
        return list;
    }

    // ------------------------------------------------------------------ 2.–3. admin steps

    public void Prepare(int sizeGb) => RunAdmin("prepare", sizeGb.ToString(), "Making room for Hearth");

    public void WriteUsb(int disk)
    {
        if (_iso is null || !File.Exists(_iso)) throw new InvalidOperationException("Choose the Hearth installer (.iso) first");
        RunAdmin("writeusb", $"{disk} \"{_iso}\"", "Making the installer USB");
    }

    public string? PickIso(IWin32Window owner)
    {
        using var dlg = new OpenFileDialog { Title = "Choose the Hearth OS installer", Filter = "Hearth OS installer (*.iso)|*.iso", CheckFileExists = true };
        if (dlg.ShowDialog(owner) != DialogResult.OK) return _iso;
        var len = new FileInfo(dlg.FileName).Length;
        if (len < 1_000_000_000L) throw new InvalidOperationException("That file is too small to be the Hearth installer");
        return _iso = dlg.FileName;
    }

    /// <summary>Downloads the installer (when it's hosted) into Downloads, with progress events.</summary>
    public async Task DownloadIsoAsync(string url)
    {
        if (string.IsNullOrEmpty(url)) throw new InvalidOperationException("The installer isn't available to download here yet. Download it from the website, then choose it.");
        var dir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), "Downloads");
        var file = Path.Combine(dir, Path.GetFileName(new Uri(url).AbsolutePath) is { Length: > 4 } n && n.EndsWith(".iso") ? n : "hearth-os.iso");
        using var http = new HttpClient();
        using var res = await http.GetAsync(url, HttpCompletionOption.ResponseHeadersRead);
        res.EnsureSuccessStatusCode();
        long total = res.Content.Headers.ContentLength ?? 0, done = 0;
        await using (var src = await res.Content.ReadAsStreamAsync())
        await using (var dst = new FileStream(file + ".part", FileMode.Create, FileAccess.Write, FileShare.None, 1 << 20))
        {
            var buf = new byte[1 << 20];
            var sw = Stopwatch.StartNew(); long last = 0;
            int n2;
            while ((n2 = await src.ReadAsync(buf)) > 0)
            {
                await dst.WriteAsync(buf.AsMemory(0, n2));
                done += n2;
                if (sw.ElapsedMilliseconds - last > 500)
                {
                    last = sw.ElapsedMilliseconds;
                    SetJob(new { kind = "download", state = "running", pct = total > 0 ? 100.0 * done / total : 0, text = $"Downloading the installer: {done / 1e9:0.0}{(total > 0 ? $" of {total / 1e9:0.0}" : "")} GB · {done / 1e6 / Math.Max(0.5, sw.Elapsed.TotalSeconds):0} MB/s" });
                }
            }
        }
        File.Move(file + ".part", file, true);
        _iso = file;
        SetJob(new { kind = "download", state = "done", pct = 100, text = "Downloaded the installer" });
    }

    private void RunAdmin(string task, string args, string title)
    {
        var progress = Path.Combine(Path.GetTempPath(), $"ember-switch-{Guid.NewGuid():N}.json");
        Process p;
        try
        {
            p = Process.Start(new ProcessStartInfo(Environment.ProcessPath!, $"--admin {task} \"{progress}\" {args}") { UseShellExecute = true, Verb = "runas" })!;
        }
        catch (Win32Exception ex) when (ex.NativeErrorCode == 1223)
        {
            throw new InvalidOperationException("Windows needs your OK (the administrator prompt) for this step.");
        }
        SetJob(new { kind = task, state = "running", pct = 0, text = title });
        _ = Task.Run(async () =>
        {
            while (!p.HasExited)
            {
                await Task.Delay(500);
                Relay(task, progress);
            }
            Relay(task, progress);
            if (p.ExitCode != 0 && _job is not null && JsonSerializer.Serialize(_job).Contains("\"running\""))
                SetJob(new { kind = task, state = "error", pct = 0, text = "", error = "That step stopped without finishing (code " + p.ExitCode + ")." });
            try { File.Delete(progress); } catch { }
        });
    }

    private void Relay(string task, string file)
    {
        try
        {
            if (!File.Exists(file)) return;
            using var doc = JsonDocument.Parse(File.ReadAllText(file));
            var r = doc.RootElement;
            SetJob(new
            {
                kind = task,
                state = r.GetProperty("state").GetString(),
                pct = r.GetProperty("pct").GetDouble(),
                text = r.GetProperty("text").GetString(),
                error = r.TryGetProperty("error", out var e) ? e.GetString() : null,
            });
        }
        catch { /* mid-write */ }
    }

    private void SetJob(object job) { _job = job; _event("switch", new { job }); }

    // ------------------------------------------------------------------ 4. restart into the USB

    /// <summary>Restarts into Windows' start-up options, where "Use a device" starts the USB drive.</summary>
    public static void RestartToBootMenu() =>
        Process.Start(new ProcessStartInfo("shutdown.exe", "/r /o /t 0") { UseShellExecute = false, CreateNoWindow = true });

    [DllImport("kernel32.dll")] private static extern bool GetFirmwareType(out int type);
}
