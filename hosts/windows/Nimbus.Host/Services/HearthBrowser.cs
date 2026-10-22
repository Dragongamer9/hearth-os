using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Text.Json;
using Microsoft.Win32;

namespace Nimbus.Host.Services;

/// <summary>
/// Hearth Browser on Windows: Mozilla's Firefox with Hearth's settings, the same as on Hearth OS
/// (os/system_files/usr/share/hearth-browser, copied next to Nimbus.exe as hearth-browser\).
///
/// Ember keeps its own copy of Firefox in %LOCALAPPDATA%\Nimbus\HearthBrowser\firefox: Mozilla's official
/// installer, checked (Mozilla's SHA256SUMS and the installer's Mozilla signature) and unpacked there without
/// installing anything system-wide, then Hearth's parts copied over it (policies, the start-up script with the look,
/// the on-screen keyboard and the start page). Hearth's policies only touch this copy: a Firefox you installed
/// yourself stays exactly as you set it up. The copy updates itself, as any Firefox does. Your bookmarks, passwords
/// and add-ons are in HearthBrowser\profile. If the copy can't be set up (offline), an installed Firefox is used
/// with that profile instead.
///
/// <c>Nimbus.exe --browser [URL]</c> opens it (like <c>hearth-browser [URL]</c> on Hearth OS); Settings › System ›
/// Web browser and Beacon's links open it from Game Mode, where it fills the screen with bigger buttons and the
/// controller drives it like a sign-in page (A Enter, B Esc, D-pad and LB/RB move, Y the touch keyboard, right
/// stick the pointer: ControllerBridge treats its windows as Target.Web).
/// </summary>
internal static class HearthBrowser
{
    public static string Root => Path.Combine(Stores.HelperTools.DataDir, "HearthBrowser");
    private static string FirefoxDir => Path.Combine(Root, "firefox");
    private static string OwnExe => Path.Combine(FirefoxDir, "firefox.exe");
    public static string ProfileDir => Path.Combine(Root, "profile");
    private static string Parts => Path.Combine(AppContext.BaseDirectory, "hearth-browser");
    private static string LogPath => Path.Combine(Root, "setup.log");

    private const string Archive = "https://archive.mozilla.org/pub/firefox/releases/";
    private const string Versions = "https://product-details.mozilla.org/1.0/firefox_versions.json";

    private static readonly HttpClient Http = CreateHttp();
    private static HttpClient CreateHttp()
    {
        var h = new HttpClient { Timeout = TimeSpan.FromMinutes(20) };
        h.DefaultRequestHeaders.UserAgent.ParseAdd("Ember/" + (typeof(HearthBrowser).Assembly.GetName().Version?.ToString(3) ?? "0"));
        return h;
    }

    // ------------------------------------------------------------------ the setup job (Settings shows it)

    private static readonly object Gate = new();
    private static string _state = "idle";
    private static double? _progress;
    private static string _line = "";
    private static string? _error;
    private static Task? _job;
    /// <summary>Called when the setup job moves on (SystemBridge sends Settings the "browser" event).</summary>
    public static Action? Changed { get; set; }

    private static void Report(string state, double? progress, string line, string? error = null)
    {
        lock (Gate) { _state = state; _progress = progress; _line = line; _error = error; }
        try { Changed?.Invoke(); } catch { }
    }

    /// <summary>For Settings: is it there, which Firefox, and the setup job.</summary>
    public static object Status()
    {
        var exe = Exe();
        lock (Gate)
            return new
            {
                available = true,
                installed = exe is not null,
                own = File.Exists(OwnExe),
                engine = "Firefox",
                version = exe is null ? null : VersionOf(exe),
                @default = false,
                state = _state,
                progress = _progress,
                line = _line,
                error = _error,
            };
    }

    /// <summary>Sets up (or refreshes) Ember's copy of Firefox in the background.</summary>
    public static Task SetupAsync()
    {
        lock (Gate)
        {
            if (_job is { IsCompleted: false }) return _job;
            _job = Task.Run(async () =>
            {
                try
                {
                    await InstallAsync();
                    Report("idle", null, "");
                }
                catch (Exception ex)
                {
                    Log("setup failed: " + ex);
                    Report("idle", null, "", ex.Message);
                }
            });
            return _job;
        }
    }

    // ------------------------------------------------------------------ finding Firefox

    /// <summary>A Firefox installed the usual way (Windows' App Paths, or Program Files).</summary>
    public static string? SystemExe()
    {
        foreach (var hive in new[] { Registry.CurrentUser, Registry.LocalMachine })
        {
            try
            {
                using var k = hive.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\App Paths\firefox.exe");
                if (k?.GetValue(null) is string p && File.Exists(p.Trim('"'))) return p.Trim('"');
            }
            catch { }
        }
        foreach (var pf in new[] { Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86) })
        {
            var p = Path.Combine(pf, "Mozilla Firefox", "firefox.exe");
            if (pf.Length > 0 && File.Exists(p)) return p;
        }
        return null;
    }

    /// <summary>The Firefox Hearth Browser runs: Ember's own copy, else an installed one.</summary>
    public static string? Exe() => File.Exists(OwnExe) ? OwnExe : SystemExe();

    private static string? VersionOf(string exe)
    {
        try
        {
            var ini = Path.Combine(Path.GetDirectoryName(exe)!, "application.ini");
            if (File.Exists(ini))
                foreach (var line in File.ReadLines(ini))
                    if (line.StartsWith("Version=", StringComparison.Ordinal)) return line[8..].Trim();
            return FileVersionInfo.GetVersionInfo(exe).ProductVersion;
        }
        catch { return null; }
    }

    private static readonly HashSet<uint> _pids = new();
    private static DateTime _pidsAt;
    /// <summary>Is this process Hearth Browser (Ember's Firefox, or the installed one it uses)? Checked often by
    /// the controller, so the list is kept for 3 seconds.</summary>
    public static bool IsBrowserProcess(uint pid)
    {
        lock (_pids)
        {
            if (DateTime.UtcNow - _pidsAt > TimeSpan.FromSeconds(3))
            {
                _pids.Clear();
                var exe = Exe();
                var dir = exe is null ? null : Path.GetDirectoryName(exe) + Path.DirectorySeparatorChar;
                if (dir is not null)
                    foreach (var p in Process.GetProcessesByName("firefox"))
                    {
                        try { if (p.MainModule?.FileName is string f && f.StartsWith(dir, StringComparison.OrdinalIgnoreCase)) _pids.Add((uint)p.Id); }
                        catch { }
                        finally { p.Dispose(); }
                    }
                _pidsAt = DateTime.UtcNow;
            }
            return _pids.Contains(pid);
        }
    }

    // ------------------------------------------------------------------ setting up Ember's copy

    private static async Task<string> LatestAsync()
    {
        using var doc = JsonDocument.Parse(await Http.GetStringAsync(Versions));
        return doc.RootElement.GetProperty("LATEST_FIREFOX_VERSION").GetString()
            ?? throw new InvalidOperationException("Mozilla's list of versions has no newest Firefox.");
    }

    /// <summary>The newest Firefox from Mozilla, checked, unpacked into HearthBrowser\firefox, with Hearth's parts.</summary>
    public static async Task InstallAsync()
    {
        Directory.CreateDirectory(Root);
        Report("setup", 0.01, "Checking Mozilla's site");
        var version = Environment.GetEnvironmentVariable("HEARTH_FIREFOX_VERSION") is { Length: > 0 } v ? v : await LatestAsync();
        var arch = RuntimeInformation.OSArchitecture == Architecture.Arm64 ? "win64-aarch64" : "win64";
        var file = $"Firefox Setup {version}.exe";
        var name = $"{arch}/en-US/{file}";
        var baseUrl = Archive + version + "/";
        Log($"Firefox {version} ({name})");

        var sums = await Http.GetStringAsync(baseUrl + "SHA256SUMS");
        string? want = null;
        foreach (var raw in sums.Split('\n'))
        {
            var line = raw.Trim();
            int sp = line.IndexOf(' ');
            if (sp > 0 && line[(sp + 1)..].Trim().TrimStart('*') == name) { want = line[..sp].ToLowerInvariant(); break; }
        }
        if (want is null) throw new InvalidOperationException("Mozilla's list has no " + name);

        var dl = Path.Combine(Root, "download");
        if (Directory.Exists(dl)) Directory.Delete(dl, true);
        Directory.CreateDirectory(dl);
        var setup = Path.Combine(dl, "firefox-setup.exe");
        await DownloadAsync(baseUrl + arch + "/en-US/" + Uri.EscapeDataString(file), setup);

        Report("setup", 0.86, "Checking the download");
        string got;
        await using (var fs = File.OpenRead(setup)) got = Convert.ToHexString(await SHA256.HashDataAsync(fs)).ToLowerInvariant();
        if (got != want) throw new InvalidOperationException("The download isn't what Mozilla published. Try again.");
        if (!SignedByMozilla(setup)) throw new InvalidOperationException("The download isn't signed by Mozilla. Try again.");
        Log("download checked: sha256 " + got + ", signed by Mozilla");

        Report("setup", 0.9, "Unpacking");
        if (Process.GetProcessesByName("firefox").Any(p => { try { return p.MainModule?.FileName?.StartsWith(FirefoxDir, StringComparison.OrdinalIgnoreCase) == true; } catch { return false; } finally { p.Dispose(); } }))
            throw new InvalidOperationException("Close Hearth Browser first, then try again.");
        var unpacked = Path.Combine(dl, "unpacked");
        string? core = Unpack(setup, unpacked);
        var old = FirefoxDir + ".old";
        if (Directory.Exists(old)) Directory.Delete(old, true);
        if (Directory.Exists(FirefoxDir)) Directory.Move(FirefoxDir, old);
        try
        {
            if (core is not null) Directory.Move(core, FirefoxDir);
            else RunInstaller(setup, FirefoxDir);
            if (!File.Exists(OwnExe)) throw new InvalidOperationException("Firefox didn't unpack.");
        }
        catch
        {
            if (!Directory.Exists(FirefoxDir) && Directory.Exists(old)) Directory.Move(old, FirefoxDir);
            throw;
        }
        try { if (Directory.Exists(old)) Directory.Delete(old, true); } catch { }

        ApplyParts();
        File.WriteAllText(Path.Combine(FirefoxDir, "hearth-firefox.json"),
            JsonSerializer.Serialize(new { version, sha256 = got, source = baseUrl + name, installedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds() }));
        try { Directory.Delete(dl, true); } catch { }
        StartMenuShortcut();
        Log($"Firefox {version} set up in {FirefoxDir}");
        Report("setup", 1, "Done");
    }

    private static async Task DownloadAsync(string url, string dest)
    {
        using var resp = await Http.GetAsync(url, HttpCompletionOption.ResponseHeadersRead);
        resp.EnsureSuccessStatusCode();
        long total = resp.Content.Headers.ContentLength ?? 0, done = 0;
        await using var src = await resp.Content.ReadAsStreamAsync();
        await using var dst = File.Create(dest);
        var buf = new byte[1 << 16];
        int n, lastPct = -1;
        while ((n = await src.ReadAsync(buf)) > 0)
        {
            await dst.WriteAsync(buf.AsMemory(0, n));
            done += n;
            int pct = total > 0 ? (int)(done * 100 / total) : 0;
            if (pct != lastPct) { lastPct = pct; Report("setup", 0.02 + pct * 0.0083, "Downloading Firefox from Mozilla"); }
        }
    }

    /// <summary>Mozilla's installer is a 7-Zip archive with Firefox in its core folder: Windows' own tar (libarchive)
    /// reads it, so nothing is installed. Its core folder, or null when that didn't work.</summary>
    private static string? Unpack(string setup, string dest)
    {
        try
        {
            Directory.CreateDirectory(dest);
            var tar = Path.Combine(Environment.SystemDirectory, "tar.exe");
            if (!File.Exists(tar)) return null;
            var psi = new ProcessStartInfo(tar) { UseShellExecute = false, CreateNoWindow = true, RedirectStandardError = true, RedirectStandardOutput = true };
            psi.ArgumentList.Add("-xf"); psi.ArgumentList.Add(setup); psi.ArgumentList.Add("-C"); psi.ArgumentList.Add(dest);
            using var p = Process.Start(psi)!;
            var err = p.StandardError.ReadToEndAsync();
            _ = p.StandardOutput.ReadToEndAsync();
            if (!p.WaitForExit(300_000)) { try { p.Kill(true); } catch { } return null; }
            var core = Path.Combine(dest, "core");
            if (File.Exists(Path.Combine(core, "firefox.exe"))) return core;
            Log($"tar didn't unpack the installer (exit {p.ExitCode}): {err.Result.Trim()}");
        }
        catch (Exception ex) { Log("tar: " + ex.Message); }
        return null;
    }

    /// <summary>When it can't be unpacked: Mozilla's installer, silently, into that folder (no shortcuts, no
    /// maintenance service, no default-browser agent).</summary>
    private static void RunInstaller(string setup, string dir)
    {
        Log("running Mozilla's installer into " + dir);
        var psi = new ProcessStartInfo(setup) { UseShellExecute = false, CreateNoWindow = true };
        foreach (var a in new[] { "/S", "/InstallDirectoryPath=" + dir, "/TaskbarShortcut=false", "/DesktopShortcut=false",
                                  "/StartMenuShortcut=false", "/PrivateBrowsingShortcut=false", "/MaintenanceService=false",
                                  "/RegisterDefaultAgent=false", "/PreventRebootRequired=true" })
            psi.ArgumentList.Add(a);
        using var p = Process.Start(psi)!;
        if (!p.WaitForExit(600_000)) throw new InvalidOperationException("Firefox's installer didn't finish.");
    }

    /// <summary>Hearth's parts (next to Nimbus.exe) over Ember's copy of Firefox, where they differ.</summary>
    public static void ApplyParts()
    {
        if (!Directory.Exists(Parts) || !Directory.Exists(FirefoxDir)) return;
        foreach (var src in Directory.EnumerateFiles(Parts, "*", SearchOption.AllDirectories))
        {
            var dst = Path.Combine(FirefoxDir, Path.GetRelativePath(Parts, src));
            var s = new FileInfo(src);
            var d = new FileInfo(dst);
            if (d.Exists && d.Length == s.Length && d.LastWriteTimeUtc >= s.LastWriteTimeUtc) continue;
            Directory.CreateDirectory(Path.GetDirectoryName(dst)!);
            File.Copy(src, dst, overwrite: true);
        }
    }

    private static void StartMenuShortcut()
    {
        try
        {
            var dir = Environment.GetFolderPath(Environment.SpecialFolder.Programs);
            var lnk = Path.Combine(dir, "Hearth Browser.lnk");
            var shellType = Type.GetTypeFromProgID("WScript.Shell");
            if (shellType is null || Environment.ProcessPath is null) return;
            dynamic shell = Activator.CreateInstance(shellType)!;
            dynamic sc = shell.CreateShortcut(lnk);
            sc.TargetPath = Environment.ProcessPath;
            sc.Arguments = "--browser";
            sc.Description = "Hearth Browser: Firefox with Hearth's settings";
            sc.IconLocation = OwnExe + ",0";
            sc.Save();
        }
        catch (Exception ex) { Log("start menu shortcut: " + ex.Message); }
    }

    // ------------------------------------------------------------------ Mozilla's signature on the installer

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct WintrustFileInfo
    {
        public uint cbStruct;
        [MarshalAs(UnmanagedType.LPWStr)] public string pcwszFilePath;
        public IntPtr hFile;
        public IntPtr pgKnownSubject;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct WintrustData
    {
        public uint cbStruct;
        public IntPtr pPolicyCallbackData;
        public IntPtr pSIPClientData;
        public uint dwUIChoice;
        public uint fdwRevocationChecks;
        public uint dwUnionChoice;
        public IntPtr pFile;
        public uint dwStateAction;
        public IntPtr hWVTStateData;
        public IntPtr pwszURLReference;
        public uint dwProvFlags;
        public uint dwUIContext;
        public IntPtr pSignatureSettings;
    }

    [DllImport("wintrust.dll", ExactSpelling = true, CharSet = CharSet.Unicode)]
    private static extern int WinVerifyTrust(IntPtr hwnd, [MarshalAs(UnmanagedType.LPStruct)] Guid action, IntPtr data);

    private static readonly Guid GenericVerifyV2 = new("00AAC56B-CD44-11d0-8CC2-00C04FC295EE");

    /// <summary>Windows says the file's signature is good, and it's Mozilla's.</summary>
    private static bool SignedByMozilla(string path)
    {
        var fileInfo = new WintrustFileInfo { cbStruct = (uint)Marshal.SizeOf<WintrustFileInfo>(), pcwszFilePath = path };
        IntPtr pFile = Marshal.AllocHGlobal(Marshal.SizeOf<WintrustFileInfo>());
        IntPtr pData = Marshal.AllocHGlobal(Marshal.SizeOf<WintrustData>());
        try
        {
            Marshal.StructureToPtr(fileInfo, pFile, false);
            var data = new WintrustData
            {
                cbStruct = (uint)Marshal.SizeOf<WintrustData>(),
                dwUIChoice = 2,              // WTD_UI_NONE
                fdwRevocationChecks = 0,     // WTD_REVOKE_NONE
                dwUnionChoice = 1,           // WTD_CHOICE_FILE
                pFile = pFile,
                dwStateAction = 1,           // WTD_STATEACTION_VERIFY
                dwProvFlags = 0x40,          // WTD_CACHE_ONLY_URL_RETRIEVAL
            };
            Marshal.StructureToPtr(data, pData, false);
            int result = WinVerifyTrust(IntPtr.Zero, GenericVerifyV2, pData);
            data = Marshal.PtrToStructure<WintrustData>(pData);
            data.dwStateAction = 2;          // WTD_STATEACTION_CLOSE
            Marshal.StructureToPtr(data, pData, true);
            WinVerifyTrust(IntPtr.Zero, GenericVerifyV2, pData);
            if (result != 0) { Log($"signature check: 0x{result:x8}"); return false; }
        }
        finally
        {
            Marshal.FreeHGlobal(pData);
            Marshal.DestroyStructure<WintrustFileInfo>(pFile);
            Marshal.FreeHGlobal(pFile);
        }
        try
        {
#pragma warning disable SYSLIB0057
            using var cert = X509Certificate.CreateFromSignedFile(path);
#pragma warning restore SYSLIB0057
            var ok = cert.Subject.Contains("O=Mozilla Corporation", StringComparison.OrdinalIgnoreCase);
            if (!ok) Log("signed by someone else: " + cert.Subject);
            return ok;
        }
        catch (Exception ex) { Log("signer: " + ex.Message); return false; }
    }

    // ------------------------------------------------------------------ opening it

    /// <summary>Opens Hearth Browser (a page in it). game: Ember's full-screen Game Mode, where it fills the screen
    /// with bigger buttons.</summary>
    public static void Open(string? url, bool game)
    {
        var exe = Exe() ?? throw new InvalidOperationException("Hearth Browser isn't set up yet.");
        if (exe == OwnExe) { try { ApplyParts(); } catch (Exception ex) { Log("parts: " + ex.Message); } }
        Directory.CreateDirectory(ProfileDir);
        var psi = new ProcessStartInfo(exe) { UseShellExecute = false, WorkingDirectory = Path.GetDirectoryName(exe)! };
        psi.ArgumentList.Add("-profile");
        psi.ArgumentList.Add(ProfileDir);
        if (url is { Length: > 0 }) psi.ArgumentList.Add(url);
        foreach (var k in new[] { "HEARTH_BROWSER_GAME", "HEARTH_BROWSER_BIG", "HEARTH_BROWSER_SCALE", "HEARTH_BROWSER_ACCENT" }) psi.Environment.Remove(k);
        // Windows already draws apps at the screen's scale, so Game Mode only asks for the bigger buttons.
        if (game) { psi.Environment["HEARTH_BROWSER_GAME"] = "1"; psi.Environment["HEARTH_BROWSER_BIG"] = "1"; }
        Log($"open {(url is { Length: > 0 } ? url : "(home)")} with {exe}{(game ? " (Game Mode)" : "")}");
        Process.Start(psi)?.Dispose();
    }

    /// <summary>A web address Settings or Beacon may open: http(s) only.</summary>
    public static bool ValidUrl(string? url) =>
        url is { Length: > 0 and < 4096 } && Uri.TryCreate(url, UriKind.Absolute, out var u) && (u.Scheme == Uri.UriSchemeHttps || u.Scheme == Uri.UriSchemeHttp);

    /// <summary><c>Nimbus.exe --browser [--game] [--setup-only] [URL]</c>. Without Ember's copy it's set up first
    /// (an installed Firefox is used if that fails). --setup-only: set up (or refresh) the copy and stop.</summary>
    public static int RunCli(string[] args)
    {
        var rest = args.Where(a => !a.Equals("--browser", StringComparison.OrdinalIgnoreCase)).ToList();
        bool setupOnly = rest.RemoveAll(a => a.Equals("--setup-only", StringComparison.OrdinalIgnoreCase)) > 0;
        bool game = rest.RemoveAll(a => a.Equals("--game", StringComparison.OrdinalIgnoreCase)) > 0;
        try
        {
            if (!File.Exists(OwnExe))
            {
                try { InstallAsync().GetAwaiter().GetResult(); }
                catch (Exception ex) when (!setupOnly && SystemExe() is not null) { Log("setup failed, using the installed Firefox: " + ex.Message); }
            }
            else ApplyParts();
            if (setupOnly) return File.Exists(OwnExe) ? 0 : 1;
            Open(rest.FirstOrDefault(), game);
            return 0;
        }
        catch (Exception ex)
        {
            Log("failed: " + ex);
            return 1;
        }
    }

    private static void Log(string line)
    {
        SystemBridge.Log("hearth browser: " + line);
        try
        {
            Directory.CreateDirectory(Root);
            File.AppendAllText(LogPath, $"{DateTime.Now:yyyy-MM-dd HH:mm:ss} {line}{Environment.NewLine}");
        }
        catch { }
    }
}
