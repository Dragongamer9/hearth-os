using System.Security.Cryptography;
using System.Text.Json.Nodes;
using HearthSteam;
using HearthSteam.Open;
using Xunit;

// Beta branches (the list from PICS, picking one, a password-protected beta's manifests, BetaKey in the appmanifest
// like Steam) and Workshop items (where they go, appworkshop_<app>.acf, what a sync downloads and removes).
public class WorkshopTests
{
    static string Tmp() { var d = Path.Combine(Path.GetTempPath(), "hs-ws-" + Guid.NewGuid().ToString("N")); Directory.CreateDirectory(d); return d; }

    static readonly byte[] Key = Enumerable.Range(1, 32).Select(i => (byte)i).ToArray();
    static string Encrypt(ulong gid)
    {
        using var aes = Aes.Create(); aes.Key = Key;
        return Convert.ToHexString(aes.EncryptEcb(BitConverter.GetBytes(gid), PaddingMode.PKCS7));
    }

    static string App() => $$"""
        "appinfo"
        {
            "common" { "name" "Hollow" "type" "Game" }
            "config" { "installdir" "Hollow" }
            "depots"
            {
                "11" { "config" { "oslist" "windows" } "manifests" { "public" { "gid" "1001" } "beta" { "gid" "2001" } } "encryptedmanifests" { "secret" { "encrypted_gid_2" "{{Encrypt(3001)}}" } } }
                "workshopdepot" "10"
                "branches"
                {
                    "public" { "buildid" "100" "timeupdated" "1700000000" }
                    "beta" { "buildid" "200" "description" "Next update" }
                    "secret" { "buildid" "300" "pwdrequired" "1" "description" "Testers" }
                }
            }
        }
        """;

    static AppInfo Parse(string branch = "public") => AppInfo.Parse(10, Vdf.Parse(App()).Get("appinfo")!, branch);

    [Fact]
    public void ListsBranchesDefaultFirst()
    {
        var offer = Betas.Offer(Parse());
        Assert.Equal(new[] { "public", "beta", "secret" }, offer.Select(b => b.Name));
        Assert.True(offer[2].PasswordRequired);
        Assert.Equal("Next update", offer[1].Description);
        Assert.Equal(200u, Parse().BuildFor("beta"));
        Assert.Equal(100u, Parse().BuildFor(""));
        Assert.Equal(0u, Parse().BuildFor("nope"));
        Assert.Equal(10u, Parse().WorkshopDepot);
    }

    [Fact]
    public void PicksABranch()
    {
        var app = Parse();
        Assert.Equal("", Betas.Pick(app, "public"));
        Assert.Equal("", Betas.Pick(app, null));
        Assert.Equal("beta", Betas.Pick(app, "BETA"));
        Assert.Throws<EngineException>(() => Betas.Pick(app, "nightly"));
        // A public beta's own manifests and build.
        var beta = Parse("beta");
        Assert.Equal(2001UL, beta.Depots.Single().Manifest);
        Assert.Equal(200u, beta.BuildId);
    }

    [Fact]
    public void APasswordProtectedBetaUnlocksWithItsKey()
    {
        var secret = Parse("secret");
        var d = secret.Depots.Single();
        Assert.Equal(0UL, d.Manifest);
        Assert.True(d.Encrypted);
        Assert.Empty(secret.PickDepots("windows", "english", _ => true));   // nothing to download without the key
        Assert.Equal(3001UL, Betas.DecryptGid(d.EncryptedGid, Key));
        var open = Betas.WithKey(secret, Key);
        Assert.Equal(3001UL, open.Depots.Single().Manifest);
        Assert.Single(open.PickDepots("windows", "english", _ => true));
        Assert.Same(secret, Betas.WithKey(secret, null));
    }

    [Fact]
    public void TheBranchIsRecordedLikeSteam()
    {
        var lib = Tmp();
        new AppManifest { AppId = 10, Name = "Hollow", InstallDir = "Hollow", StateFlags = 4, BetaKey = "beta" }.Save(lib);
        var text = File.ReadAllText(AppManifest.Path(lib, 10));
        var v = Vdf.Parse(text).Get("AppState")!;
        Assert.Equal("beta", v.Str("UserConfig", "BetaKey"));
        Assert.Equal("beta", v.Str("MountedConfig", "BetaKey"));
        Assert.Equal("beta", AppManifest.Load(lib, 10)!.BetaKey);
        // The default branch: no BetaKey at all (as Steam writes it).
        new AppManifest { AppId = 11, Name = "X", InstallDir = "X", StateFlags = 4 }.Save(lib);
        Assert.DoesNotContain("BetaKey", File.ReadAllText(AppManifest.Path(lib, 11)));
        new AppManifest { AppId = 12, Name = "X", InstallDir = "X", StateFlags = 4, BetaKey = "public" }.Save(lib);
        Assert.DoesNotContain("BetaKey", File.ReadAllText(AppManifest.Path(lib, 12)));
    }

    [Fact]
    public void WorkshopItemsGoWhereSteamPutsThem()
    {
        var lib = Tmp();
        Assert.Equal(Path.Combine(lib, "steamapps", "workshop", "content", "10", "123"), WorkshopManifest.ContentDir(lib, 10, 123));
        var m = new WorkshopManifest { AppId = 10, LastBuildId = 100, TimeLastUpdated = 1700000500, SubscribedBy = 22202 };
        m.Items[123] = (5000, 1700000000, 777);
        m.Items[456] = (10, 1600000000, 0);
        m.Save(lib);
        var text = File.ReadAllText(WorkshopManifest.Path(lib, 10));
        var v = Vdf.Parse(text).Get("AppWorkshop")!;
        Assert.Equal("10", v.Str("appid"));
        Assert.Equal("5010", v.Str("SizeOnDisk"));
        Assert.Equal("777", v.Str("WorkshopItemsInstalled", "123", "manifest"));
        Assert.Equal("1700000000", v.Str("WorkshopItemsInstalled", "123", "timeupdated"));
        Assert.Equal("22202", v.Str("WorkshopItemDetails", "123", "subscribedby"));
        var back = WorkshopManifest.Load(lib, 10);
        Assert.Equal(2, back.Items.Count);
        Assert.Equal((5000UL, 1700000000L, 777UL), back.Items[123]);
        Assert.Equal(22202u, back.SubscribedBy);
        Assert.Empty(WorkshopManifest.Load(lib, 99).Items);
    }

    [Fact]
    public void WhatASyncDownloadsAndRemoves()
    {
        var m = new WorkshopManifest { AppId = 10 };
        m.Items[1] = (10, 100, 11);     // up to date
        m.Items[2] = (10, 100, 21);     // updated since
        m.Items[3] = (10, 100, 31);     // unsubscribed
        var subs = new List<WorkshopItem>
        {
            new(1, "Same", 11, 10, 100), new(2, "Updated", 22, 10, 200), new(4, "New", 41, 10, 50),
            new(5, "Old file", 0, 3, 60, "https://example.invalid/f", "map.bsp"), new(6, "Nothing to get", 0, 0, 1),
        };
        var (get, remove) = WorkshopRules.Plan(subs, m);
        Assert.Equal(new ulong[] { 2, 4, 5 }, get.Select(i => i.Id));
        Assert.Equal(new ulong[] { 3 }, remove);
        // No list from Steam, or an empty one: nothing is ever removed.
        Assert.Empty(WorkshopRules.Plan(null, m).Remove);
        Assert.Empty(WorkshopRules.Plan(new List<WorkshopItem>(), m).Remove);
    }

    [Fact]
    public void RemovingAnItemStaysInsideItsFolder()
    {
        var lib = Tmp();
        var m = new WorkshopManifest { AppId = 10 };
        var dir = WorkshopManifest.ContentDir(lib, 10, 3);
        Directory.CreateDirectory(dir); File.WriteAllText(Path.Combine(dir, "a"), "1");
        m.Items[3] = (1, 1, 1);
        WorkshopRules.Remove(lib, m, 3);
        Assert.False(Directory.Exists(dir));
        Assert.Empty(m.Items);
        // A link as an item's folder: the link goes, what it points at stays.
        var outside = Tmp(); File.WriteAllText(Path.Combine(outside, "keep"), "1");
        Directory.CreateDirectory(Path.GetDirectoryName(WorkshopManifest.ContentDir(lib, 10, 4))!);
        Directory.CreateSymbolicLink(WorkshopManifest.ContentDir(lib, 10, 4), outside);
        m.Items[4] = (1, 1, 1);
        WorkshopRules.Remove(lib, m, 4);
        Assert.True(File.Exists(Path.Combine(outside, "keep")));
        Assert.Equal("map.bsp", WorkshopRules.FileName("maps\\map.bsp", 1));
        Assert.Equal("7.bin", WorkshopRules.FileName("..", 7));
        Assert.Equal("7.bin", WorkshopRules.FileName("", 7));
    }

    [Fact]
    public void BetaAndWorkshopCommandsGoThroughTheSocket()
    {
        var e = new FakeOpenEngine();
        var logged = new List<string>();
        var s = new Server(e, logged.Add);
        foreach (var l in new[]
        {
            "{\"cmd\":\"betas\",\"appid\":10}", "{\"cmd\":\"beta.password\",\"appid\":10,\"password\":\"hunter2\"}", "{\"cmd\":\"beta.set\",\"appid\":10,\"branch\":\"beta\"}",
            "{\"cmd\":\"workshop\",\"appid\":10}", "{\"cmd\":\"workshop.sync\",\"appid\":10}", "{\"cmd\":\"workshop.unsubscribe\",\"appid\":10,\"item\":\"3000000001\"}",
        })
            Assert.True(((JsonObject)JsonNode.Parse(s.Handle(l).Result)!)["ok"]!.GetValue<bool>(), l);
        Assert.Equal(new[] { "betas 10", "beta.password 10", "beta.set 10 beta", "workshop 10", "workshop.sync 10", "workshop.unsubscribe 10 3000000001" }, e.Calls);
        Assert.DoesNotContain(logged, l => l.Contains("hunter2"));
        Assert.Equal(Codes.BadRequest, ((JsonObject)JsonNode.Parse(s.Handle("{\"cmd\":\"workshop.unsubscribe\",\"appid\":10}").Result)!)["code"]!.GetValue<string>());
    }
}
