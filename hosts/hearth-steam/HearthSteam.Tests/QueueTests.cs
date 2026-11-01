using System.Text.Json.Nodes;
using HearthSteam;
using HearthSteam.Open;
using Xunit;

// The Downloads page's control over the Open engine: pause / resume one or all, move up / down / top, cancel, speed
// and time left, kept across restarts; automatic updates (off / when idle / always) and the speed limit.
public class QueueTests
{
    static DownloadQueue Three()
    {
        var q = new DownloadQueue();
        foreach (var (id, n) in new[] { (10u, "A"), (20u, "B"), (30u, "C") }) q.Add(new QueueItem { AppId = id, Name = n, Library = "/lib" });
        return q;
    }
    static uint[] Order(DownloadQueue q) => q.Items().Select(i => i.AppId).ToArray();

    [Fact]
    public void ReorderUpDownTop()
    {
        var q = Three();
        Assert.True(q.Move(30, "up"));
        Assert.Equal(new uint[] { 10, 30, 20 }, Order(q));
        Assert.True(q.Move(10, "down"));
        Assert.Equal(new uint[] { 30, 10, 20 }, Order(q));
        Assert.False(q.Move(30, "up"));            // already first
        Assert.False(q.Move(20, "down"));          // already last
        Assert.True(q.Move(20, "top"));
        Assert.Equal(new uint[] { 20, 30, 10 }, Order(q));
        Assert.False(q.Move(99, "top"));
        Assert.Throws<EngineException>(() => q.Move(10, "sideways"));
    }

    [Fact]
    public void PauseResumeOneAndAll()
    {
        var q = Three();
        Assert.Equal(10u, q.Next(false, false, false)!.AppId);
        Assert.True(q.Pause(10));
        Assert.False(q.Pause(10));
        Assert.Equal(20u, q.Next(false, false, false)!.AppId);   // the paused one waits, the next one runs
        Assert.Equal("paused", q.Answer()["downloads"]![0]!["state"]!.GetValue<string>());
        Assert.Equal(2, q.PauseAll());
        Assert.Null(q.Next(false, false, false));
        Assert.True(q.Answer()["paused"]!.GetValue<bool>());
        Assert.Equal(3, q.ResumeAll());
        Assert.Equal(10u, q.Next(false, false, false)!.AppId);
        // Moving a paused one to the top resumes it.
        q.Pause(30); q.Move(30, "top");
        Assert.Equal(30u, q.Next(false, false, false)!.AppId);
        // Standby / signed out: nothing runs.
        Assert.Null(q.Next(true, false, false));
    }

    [Fact]
    public void TheRunningJobYieldsWhenPausedMovedOrCancelled()
    {
        var q = Three();
        var a = q.Next(false, false, false)!;
        q.Started(a);
        Assert.False(q.ShouldYield(false, false, false));
        q.Move(20, "top");
        Assert.True(q.ShouldYield(false, false, false));        // B goes first now
        q.Ended(a, "waiting");
        Assert.Equal(new uint[] { 20, 10, 30 }, Order(q));     // A keeps its place and what it downloaded
        Assert.Equal("queued", a.State);
        var b = q.Next(false, false, false)!;
        q.Started(b);
        q.Pause(20);
        Assert.True(q.ShouldYield(false, false, false));
        q.Ended(b, "waiting");
        var c = q.Next(false, false, false)!;
        Assert.Equal(10u, c.AppId);
        q.Started(c);
        Assert.True(q.ShouldYield(true, false, false));         // standby
        Assert.True(q.Cancel(10));
        Assert.True(q.ShouldYield(false, false, false));        // cancelled: gone from the queue
        q.Ended(c, "cancelled");
        Assert.DoesNotContain(10u, Order(q));
        Assert.Contains(q.Answer()["downloads"]!.AsArray(), d => d!["appid"]!.GetValue<uint>() == 10 && d["state"]!.GetValue<string>() == "cancelled" && d["position"]!.GetValue<int>() == -1);
    }

    [Fact]
    public void FinishedAndFailedLeaveTheQueue()
    {
        var q = Three();
        var a = q.Next(false, false, false)!; q.Started(a); q.Ended(a, "done");
        var b = q.Next(false, false, false)!; q.Started(b); q.Ended(b, "failed", "No access to depot 21.");
        Assert.Equal(new uint[] { 30 }, Order(q));
        var all = q.Answer()["downloads"]!.AsArray();
        Assert.Equal("No access to depot 21.", all.Single(d => d!["appid"]!.GetValue<uint>() == 20)!["error"]!.GetValue<string>());
        // Installing it again takes it out of "finished".
        Assert.True(q.Add(new QueueItem { AppId = 20 }));
        Assert.Single(q.Answer()["downloads"]!.AsArray(), d => d!["appid"]!.GetValue<uint>() == 20);
    }

    [Fact]
    public void SpeedAndTimeLeft()
    {
        var i = new QueueItem { AppId = 1, Total = 10_000_000, Done = 4_000_000, Speed = 2_000_000 };
        Assert.Equal(3, i.EtaSeconds());
        Assert.Null(new QueueItem { AppId = 1, Total = 10, Done = 1, Speed = 0 }.EtaSeconds());
        Assert.Null(new QueueItem { AppId = 1, Total = 10, Done = 10, Speed = 5 }.EtaSeconds());
        var a = i.Answer(0);
        Assert.Equal(3, a["eta"]!.GetValue<long>());
        Assert.Equal(2_000_000, a["speed"]!.GetValue<long>());
    }

    [Fact]
    public void KeptAcrossRestartsInOrder()
    {
        var q = Three();
        q.Move(30, "top"); q.Pause(10);
        q.Add(new QueueItem { AppId = 40, Kind = "auto", Branch = "beta" });
        var saved = JsonNode.Parse(q.Save().ToJsonString())!.AsArray();   // as written to open-state.json
        var back = new DownloadQueue();
        Assert.Equal(3, back.Load(saved, i => i.AppId == 20));             // 20 finished by Valve's client meanwhile
        Assert.Equal(new uint[] { 30, 10, 40 }, Order(back));
        Assert.True(back.Items()[1].Paused);
        Assert.Equal("auto", back.Items()[2].Kind);
        Assert.Equal("beta", back.Items()[2].Branch);
        // 0.6.60's saved queue ({appid, library, verify}) still loads.
        var old = new DownloadQueue();
        old.Load(JsonNode.Parse("[{\"appid\":620,\"library\":\"/lib\",\"verify\":true},{\"appid\":0},null]")!.AsArray());
        Assert.Equal("verify", old.Items().Single().Kind);
    }

    [Fact]
    public void ManualInstallTakesOverAWaitingAutomaticUpdate()
    {
        var q = new DownloadQueue();
        Assert.True(q.Add(new QueueItem { AppId = 5, Kind = "auto", Paused = true }));
        Assert.False(q.Add(new QueueItem { AppId = 5, Kind = "verify" }));
        Assert.Equal("verify", q.Items().Single().Kind);
        Assert.False(q.Items().Single().Paused);
        Assert.False(q.Add(new QueueItem { AppId = 5, Kind = "auto" }));   // no double
        Assert.Single(q.Items());
    }

    [Fact]
    public void AutomaticUpdatesWaitForTheGame()
    {
        var q = new DownloadQueue();
        q.Add(new QueueItem { AppId = 1, Kind = "auto" });
        q.Add(new QueueItem { AppId = 2, Kind = "install" });
        // A game is running: the player's own install goes ahead, the automatic update waits (unless "always").
        Assert.Equal(2u, q.Next(false, true, AutoUpdate.DuringGame("idle"))!.AppId);
        Assert.Equal(1u, q.Next(false, true, AutoUpdate.DuringGame("always"))!.AppId);
        Assert.Equal(1u, q.Next(false, false, false)!.AppId);
        // An automatic update running when a game starts stops and waits.
        var a = q.Next(false, false, false)!; q.Started(a);
        Assert.True(q.ShouldYield(false, true, false));
        Assert.False(q.ShouldYield(false, true, true));
    }

    [Fact]
    public void WhenToCheckForUpdates()
    {
        var t0 = new DateTime(2026, 11, 1, 12, 0, 0, DateTimeKind.Utc);
        var never = DateTime.MinValue;
        Assert.False(AutoUpdate.CheckDue("off", false, never, t0));
        Assert.True(AutoUpdate.CheckDue("idle", false, never, t0));
        Assert.False(AutoUpdate.CheckDue("idle", true, never, t0), "never during a game");
        Assert.True(AutoUpdate.CheckDue("always", true, never, t0));
        Assert.False(AutoUpdate.CheckDue("idle", false, t0, t0 + TimeSpan.FromMinutes(5)), "checked lately");
        Assert.True(AutoUpdate.CheckDue("idle", false, t0, t0 + AutoUpdate.Every));
        Assert.Equal("idle", AutoUpdate.Normal(null));
        Assert.Equal("idle", AutoUpdate.Normal("whatever"));
        Assert.Equal("off", AutoUpdate.Normal("off"));
    }

    [Fact]
    public void WhichGamesNeedAnUpdate()
    {
        var installed = new Dictionary<uint, (uint Build, string Branch, bool Installed)>
        {
            [10] = (100, "", true), [20] = (200, "", true), [30] = (300, "beta", true), [40] = (400, "", false),
        };
        uint Latest(uint app, string branch) => (app, branch) switch { (10, "") => 101, (20, "") => 200, (30, "beta") => 301, (40, "") => 401, _ => 0 };
        // Only what changed since the last check (PICS change numbers).
        Assert.Equal(new uint[] { 10 }, AutoUpdate.Needing(installed, new HashSet<uint> { 10, 20, 99 }, false, Latest));
        // A full update (first check, or Steam says so): every installed game, its own branch.
        Assert.Equal(new uint[] { 10, 30 }, AutoUpdate.Needing(installed, new HashSet<uint>(), true, Latest));
        // Unknown latest build: left alone.
        Assert.Empty(AutoUpdate.Needing(installed, new HashSet<uint> { 10 }, false, (_, _) => 0));
    }

    [Fact]
    public void SpeedLimit()
    {
        var now = new DateTime(2026, 1, 1, 0, 0, 0, DateTimeKind.Utc);
        var r = new RateLimiter(() => now);
        Assert.Equal(TimeSpan.Zero, r.Take(100_000_000));   // no limit
        r.BytesPerSecond = 1_000_000;
        Assert.Equal(TimeSpan.Zero, r.Take(0));
        var wait = r.Take(2_000_000);                        // two seconds' worth
        Assert.InRange(wait.TotalSeconds, 1.99, 2.01);
        now += TimeSpan.FromSeconds(3);
        Assert.Equal(TimeSpan.Zero, r.Take(500_000));
        r.BytesPerSecond = 0;
        Assert.Equal(TimeSpan.Zero, r.Take(10_000_000));
    }

    [Fact]
    public void DownloadCommandsGoThroughTheSocket()
    {
        var e = new FakeOpenEngine();
        var s = new Server(e);
        JsonObject Ask(string l) => (JsonObject)JsonNode.Parse(s.Handle(l).Result)!;
        Assert.True(Ask("{\"cmd\":\"pause\",\"appid\":620}")["ok"]!.GetValue<bool>());
        Ask("{\"cmd\":\"pause\"}");
        Ask("{\"cmd\":\"resume\"}");
        Ask("{\"cmd\":\"move\",\"appid\":620,\"to\":\"top\"}");
        Ask("{\"cmd\":\"config\",\"autoUpdate\":\"always\",\"limitKBps\":2048,\"gameRunning\":true}");
        Assert.Equal(new[] { "pause 620", "pause all", "resume all", "move 620 top", "config always 2048 True" }, e.Calls);
        Assert.Equal(Codes.BadRequest, Ask("{\"cmd\":\"move\",\"appid\":620}")["code"]!.GetValue<string>());
        Assert.Equal(Codes.Unsupported, ((JsonObject)JsonNode.Parse(new Server(new FakeEngine()).Handle("{\"cmd\":\"pause\"}").Result)!)["code"]!.GetValue<string>());
    }
}
