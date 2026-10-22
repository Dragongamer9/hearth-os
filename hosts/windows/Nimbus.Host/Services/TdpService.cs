using System.ComponentModel;
using System.Diagnostics;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Security.Principal;
using System.Text.Json;

namespace Nimbus.Host.Services;

/// <summary>
/// The power limit (TDP) on AMD handhelds and laptops, for Quick Access › Performance and each game's own limit
/// (Game settings › Power limit). Like Hearth's hearth-admin, it uses RyzenAdj: --stapm-limit, --fast-limit and
/// --slow-limit all set to the same watts, 5 to 30 W.
///
/// RyzenAdj (github.com/FlyGoat/RyzenAdj, LGPL-3.0) is a separate program, downloaded from its GitHub release the
/// first time and checked against a pinned SHA-256, like legendary and gogdl. It needs administrator rights (its
/// WinRing0 driver), so unless Ember itself runs as administrator, each change goes through Windows' permission
/// prompt: a second copy of Ember ("--admin tdp") copies the checked files where only administrators can change them,
/// checks every file's SHA-256 again, and runs it.
///
/// Steps aside when Handheld Companion runs (it owns the power limit and has no public API to ask it), and on
/// processors RyzenAdj doesn't know (Intel, desktop Ryzen). Windows reads the limit back only with administrator
/// rights, so the number shown is the one Ember set last.
/// </summary>
public sealed class TdpService
{
    public const int Min = 5, Max = 30;

    // RyzenAdj v0.17.0, Windows x64. The zip, and each file the elevated helper runs (checked again before running).
    private const string ZipUrl = "https://github.com/FlyGoat/RyzenAdj/releases/download/v0.17.0/ryzenadj-win64.zip";
    private const string ZipSha256 = "7366f9b60c0faaab2ee550ad6432de6a9eea71116cce434c25a573ca8a5c947c";
    private static readonly Dictionary<string, string> Files = new(StringComparer.OrdinalIgnoreCase)
    {
        ["ryzenadj.exe"] = "2ea7c625d14234b99608cbd54c5d4b767c0f325fdda32eea5e0629c283837ed6",
        ["libryzenadj.dll"] = "ae4793015b38cfe9b7544edf05b7374b961887203eab8bb87779a8defdf62349",
        ["WinRing0x64.dll"] = "a746fd5728e7485f741cc330a279674bc8590b1b8007d8614046c49f58698485",
        ["WinRing0x64.sys"] = "11bd2c9f9e2397c9a16e0990e4ed2cf0679498fe0fd418a3dfdac60b5c160ee5",
        ["inpoutx64.dll"] = "5f27ed4d5cd58a1ee23deeb802e09e73f3a1d884ce2135f6e827f67b171269e7",
    };

    private static string UserDir => Path.Combine(Stores.HelperTools.DataDir, "tools", "ryzenadj");
    /// <summary>Where the elevated helper runs it from: made by an administrator, only administrators may change it.</summary>
    private static string AdminDir => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData), "EmberRyzenAdj");

    private readonly Func<int> _saved;
    private readonly Action<int> _save;
    private readonly SemaphoreSlim _gate = new(1, 1);
    private string? _lastError;

    /// <param name="saved">The limit Ember set last (0: none yet).</param>
    public TdpService(Func<int> saved, Action<int> save) { _saved = saved; _save = save; }

    public static bool IsElevated()
    {
        try { using var id = WindowsIdentity.GetCurrent(); return new WindowsPrincipal(id).IsInRole(WindowsBuiltInRole.Administrator); }
        catch { return false; }
    }

    private static string Cpu()
    {
        try { return (Microsoft.Win32.Registry.LocalMachine.OpenSubKey(@"HARDWARE\DESCRIPTION\System\CentralProcessor\0")?.GetValue("ProcessorNameString") as string ?? "").Trim(); }
        catch { return ""; }
    }

    private static bool HandheldCompanion()
    {
        foreach (var n in new[] { "HandheldCompanion", "HandheldCompanion.Service" })
        {
            var ps = Process.GetProcessesByName(n);
            bool any = ps.Length > 0;
            foreach (var p in ps) p.Dispose();
            if (any) return true;
        }
        return false;
    }

    /// <summary>Why the limit can't be set here, or null when it can.</summary>
    private static string? Unsupported()
    {
        if (System.Runtime.InteropServices.RuntimeInformation.OSArchitecture != System.Runtime.InteropServices.Architecture.X64) return "The power limit can only be set on AMD processors.";
        var cpu = Cpu();
        if (!cpu.Contains("AMD", StringComparison.OrdinalIgnoreCase)) return "The power limit can only be set on AMD processors.";
        // RyzenAdj knows AMD's laptop and handheld chips (with Radeon graphics on the chip), not desktop Ryzens.
        bool battery = !SystemInformation.PowerStatus.BatteryChargeStatus.HasFlag(BatteryChargeStatus.NoSystemBattery);
        bool mobile = battery || cpu.Contains("Radeon", StringComparison.OrdinalIgnoreCase) || cpu.Contains(" Z1", StringComparison.OrdinalIgnoreCase) || cpu.Contains(" Z2", StringComparison.OrdinalIgnoreCase);
        if (!mobile) return "The power limit can be set on AMD handhelds and laptops.";
        if (HandheldCompanion()) return "Handheld Companion manages the power limit on this device: set it there.";
        return null;
    }

    /// <summary>{supported, watts, min, max, via, needsAdmin, reason}: what Quick Access shows (p.tdp).</summary>
    public object Status()
    {
        var why = Unsupported();
        int w = _saved();
        return new
        {
            supported = why is null,
            watts = Math.Clamp(w > 0 ? w : 15, Min, Max),
            min = Min, max = Max,
            via = why is null ? "ryzenadj" : null,
            needsAdmin = !IsElevated(),
            reason = why ?? _lastError,
        };
    }
    public bool Supported => Unsupported() is null;

    /// <summary>Downloads RyzenAdj the first time (checked against its pinned SHA-256) into %LOCALAPPDATA%\Nimbus\tools\ryzenadj.</summary>
    private static async Task EnsureAsync()
    {
        if (Files.Keys.All(f => File.Exists(Path.Combine(UserDir, f)) && Hash(Path.Combine(UserDir, f)) == Files[f])) return;
        Directory.CreateDirectory(UserDir);
        var tmp = Path.Combine(UserDir, "ryzenadj.zip.download");
        using (var resp = await Stores.HelperTools.Http.GetAsync(ZipUrl, HttpCompletionOption.ResponseHeadersRead))
        {
            resp.EnsureSuccessStatusCode();
            await using var src = await resp.Content.ReadAsStreamAsync();
            await using var dst = File.Create(tmp);
            await src.CopyToAsync(dst);
        }
        if (Hash(tmp) != ZipSha256)
        {
            File.Delete(tmp);
            throw new InvalidOperationException("The download of RyzenAdj didn't match its checksum, so Ember didn't use it. Try again later.");
        }
        using (var zip = ZipFile.OpenRead(tmp))
            foreach (var name in Files.Keys)
            {
                var e = zip.Entries.FirstOrDefault(x => string.Equals(x.Name, name, StringComparison.OrdinalIgnoreCase))
                        ?? throw new InvalidOperationException($"RyzenAdj's download is missing {name}");
                e.ExtractToFile(Path.Combine(UserDir, name), overwrite: true);
            }
        File.Delete(tmp);
        foreach (var (f, h) in Files)
            if (Hash(Path.Combine(UserDir, f)) != h) throw new InvalidOperationException($"RyzenAdj's {f} didn't match its checksum, so Ember didn't use it.");
        SystemBridge.Log("tools: downloaded RyzenAdj");
    }

    private static string Hash(string file)
    {
        using var f = File.OpenRead(file);
        return Convert.ToHexString(SHA256.HashData(f)).ToLowerInvariant();
    }

    /// <summary>Sets the limit (5–30 W). Asks Windows for permission when Ember isn't running as administrator.
    /// Returns the new status.</summary>
    public async Task<object> SetAsync(int watts, string why = "")
    {
        if (Unsupported() is { } no) throw new InvalidOperationException(no);
        int w = Math.Clamp(watts, Min, Max);
        await _gate.WaitAsync();
        try
        {
            await EnsureAsync();
            if (IsElevated())
            {
                var (ok, msg) = await Task.Run(() => ApplyAsAdmin(UserDir, w));
                if (!ok) throw new InvalidOperationException(msg);
            }
            else await RunElevatedAsync(w);
            _save(w);
            _lastError = null;
            SystemBridge.Log($"power: limit {w} W{(why.Length > 0 ? " (" + why + ")" : "")}");
        }
        catch (Exception ex) { _lastError = ex.Message; SystemBridge.Log("power: limit FAILED " + ex.Message); throw; }
        finally { _gate.Release(); }
        return Status();
    }

    private static async Task RunElevatedAsync(int watts)
    {
        var progress = Path.Combine(Path.GetTempPath(), $"ember-tdp-{Guid.NewGuid():N}.json");
        Process? p;
        try
        {
            p = Process.Start(new ProcessStartInfo(Environment.ProcessPath!, $"--admin tdp \"{progress}\" {watts} \"{UserDir}\"")
            { UseShellExecute = true, Verb = "runas", WindowStyle = ProcessWindowStyle.Hidden });
        }
        catch (Win32Exception ex) when (ex.NativeErrorCode == 1223)
        {
            throw new InvalidOperationException("Windows asked for permission to change the power limit, and it wasn't given.");
        }
        if (p is null) throw new InvalidOperationException("Windows didn't start the helper");
        using (p)
        {
            using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(60));
            try { await p.WaitForExitAsync(cts.Token); }
            catch (OperationCanceledException) { throw new InvalidOperationException("Changing the power limit took too long."); }
            string? error = null;
            try
            {
                if (File.Exists(progress))
                {
                    using var doc = JsonDocument.Parse(File.ReadAllText(progress));
                    if (doc.RootElement.TryGetProperty("error", out var e) && e.ValueKind == JsonValueKind.String) error = e.GetString();
                }
            }
            catch { }
            finally { try { File.Delete(progress); } catch { } }
            if (p.ExitCode != 0) throw new InvalidOperationException(error ?? $"RyzenAdj couldn't change the power limit (exit code {p.ExitCode}).");
        }
    }

    // ------------------------------------------------------------------ the elevated side (Ember.exe --admin tdp FILE WATTS)

    /// <summary>Runs as administrator (AdminTasks): the watts checked again, the files copied to a folder only
    /// administrators may change and their SHA-256 checked there, then RyzenAdj run from it.</summary>
    public static void RunElevated(string[] rest, Action<string, double, string, string?> report)
    {
        if (rest.Length < 1 || !int.TryParse(rest[0], out var w) || w < Min || w > Max) throw new ArgumentException("The power limit is 5 to 30 W");
        // Where the normal Ember downloaded it (another account's folder when someone else typed the password).
        var src = rest.Length > 1 && Path.IsPathFullyQualified(rest[1]) ? rest[1] : UserDir;
        var dir = AdminDir;
        Directory.CreateDirectory(dir);
        // Owned by Administrators; they and the system may change it, everyone else may only read and run it (also
        // when someone made the folder before Ember did).
        Exec("icacls.exe", $"\"{dir}\" /setowner *S-1-5-32-544 /T /Q");
        Exec("icacls.exe", $"\"{dir}\" /inheritance:r /grant:r *S-1-5-32-544:(OI)(CI)F *S-1-5-18:(OI)(CI)F *S-1-5-32-545:(OI)(CI)RX /T /Q");
        foreach (var (f, h) in Files)
        {
            var target = Path.Combine(dir, f);
            if (File.Exists(target) && Hash(target) == h) continue;   // already there (the driver may be in use)
            var bytes = File.ReadAllBytes(Path.Combine(src, f));
            if (Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant() != h) throw new InvalidOperationException($"RyzenAdj's {f} isn't the one Ember checked. Nothing was changed.");
            File.WriteAllBytes(target, bytes);
        }
        var (ok, msg) = ApplyAsAdmin(dir, w);
        if (!ok) throw new InvalidOperationException(msg);
        report("done", 100, $"Power limit {w} W", null);
    }

    /// <summary>Runs RyzenAdj from a folder (already checked). (worked, message)</summary>
    private static (bool, string) ApplyAsAdmin(string dir, int w)
    {
        foreach (var (f, h) in Files)
            if (Hash(Path.Combine(dir, f)) != h) return (false, $"RyzenAdj's {f} isn't the one Ember checked. Nothing was changed.");
        var mw = (w * 1000).ToString();
        var psi = new ProcessStartInfo(Path.Combine(dir, "ryzenadj.exe"))
        {
            UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true, WorkingDirectory = dir,
        };
        foreach (var a in new[] { "--stapm-limit=" + mw, "--fast-limit=" + mw, "--slow-limit=" + mw }) psi.ArgumentList.Add(a);
        using var p = Process.Start(psi);
        if (p is null) return (false, "Couldn't start RyzenAdj");
        var o = p.StandardOutput.ReadToEndAsync();
        var e = p.StandardError.ReadToEndAsync();
        if (!p.WaitForExit(30_000)) { try { p.Kill(true); } catch { } return (false, "RyzenAdj took too long"); }
        var text = (o.GetAwaiter().GetResult() + "\n" + e.GetAwaiter().GetResult()).Trim();
        if (p.ExitCode == 0) return (true, "");
        var last = text.Split('\n').Select(l => l.Trim()).LastOrDefault(l => l.Length > 0) ?? $"exit code {p.ExitCode}";
        // The usual reason on Windows 11: the WinRing0 driver is on Microsoft's vulnerable-driver blocklist (Memory integrity).
        if (text.Contains("init", StringComparison.OrdinalIgnoreCase) || text.Contains("driver", StringComparison.OrdinalIgnoreCase))
            last += ". Windows may be blocking RyzenAdj's driver (WinRing0): with Memory integrity or the vulnerable-driver blocklist on, the power limit can't be changed this way.";
        return (false, "RyzenAdj: " + last);
    }

    private static void Exec(string file, string args)
    {
        using var p = Process.Start(new ProcessStartInfo(file, args) { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true });
        if (p is null) throw new InvalidOperationException("Couldn't run " + file);
        // Both at once: reading one to the end first can wait forever while the program waits for the other to be read.
        var err = p.StandardError.ReadToEndAsync();
        p.StandardOutput.ReadToEnd(); err.GetAwaiter().GetResult();
        p.WaitForExit();
        if (p.ExitCode != 0) throw new InvalidOperationException($"{file} failed ({p.ExitCode})");
    }
}
