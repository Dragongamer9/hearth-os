using System.Text.Json;
using System.Xml.Linq;
using Microsoft.Win32;
using Windows.Management.Deployment;

namespace Nimbus.Host.Library;

/// <summary>Epic Games Launcher installs, from the launcher's manifest files.</summary>
public static class EpicScanner
{
    public static List<Game> Scan()
    {
        var games = new List<Game>();
        var dir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.CommonApplicationData),
                               "Epic", "EpicGamesLauncher", "Data", "Manifests");
        if (!Directory.Exists(dir)) return games;

        foreach (var file in Directory.EnumerateFiles(dir, "*.item"))
        {
            try
            {
                using var doc = JsonDocument.Parse(File.ReadAllText(file));
                var m = doc.RootElement;
                string? S(string k) => m.TryGetProperty(k, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString() : null;

                if (m.TryGetProperty("bIsIncompleteInstall", out var inc) && inc.ValueKind == JsonValueKind.True) continue;
                // DLC and engines have their own manifests; keep only things categorised as games.
                if (m.TryGetProperty("AppCategories", out var cats) && cats.ValueKind == JsonValueKind.Array &&
                    !cats.EnumerateArray().Any(c => c.GetString() == "games")) continue;

                var appName = S("AppName"); var title = S("DisplayName"); var install = S("InstallLocation");
                var ns = S("CatalogNamespace"); var item = S("CatalogItemId");
                if (appName is null || title is null || install is null) continue;

                long size = m.TryGetProperty("InstallSize", out var sz) && sz.TryGetInt64(out var n) ? n : 0;
                var uri = ns is not null && item is not null
                    ? $"com.epicgames.launcher://apps/{ns}%3A{item}%3A{appName}?action=launch&silent=true"
                    : $"com.epicgames.launcher://apps/{appName}?action=launch&silent=true";

                var g = new Game
                {
                    Id = "epic:" + appName,
                    Title = title,
                    Source = "epic",
                    InstallPath = install,
                    SizeBytes = size > 0 ? size : null,
                    Added = Time.Ms(Directory.Exists(install) ? Directory.GetCreationTimeUtc(install) : File.GetCreationTimeUtc(file)),
                    LaunchUri = uri,
                };
                g.ProcessDirs.Add(install);
                games.Add(g);
            }
            catch { /* skip unreadable manifest */ }
        }
        return games;
    }
}

/// <summary>GOG installs (Galaxy or offline installers), from the registry.</summary>
public static class GogScanner
{
    public static List<Game> Scan()
    {
        var games = new List<Game>();
        using var hklm = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, RegistryView.Registry32);
        using var root = hklm.OpenSubKey(@"SOFTWARE\GOG.com\Games");
        if (root is null) return games;

        foreach (var id in root.GetSubKeyNames())
        {
            using var k = root.OpenSubKey(id);
            if (k is null) continue;
            var title = k.GetValue("gameName") as string;
            var exe = k.GetValue("exe") as string;
            var path = k.GetValue("path") as string;
            if (string.IsNullOrWhiteSpace(title) || string.IsNullOrWhiteSpace(exe) || !File.Exists(exe)) continue;
            if (!string.IsNullOrEmpty(k.GetValue("dependsOn") as string)) continue; // DLC

            var work = k.GetValue("workingDir") as string;
            var g = new Game
            {
                Id = "gog:" + (k.GetValue("gameID") as string ?? id),
                Title = title,
                Source = "gog",
                InstallPath = path,
                Added = Time.Ms(Directory.Exists(path) ? Directory.GetCreationTimeUtc(path!) : File.GetCreationTimeUtc(exe)),
                Exe = exe,
                Args = k.GetValue("launchParam") as string,
                WorkDir = string.IsNullOrEmpty(work) ? Path.GetDirectoryName(exe) : work,
            };
            g.ProcessDirs.Add(string.IsNullOrEmpty(path) ? Path.GetDirectoryName(exe)! : path);
            games.Add(g);
        }
        return games;
    }
}

/// <summary>Xbox app / Game Pass PC games: Store packages that ship a MicrosoftGame.config.</summary>
public static class XboxScanner
{
    public static List<Game> Scan()
    {
        var games = new List<Game>();
        IEnumerable<Windows.ApplicationModel.Package> packages;
        try { packages = new PackageManager().FindPackagesForUser(string.Empty); }
        catch { return games; }

        foreach (var p in packages)
        {
            try
            {
                if (p.IsFramework || p.IsResourcePackage || p.IsBundle) continue;
                string dir;
                try { dir = p.InstalledPath; } catch { dir = p.InstalledLocation.Path; }

                var config = Path.Combine(dir, "MicrosoftGame.config");
                if (!File.Exists(config)) continue;

                var g = new Game
                {
                    Id = "xbox:" + p.Id.FamilyName,
                    Title = string.IsNullOrWhiteSpace(p.DisplayName) ? p.Id.Name : p.DisplayName,
                    Source = "xbox",
                    InstallPath = dir,
                    Added = p.InstalledDate.ToUnixTimeMilliseconds(),
                    XboxFamily = p.Id.FamilyName,
                };
                g.ProcessDirs.Add(dir);

                try
                {
                    // <ShellVisuals Square480x480Logo="..." SplashScreenImage="..." />
                    var visuals = XDocument.Load(config).Descendants().FirstOrDefault(e => e.Name.LocalName == "ShellVisuals");
                    void Art(string kind, string attr)
                    {
                        var rel = visuals?.Attribute(attr)?.Value;
                        if (string.IsNullOrEmpty(rel)) return;
                        var f = Path.Combine(dir, rel);
                        if (File.Exists(f)) g.Art[kind] = f;
                    }
                    Art("cover", "Square480x480Logo");
                    Art("hero", "SplashScreenImage");
                    Art("wide", "SplashScreenImage");
                }
                catch { /* art is optional */ }

                games.Add(g);
            }
            catch { /* one bad package shouldn't hide the rest */ }
        }
        return games;
    }
}
