// Adapted from OpenSteamClient (c#-remake), OpenSteamworks.Client/Startup/Bootstrapper.cs.
// Copyright (c) 2023 Onni Kukkonen. MIT licensed: see hosts/hearth-steam/LICENSE.OpenSteamClient.
// Hearth's changes: Linux only; no Steam web UI, Big Picture, friends UI, CEF or runtime packages (the runtime is
// linked from the user's own Steam); the datalinks it changes are remembered and put back on exit.

using System.Diagnostics;
using System.IO.Compression;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json.Nodes;

namespace HearthSteam.Vendor;

internal static class Bootstrap
{
    const string BaseUrl = "https://client-update.akamai.steamstatic.com/";
    static readonly HttpClient Http = new() { Timeout = TimeSpan.FromMinutes(30) };

    /// <summary>Packages left out: Steam's own UI (web, Big Picture, friends), its browser (CEF) and its runtimes.</summary>
    public static bool Skip(string package) =>
        package.StartsWith("tenfoot_") || package.StartsWith("resources_") || package.StartsWith("friendsui_") ||
        package.StartsWith("steamui_") || package.StartsWith("webkit_") || package.StartsWith("runtime_") ||
        (package.StartsWith("public_") && !package.StartsWith("public_all"));

    /// <summary>Downloads Valve's own client packages of the pinned build from Valve's update server, checks each
    /// file's SHA-256 from the manifest, and unpacks them into <paramref name="dir"/>.</summary>
    public static async Task Run(string manifestText, string dir, string userSteam, Action<string, int> progress, CancellationToken ct)
    {
        var packages = Path.Combine(dir, "package");
        Directory.CreateDirectory(packages);
        var root = Vdf.Parse(manifestText).Get("ubuntu12") ?? throw new InvalidDataException("No ubuntu12 manifest.");
        var wanted = root.Children.Where(p => p.Value.Children.Count > 0 && !Skip(p.Key) && p.Value.Str("IsBootstrapperPackage") != "1"
                                              && p.Value.Str("file") != null && p.Value.Str("sha2") != null).ToList();
        int n = 0;
        foreach (var (name, p) in wanted)
        {
            ct.ThrowIfCancellationRequested();
            var file = p.Str("file")!;
            if (file.Contains('/') || file.Contains("..")) throw new InvalidDataException("Bad package name.");
            var save = Path.Combine(packages, file);
            var sha = p.Str("sha2")!.ToUpperInvariant();
            progress($"Downloading Steam's {name}", n * 100 / wanted.Count);
            for (int attempt = 0; ; attempt++)
            {
                if (!File.Exists(save))
                {
                    using var resp = await Http.GetAsync(BaseUrl + file, HttpCompletionOption.ResponseHeadersRead, ct);
                    resp.EnsureSuccessStatusCode();
                    await using var fs = new FileStream(save + ".part", FileMode.Create, FileAccess.Write);
                    await resp.Content.CopyToAsync(fs, ct);
                    fs.Close();
                    File.Move(save + ".part", save, true);
                }
                string got;
                await using (var rs = File.OpenRead(save)) got = Convert.ToHexString(await SHA256.HashDataAsync(rs, ct));
                if (got == sha) break;
                File.Delete(save);
                if (attempt >= 2) throw new InvalidDataException($"Steam's {name} didn't download correctly.");
            }
            n++;
        }
        progress("Unpacking Steam", 95);
        foreach (var (_, p) in wanted) ZipFile.ExtractToDirectory(Path.Combine(packages, p.Str("file")!), dir, true);
        foreach (var sub in new[] { "linux64", "linux32", "ubuntu12_32", "ubuntu12_64" })
        {
            var d = Path.Combine(dir, sub);
            if (!Directory.Exists(d)) continue;
            foreach (var f in Directory.EnumerateFiles(d, "*", SearchOption.AllDirectories))
                try { File.SetUnixFileMode(f, File.GetUnixFileMode(f) | UnixFileMode.UserExecute | UnixFileMode.UserRead); } catch (Exception) { }
        }
        LinkUserSteam(dir, userSteam);
        File.WriteAllText(Path.Combine(packages, SteamFiles.ManifestName + ".manifest"), manifestText);
        // Downloaded zips are no longer needed once unpacked.
        foreach (var (_, p) in wanted) try { File.Delete(Path.Combine(packages, p.Str("file")!)); } catch (IOException) { }
    }

    /// <summary>The user's games, Steam runtime and Proton builds, shared from their normal Steam (links only).</summary>
    public static void LinkUserSteam(string dir, string userSteam)
    {
        void Link(string rel, string target)
        {
            var at = Path.Combine(dir, rel);
            if (Directory.Exists(target) && !Directory.Exists(at) && !File.Exists(at))
            {
                Directory.CreateDirectory(Path.GetDirectoryName(at)!);
                Directory.CreateSymbolicLink(at, target);
            }
        }
        Link("steamapps", Path.Combine(userSteam, "steamapps"));
        Link("ubuntu12_32/steam-runtime", Path.Combine(userSteam, "ubuntu12_32", "steam-runtime"));
        Link("compatibilitytools.d", Path.Combine(userSteam, "compatibilitytools.d"));
    }

    // ---------------------------------------------------------------- ~/.steam links (games find Steam through them)

    static string StateFile(string state) => Path.Combine(state, "links.json");

    /// <summary>Points ~/.steam/sdk32, sdk64 and steam.pid at this engine, remembering what they were. Only links
    /// are changed, never real folders. The normal Steam client re-points them itself whenever it starts.</summary>
    public static void TakeDatalinks(string home, string dir, string state)
    {
        var link = Path.Combine(home, ".steam");
        Directory.CreateDirectory(link);
        Directory.CreateDirectory(state);
        var saved = new JsonObject();
        foreach (var (name, target) in new[] { ("sdk64", Path.Combine(dir, "linux64")), ("sdk32", Path.Combine(dir, "linux32")) })
        {
            var at = Path.Combine(link, name);
            var info = new DirectoryInfo(at);
            if (info.Exists && info.LinkTarget == null) continue;   // a real folder: leave it alone
            saved[name] = info.LinkTarget;
            if (info.LinkTarget != null) info.Delete();
            Directory.CreateSymbolicLink(at, target);
        }
        var pid = Path.Combine(link, "steam.pid");
        saved["steam.pid"] = File.Exists(pid) ? File.ReadAllText(pid) : null;
        File.WriteAllText(StateFile(state), saved.ToJsonString());
        File.WriteAllText(pid, Environment.ProcessId.ToString());
    }

    /// <summary>Puts back what TakeDatalinks changed (also after a crash: called at start).</summary>
    public static void RestoreDatalinks(string home, string state)
    {
        var f = StateFile(state);
        if (!File.Exists(f)) return;
        try
        {
            var saved = JsonNode.Parse(File.ReadAllText(f)) as JsonObject ?? new JsonObject();
            var link = Path.Combine(home, ".steam");
            foreach (var name in new[] { "sdk64", "sdk32" })
            {
                if (!saved.ContainsKey(name)) continue;
                var at = new DirectoryInfo(Path.Combine(link, name));
                if (at.LinkTarget == null) continue;
                at.Delete();
                if (saved[name]?.GetValue<string>() is string old) Directory.CreateSymbolicLink(at.FullName, old);
            }
            var pid = Path.Combine(link, "steam.pid");
            if (saved.ContainsKey("steam.pid"))
            {
                if (saved["steam.pid"]?.GetValue<string>() is string old) File.WriteAllText(pid, old); else File.Delete(pid);
            }
        }
        catch (Exception e) { Console.Error.WriteLine($"hearth-steam: couldn't put ~/.steam back: {e.Message}"); }
        File.Delete(f);
    }

    // ---------------------------------------------------------------- loading steamclient.so

    public static void SetEnvsForSteamLoad(string dir)
    {
        Environment.SetEnvironmentVariable("SteamAppId", "");
        Environment.SetEnvironmentVariable("SteamPath", dir);
        Environment.SetEnvironmentVariable("ValvePlatformMutex", dir.ToLowerInvariant());
        Environment.SetEnvironmentVariable("BREAKPAD_DUMP_LOCATION", Path.Combine(dir, "dumps"));
    }

    [DllImport("libc", SetLastError = true)]
    static extern int execv([MarshalAs(UnmanagedType.LPUTF8Str)] string file, [MarshalAs(UnmanagedType.LPArray, ArraySubType = UnmanagedType.LPUTF8Str)] string?[] args);

    /// <summary>steamclient.so needs Steam's libraries and the Steam runtime's on LD_LIBRARY_PATH, which the
    /// loader only reads at process start: so the process starts itself again once (HEARTH_STEAM_REEXEC=1).</summary>
    public static void ReExecWithEnvs(string dir)
    {
        if (Environment.GetEnvironmentVariable("HEARTH_STEAM_REEXEC") == "1") return;
        string runtimePaths = "";
        var run = Path.Combine(dir, "ubuntu12_32", "steam-runtime", "run.sh");
        if (File.Exists(run))
        {
            var p = new Process { StartInfo = new ProcessStartInfo(run, "--print-steam-runtime-library-paths") { RedirectStandardOutput = true, UseShellExecute = false } };
            p.Start();
            runtimePaths = p.StandardOutput.ReadToEnd().Trim();
            p.WaitForExit();
        }
        var ld = string.Join(':', new[] { Path.Combine(dir, "ubuntu12_64"), Path.Combine(dir, "ubuntu12_32"), dir, Path.Combine(dir, "linux64"), runtimePaths }.Where(s => s.Length > 0));
        Environment.SetEnvironmentVariable("STEAM_RUNTIME_LIBRARY_PATH", runtimePaths);
        Environment.SetEnvironmentVariable("LD_LIBRARY_PATH", ld);
        Environment.SetEnvironmentVariable("HEARTH_STEAM_REEXEC", "1");
        var exe = Directory.ResolveLinkTarget("/proc/self/exe", false)?.FullName ?? Environment.ProcessPath!;
        var args = Environment.GetCommandLineArgs().Skip(1).Prepend(exe).Append(null).ToArray();
        execv(exe, args);
        throw new InvalidOperationException($"Couldn't restart with Steam's libraries (errno {Marshal.GetLastWin32Error()}).");
    }
}
