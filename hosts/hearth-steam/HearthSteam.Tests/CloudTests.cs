using System.IO.Compression;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json.Nodes;
using HearthSteam;
using HearthSteam.Open;
using Xunit;

/// <summary>A fake of Steam's Cloud service (as SteamKit2's unified Cloud messages answer): files with content, the
/// calls made in order, optional zipped downloads and failing uploads.</summary>
public sealed class FakeCloud : ICloudApi
{
    public Dictionary<string, (byte[] Data, long Time)> Files = new(StringComparer.OrdinalIgnoreCase);
    public List<string> Calls = new();
    public ulong Change = 100;
    public bool Zip;
    public string? FailUpload;
    ulong nextBatch = 7;

    public static string Sha(byte[] d) => Convert.ToHexString(SHA1.HashData(d)).ToLowerInvariant();

    public Task<CloudList> List(uint app, ulong since)
    {
        Calls.Add("list");
        return Task.FromResult(new CloudList(Change, Files.Select(f => new CloudFile(f.Key, Sha(f.Value.Data), f.Value.Time, f.Value.Data.Length)).ToList()));
    }

    public Task<byte[]> Download(uint app, string name)
    {
        Calls.Add("download " + name);
        var d = Files[name].Data;
        if (!Zip) return Task.FromResult(d);
        var ms = new MemoryStream();
        using (var z = new ZipArchive(ms, ZipArchiveMode.Create, true)) { using var s = z.CreateEntry("file").Open(); s.Write(d); }
        return Task.FromResult(ms.ToArray());
    }

    public Task<ulong> BeginBatch(uint app, IReadOnlyList<string> upload, IReadOnlyList<string> delete)
    {
        Calls.Add($"begin {string.Join(",", upload)} delete={delete.Count}");
        return Task.FromResult(nextBatch++);
    }

    public Task Upload(uint app, ulong batch, string name, byte[] data, byte[] sha, long time)
    {
        Calls.Add($"upload {batch} {name}");
        if (name == FailUpload) throw new EngineException(Codes.SteamError, "A save didn't go up.");
        Assert.Equal(Sha(data), Convert.ToHexString(sha).ToLowerInvariant());
        Files[name] = (data, time);
        Change++;
        return Task.CompletedTask;
    }

    public Task CompleteBatch(uint app, ulong batch, bool ok) { Calls.Add($"complete {batch} {(ok ? "ok" : "failed")}"); return Task.CompletedTask; }
}

// Steam Cloud for games Hearth starts directly: the save rules (ufs) from the app info, where each file goes (in the
// Proton prefix for Windows builds, Linux overrides, path tokens), what to download / upload, conflicts and the
// player's three answers, and the upload batch (begin, each file, complete; failures complete it as failed).
public class CloudTests
{
    static string Tmp() { var d = Path.Combine(Path.GetTempPath(), "hs-cloud-" + Guid.NewGuid().ToString("N")); Directory.CreateDirectory(d); return d; }
    static byte[] B(string s) => Encoding.UTF8.GetBytes(s);
    const ulong Me = 76561197960287930UL;   // account id 22202

    const string UfsText = """
        "ufs"
        {
            "quota" "100000000" "maxnumfiles" "50"
            "savefiles"
            {
                "0" { "root" "WinMyDocuments" "path" "My Games/Hollow/{64BitSteamID}" "pattern" "*.sav" "platforms" { "1" "Windows" } }
                "1" { "root" "GameInstall" "path" "saves" "pattern" "*" "recursive" "1" }
                "2" { "root" "WinAppDataLocalLow" "path" "Studio/Game" "pattern" "*.dat" "platforms" { "1" "Windows" } }
                "3" { "root" "MacAppSupport" "path" "Game" "pattern" "*" "platforms" { "1" "MacOS" } }
            }
            "rootoverrides"
            {
                "0" { "root" "WinAppDataLocalLow" "os" "Linux" "oscompare" "=" "useinstead" "LinuxXdgConfigHome" "addpath" "unity3d/Studio" }
            }
        }
        """;

    static Ufs Rules(string text = UfsText) => Ufs.Parse(Vdf.Parse(text).Get("ufs"));

    [Fact]
    public void ReadsTheSaveRules()
    {
        var u = Rules();
        Assert.True(u.HasCloud);
        Assert.Equal(4, u.Saves.Count);
        Assert.Equal("WinMyDocuments", u.Saves[0].Root);
        Assert.True(u.Saves[1].Recursive);
        Assert.Equal(new[] { "Windows" }, u.Saves[0].Platforms);
        Assert.Single(u.Overrides);
        Assert.False(Ufs.Parse(null).HasCloud);
        // Through the app info, too.
        var app = AppInfo.Parse(1, Vdf.Parse("\"appinfo\" { \"common\" { \"name\" \"X\" } " + UfsText + " }").Get("appinfo")!);
        Assert.Equal(4, app.Ufs.Saves.Count);
    }

    [Fact]
    public void WindowsSavesGoInsideTheProtonPrefix()
    {
        var p = new CloudPaths(Rules(), "/lib/steamapps/common/Hollow", "windows", "/lib/steamapps/compatdata/1/pfx", "/home/u", Me);
        Assert.Equal("/lib/steamapps/compatdata/1/pfx/drive_c/users/steamuser/Documents/My Games/Hollow/76561197960287930/a.sav",
            p.Local("%WinMyDocuments%My Games/Hollow/76561197960287930/a.sav"));
        Assert.Equal("/lib/steamapps/compatdata/1/pfx/drive_c/users/steamuser/AppData/LocalLow/Studio/Game/p.dat", p.Local("%WinAppDataLocalLow%Studio/Game/p.dat"));
        Assert.Equal("/lib/steamapps/common/Hollow/saves/slot1/x.bin", p.Local("%GameInstall%saves/slot1/x.bin"));
        Assert.Equal("/lib/steamapps/compatdata/1/pfx/drive_c/users/steamuser/AppData/Roaming/X/y", p.Local("%WinAppDataRoaming%X\\y"));
        // Never outside its root, never an unknown root, never Steam's legacy storage.
        Assert.Null(p.Local("%WinMyDocuments%../../../../etc/passwd"));
        Assert.Null(p.Local("%MacAppSupport%Game/a"));
        Assert.Null(p.Local("%LinuxHome%.bashrc"));
        Assert.Null(p.Local("remote_save.dat"));
        Assert.Equal("76561197960287930/22202", p.Tokens("{64BitSteamID}/{Steam3AccountID}"));
        Assert.True(p.Supported().Ok);
        Assert.Equal(3, p.Rules().Count());   // the Windows and all-platform rules, not the Mac one
    }

    [Fact]
    public void LinuxBuildsUseTheRootOverrides()
    {
        var p = new CloudPaths(Rules(), "/g", "linux", "/pfx", "/home/u", Me);
        Assert.Equal("/home/u/.config/unity3d/Studio/Game/p.dat", p.Local("%WinAppDataLocalLow%Game/p.dat"));
        Assert.Equal("/g/saves/a", p.Local("%GameInstall%saves/a"));
        Assert.Null(p.Local("%WinMyDocuments%My Games/a.sav"));   // no override for Linux: not placed
        Assert.True(p.Supported().Ok);                              // the Linux build's rules (all-platform) are placeable
        // A Windows-only rule without an override, for a Linux build that has only that rule: left to Valve's client.
        var only = Rules("\"ufs\" { \"quota\" \"1\" \"savefiles\" { \"0\" { \"root\" \"WinMyDocuments\" \"path\" \"G\" \"platforms\" { \"1\" \"Linux\" } } } }");
        var (ok, why) = new CloudPaths(only, "/g", "linux", "/pfx", "/home/u", Me).Supported();
        Assert.False(ok);
        Assert.Contains("WinMyDocuments", why);
        // Cloud on, no save rules at all (the game uses the Steamworks API): Valve's client.
        Assert.False(new CloudPaths(Rules("\"ufs\" { \"quota\" \"1000\" \"maxnumfiles\" \"10\" }"), "/g", "windows", "/pfx", "/h", Me).Supported().Ok);
        // No Steam Cloud: nothing to do, starts directly.
        Assert.True(new CloudPaths(new Ufs(), "/g", "windows", "/pfx", "/h", Me).Supported().Ok);
    }

    [Fact]
    public void FindsLocalSavesByTheRules()
    {
        var root = Tmp();
        var pfx = Path.Combine(root, "pfx"); var game = Path.Combine(root, "game");
        var docs = Path.Combine(pfx, "drive_c/users/steamuser/Documents/My Games/Hollow", Me.ToString());
        Directory.CreateDirectory(docs); Directory.CreateDirectory(Path.Combine(game, "saves/deep"));
        File.WriteAllText(Path.Combine(docs, "a.sav"), "A"); File.WriteAllText(Path.Combine(docs, "notes.txt"), "no");
        File.WriteAllText(Path.Combine(game, "saves/deep/b.bin"), "B");
        File.CreateSymbolicLink(Path.Combine(game, "saves/link"), "/etc/passwd");
        var names = new CloudPaths(Rules(), game, "windows", pfx, "/home/u", Me).Scan().Select(s => s.Name).OrderBy(n => n).ToList();
        Assert.Equal(new[] { "%GameInstall%saves/deep/b.bin", $"%WinMyDocuments%My Games/Hollow/{Me}/a.sav" }, names);
    }

    static (FakeCloud cloud, CloudSync sync, CloudPaths paths, string docs, string state, string copies) Setup()
    {
        var root = Tmp();
        var pfx = Path.Combine(root, "pfx"); var game = Path.Combine(root, "game");
        var docs = Path.Combine(pfx, "drive_c/users/steamuser/Documents/My Games/Hollow", Me.ToString());
        Directory.CreateDirectory(docs); Directory.CreateDirectory(game);
        var cloud = new FakeCloud();
        var logs = new List<string>();
        var sync = new CloudSync(cloud, Path.Combine(root, "state"), Path.Combine(root, "copies"), logs.Add);
        return (cloud, sync, new CloudPaths(Rules(), game, "windows", pfx, "/home/u", Me), docs, Path.Combine(root, "state"), Path.Combine(root, "copies"));
    }
    static string N(string f) => $"%WinMyDocuments%My Games/Hollow/{Me}/{f}";

    [Fact]
    public async Task NewerCloudSavesComeDownBeforeTheStart()
    {
        var (cloud, sync, paths, docs, _, _) = Setup();
        cloud.Files[N("a.sav")] = (B("cloud A"), 1_700_000_000);
        cloud.Zip = true;   // Steam serves some files zipped
        var r = await sync.Before(620, paths);
        Assert.Equal(1, r["downloaded"]!.GetValue<int>());
        Assert.Equal("cloud A", File.ReadAllText(Path.Combine(docs, "a.sav")));
        Assert.Equal(1_700_000_000, new DateTimeOffset(File.GetLastWriteTimeUtc(Path.Combine(docs, "a.sav"))).ToUnixTimeSeconds());
        // Nothing changed: nothing comes down again.
        cloud.Calls.Clear();
        await sync.Before(620, paths);
        Assert.DoesNotContain(cloud.Calls, c => c.StartsWith("download"));
    }

    [Fact]
    public async Task ChangedSavesGoUpInOneBatchAfterTheGame()
    {
        var (cloud, sync, paths, docs, _, _) = Setup();
        cloud.Files[N("a.sav")] = (B("v1"), 1);
        await sync.Before(620, paths);
        File.WriteAllText(Path.Combine(docs, "a.sav"), "v2 played");
        File.WriteAllText(Path.Combine(docs, "b.sav"), "new slot");
        cloud.Calls.Clear();
        var r = await sync.After(620, paths);
        Assert.Equal(2, r["uploaded"]!.GetValue<int>());
        Assert.Equal(new[] { "list", $"begin {N("a.sav")},{N("b.sav")} delete=0", $"upload 7 {N("a.sav")}", $"upload 7 {N("b.sav")}", "complete 7 ok" }, cloud.Calls);
        Assert.Equal("v2 played", Encoding.UTF8.GetString(cloud.Files[N("a.sav")].Data));
        // A second exit with nothing changed: no batch at all.
        cloud.Calls.Clear();
        await sync.After(620, paths);
        Assert.Equal(new[] { "list" }, cloud.Calls);
    }

    [Fact]
    public async Task AFailedUploadCompletesTheBatchAsFailed()
    {
        var (cloud, sync, paths, docs, _, _) = Setup();
        File.WriteAllText(Path.Combine(docs, "a.sav"), "A"); File.WriteAllText(Path.Combine(docs, "b.sav"), "B");
        cloud.FailUpload = N("b.sav");
        await Assert.ThrowsAsync<EngineException>(() => sync.After(620, paths));
        Assert.Equal("complete 7 failed", cloud.Calls.Last());
        // Next time the one that didn't go up still goes up (the record only has what Steam took).
        cloud.FailUpload = null; cloud.Calls.Clear();
        await sync.After(620, paths);
        Assert.Contains($"upload 8 {N("b.sav")}", cloud.Calls);
    }

    [Fact]
    public void WhatToDoWithEachFile()
    {
        LocalSave L(string n) => new(n, "/x/" + n, 1, 1);
        var known = new Dictionary<string, string> { ["same"] = "aa", ["cloudnew"] = "aa", ["localnew"] = "aa", ["both"] = "aa", ["gonecloud"] = "aa", ["gonelocal"] = "aa" };
        var remote = new[]
        {
            new CloudFile("same", "aa", 1, 1), new CloudFile("cloudnew", "bb", 1, 1), new CloudFile("localnew", "aa", 1, 1),
            new CloudFile("both", "cc", 1, 1), new CloudFile("gonelocal", "aa", 1, 1), new CloudFile("onlycloud", "dd", 1, 1),
            new CloudFile("GONECLOUD", "aa", 1, 1, Deleted: true), new CloudFile("firsttime", "e1", 1, 1),
        };
        var local = new[] { (L("same"), "aa"), (L("cloudnew"), "aa"), (L("localnew"), "ff"), (L("both"), "ee"), (L("gonecloud"), "aa"), (L("onlylocal"), "99"), (L("firsttime"), "e2") };
        var plan = CloudRules.Plan(remote, local, known);
        Assert.Equal(new[] { "cloudnew", "onlycloud" }, plan.Download.Select(f => f.Name).OrderBy(n => n));
        Assert.Equal(new[] { "localnew", "onlylocal" }, plan.Upload.Select(f => f.Name).OrderBy(n => n));
        Assert.Equal(new[] { "both", "firsttime" }, plan.Conflicts.OrderBy(n => n));   // never synced here and different: asked
        Assert.Equal(1, plan.Same);
        Assert.Equal(1, plan.RemoteDeleted);   // removed in the cloud: kept here, not deleted
        Assert.Equal(1, plan.LocalDeleted);    // removed here: kept in the cloud, not deleted
    }

    async Task<(FakeCloud, CloudSync, CloudPaths, string, string, string)> Conflicted()
    {
        var s = Setup();
        var (cloud, sync, paths, docs, _, _) = s;
        cloud.Files[N("a.sav")] = (B("base"), 1);
        await sync.Before(620, paths);
        File.WriteAllText(Path.Combine(docs, "a.sav"), "played here");
        cloud.Files[N("a.sav")] = (B("played elsewhere"), 2);
        return s;
    }

    [Fact]
    public async Task BothChangedAsksAndStartsNothing()
    {
        var (cloud, sync, paths, docs, _, _) = await Conflicted();
        cloud.Calls.Clear();
        var r = await sync.Before(620, paths);
        Assert.True(r["conflict"]!.GetValue<bool>());
        Assert.Equal(1, r["files"]!.GetValue<int>());
        Assert.DoesNotContain(cloud.Calls, c => c.StartsWith("download") || c.StartsWith("upload"));
        Assert.Equal("played here", File.ReadAllText(Path.Combine(docs, "a.sav")));   // untouched until answered
        Assert.NotNull(sync.Pending(620));
        // After the game, too (the cloud changed while playing): asked, nothing uploaded.
        Assert.True((await sync.After(620, paths))["conflict"]!.GetValue<bool>());
        Assert.DoesNotContain(cloud.Calls, c => c.StartsWith("upload"));
    }

    [Fact]
    public async Task KeepLocal()
    {
        var (cloud, sync, paths, docs, _, _) = await Conflicted();
        var r = await sync.Resolve(620, "local", paths);
        Assert.Equal(1, r["uploaded"]!.GetValue<int>());
        Assert.Equal("played here", Encoding.UTF8.GetString(cloud.Files[N("a.sav")].Data));
        Assert.Null(sync.Pending(620));
        Assert.False((await sync.Before(620, paths)).ContainsKey("conflict"));
    }

    [Fact]
    public async Task KeepCloudKeepsTheLocalOnesAside()
    {
        var (cloud, sync, paths, docs, state, copies) = await Conflicted();
        var r = await sync.Resolve(620, "cloud", paths);
        Assert.Equal("played elsewhere", File.ReadAllText(Path.Combine(docs, "a.sav")));
        Assert.False(r.ContainsKey("copies"));
        // This device's save isn't gone: it's in Hearth's state folder.
        Assert.Contains(Directory.EnumerateFiles(Path.Combine(state, "cloud-kept"), "*", SearchOption.AllDirectories), f => File.ReadAllText(f) == "played here");
        Assert.False(Directory.Exists(copies));
    }

    [Fact]
    public async Task KeepBothCopies()
    {
        var (cloud, sync, paths, docs, _, copies) = await Conflicted();
        var r = await sync.Resolve(620, "both", paths);
        Assert.Equal("played elsewhere", File.ReadAllText(Path.Combine(docs, "a.sav")));
        var where = r["copies"]!.GetValue<string>();
        Assert.StartsWith(copies, where);
        Assert.Contains(Directory.EnumerateFiles(where, "*", SearchOption.AllDirectories), f => File.ReadAllText(f) == "played here");
        Assert.Equal("played elsewhere", Encoding.UTF8.GetString(cloud.Files[N("a.sav")].Data));   // the cloud's stays
        await Assert.ThrowsAsync<EngineException>(() => sync.Resolve(620, "neither", paths));
    }

    [Fact]
    public async Task UnplaceableAppsAreLeftToValvesClient()
    {
        var (cloud, sync, _, _, _, _) = Setup();
        var none = new CloudPaths(Rules("\"ufs\" { \"quota\" \"1000\" }"), "/g", "windows", "/pfx", "/h", Me);
        var r = await sync.Before(1, none);
        Assert.False(r["supported"]!.GetValue<bool>());
        Assert.Empty(cloud.Calls);
    }

    [Fact]
    public void DownloadedContentIsChecked()
    {
        var d = B("save data");
        Assert.Equal(d, CloudRules.Content(d, FakeCloud.Sha(d)));
        Assert.Null(CloudRules.Content(B("tampered"), FakeCloud.Sha(d)));
        var ms = new MemoryStream();
        using (var z = new ZipArchive(ms, ZipArchiveMode.Create, true)) { using var s = z.CreateEntry("f").Open(); s.Write(d); }
        Assert.Equal(d, CloudRules.Content(ms.ToArray(), FakeCloud.Sha(d)));
    }

    [Fact]
    public async Task LogsCountsNotFileNames()
    {
        var root = Tmp();
        var logs = new List<string>();
        var cloud = new FakeCloud();
        var pfx = Path.Combine(root, "pfx");
        var docs = Path.Combine(pfx, "drive_c/users/steamuser/Documents/My Games/Hollow", Me.ToString());
        Directory.CreateDirectory(docs);
        File.WriteAllText(Path.Combine(docs, "secret-name.sav"), "x");
        var sync = new CloudSync(cloud, Path.Combine(root, "state"), Path.Combine(root, "copies"), logs.Add);
        await sync.After(620, new CloudPaths(Rules(), Path.Combine(root, "g"), "windows", pfx, "/h", Me));
        Assert.NotEmpty(logs);
        Assert.DoesNotContain(logs, l => l.Contains("secret-name") || l.Contains("Hollow"));
    }

    [Fact]
    public void CloudCommandsGoThroughTheSocket()
    {
        var e = new FakeOpenEngine();
        var s = new Server(e);
        foreach (var l in new[] { "{\"cmd\":\"cloud.before\",\"appid\":620}", "{\"cmd\":\"cloud.after\",\"appid\":620}", "{\"cmd\":\"cloud.resolve\",\"appid\":620,\"choice\":\"both\"}" })
            Assert.True(((JsonObject)JsonNode.Parse(s.Handle(l).Result)!)["ok"]!.GetValue<bool>());
        Assert.Equal(new[] { "cloud.before 620", "cloud.after 620", "cloud.resolve 620 both" }, e.Calls);
    }
}
