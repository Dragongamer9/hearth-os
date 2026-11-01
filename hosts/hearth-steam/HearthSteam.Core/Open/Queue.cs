using System.Text.Json.Nodes;

namespace HearthSteam.Open;

// The Open engine's download queue as Hearth's Downloads page drives it: pause / resume one or all, move up / down /
// to the top, cancel, speed and time left, kept across restarts; automatic updates (off / when idle / always) and a
// bandwidth limit. Pure: the engine runs the jobs, the tests drive this directly.

public sealed class QueueItem
{
    public uint AppId { get; init; }
    public string Library { get; set; } = "";
    /// <summary>"install", "update", "verify", "auto" (an automatic update) or "workshop".</summary>
    public string Kind { get; set; } = "install";
    public string Name { get; set; } = "";
    /// <summary>The beta branch to install ("" / "public": the default branch).</summary>
    public string Branch { get; set; } = "";
    public bool Paused { get; set; }
    public string State { get; set; } = "queued";
    public string Error { get; set; } = "";
    public long Done, Total, Speed;
    public bool Verify => Kind == "verify";
    public bool Auto => Kind == "auto";

    public JsonObject Saved() => new()
    {
        ["appid"] = AppId, ["library"] = Library, ["kind"] = Kind, ["name"] = Name, ["paused"] = Paused,
        ["branch"] = Branch.Length > 0 ? Branch : null, ["verify"] = Verify,
    };

    public static QueueItem? Load(JsonNode? n)
    {
        if (n is not JsonObject o || o["appid"]?.GetValue<uint>() is not uint id || id == 0) return null;
        var kind = o["kind"]?.GetValue<string>() ?? (o["verify"]?.GetValue<bool>() == true ? "verify" : "install");
        if (kind is not ("install" or "update" or "verify" or "auto" or "workshop")) kind = "install";
        return new QueueItem
        {
            AppId = id, Library = o["library"]?.GetValue<string>() ?? "", Kind = kind, Name = o["name"]?.GetValue<string>() ?? "",
            Paused = o["paused"]?.GetValue<bool>() == true, Branch = o["branch"]?.GetValue<string>() ?? "",
        };
    }

    /// <summary>Seconds left at the current speed, or null when unknown.</summary>
    public long? EtaSeconds() => Speed > 0 && Total > 0 && Done < Total ? (Total - Done + Speed - 1) / Speed : null;

    public JsonObject Answer(int position) => new()
    {
        ["appid"] = AppId, ["name"] = Name, ["state"] = Paused && State is "queued" or "waiting" ? "paused" : State, ["paused"] = Paused,
        ["kind"] = Kind, ["auto"] = Auto, ["verify"] = Verify, ["branch"] = Branch.Length > 0 ? Branch : null,
        ["done"] = Done, ["total"] = Total, ["speed"] = Speed, ["eta"] = EtaSeconds(), ["position"] = position,
        ["error"] = Error.Length > 0 ? Error : null,
    };
}

/// <summary>The queue: one job runs at a time (the first one that isn't paused and may run now), the rest wait in
/// order. Thread-safe; every change is meant to be saved (<see cref="Save"/>).</summary>
public sealed class DownloadQueue
{
    readonly object gate = new();
    readonly List<QueueItem> items = new();
    readonly Dictionary<uint, QueueItem> finished = new();
    public QueueItem? Current { get; private set; }

    public List<QueueItem> Items() { lock (gate) return items.ToList(); }

    /// <summary>Adds a job (false: that app is already queued; a manual install/update/verify takes over a waiting
    /// automatic update of the same app).</summary>
    public bool Add(QueueItem item)
    {
        lock (gate)
        {
            var have = Find(item.AppId);
            if (have != null)
            {
                if (have.Auto && !item.Auto && have != Current) { have.Kind = item.Kind; have.Paused = false; if (item.Branch.Length > 0) have.Branch = item.Branch; }
                return false;
            }
            finished.Remove(item.AppId);
            items.Add(item);
            return true;
        }
    }

    QueueItem? Find(uint app) => items.FirstOrDefault(i => i.AppId == app);
    public bool Has(uint app) { lock (gate) return Find(app) != null; }

    public bool Pause(uint app)
    {
        lock (gate) { var i = Find(app); if (i == null || i.Paused) return false; i.Paused = true; return true; }
    }

    public bool Resume(uint app)
    {
        lock (gate) { var i = Find(app); if (i == null || !i.Paused) return false; i.Paused = false; if (i.State is "failed" or "cancelled") i.State = "queued"; return true; }
    }

    public int PauseAll() { lock (gate) { int n = 0; foreach (var i in items) if (!i.Paused) { i.Paused = true; n++; } return n; } }
    public int ResumeAll() { lock (gate) { int n = 0; foreach (var i in items) if (i.Paused) { i.Paused = false; n++; } return n; } }

    /// <summary>Moves a job "up", "down" or to the "top". The top one runs next (the running one is stopped and waits
    /// by the engine: see <see cref="ShouldYield"/>).</summary>
    public bool Move(uint app, string to)
    {
        lock (gate)
        {
            var i = Find(app);
            if (i == null) return false;
            int at = items.IndexOf(i);
            int dest = to switch { "top" => 0, "up" => Math.Max(0, at - 1), "down" => Math.Min(items.Count - 1, at + 1), _ => -1 };
            if (dest < 0) throw new EngineException(Codes.BadRequest, "Move it up, down or to the top.");
            if (dest == at) return false;
            items.RemoveAt(at);
            items.Insert(dest, i);
            if (to == "top") i.Paused = false;
            return true;
        }
    }

    /// <summary>Removes a job (what's downloaded is kept). True when it was there; the engine stops it when it's
    /// the running one.</summary>
    public bool Cancel(uint app)
    {
        lock (gate)
        {
            var i = Find(app);
            if (i == null) return false;
            items.Remove(i);
            i.State = "cancelled";
            finished[app] = i;
            return true;
        }
    }

    /// <summary>The job that should run now (null: none). Automatic updates wait while a game runs unless
    /// <paramref name="autoDuringGame"/>; nothing runs on <paramref name="hold"/> (standby, signed out).</summary>
    public QueueItem? Next(bool hold, bool gameRunning, bool autoDuringGame)
    {
        lock (gate)
        {
            if (hold) return null;
            return items.FirstOrDefault(i => !i.Paused && (!i.Auto || !gameRunning || autoDuringGame));
        }
    }

    public void Started(QueueItem i) { lock (gate) { Current = i; i.State = "preparing"; i.Error = ""; } }

    /// <summary>The running job should stop and wait (it was paused, moved down below another runnable job, or may
    /// not run now). Not for cancel: <see cref="Cancel"/> takes it out.</summary>
    public bool ShouldYield(bool hold, bool gameRunning, bool autoDuringGame)
    {
        lock (gate)
        {
            if (Current == null) return false;
            if (!items.Contains(Current)) return true;   // cancelled
            var next = Next(hold, gameRunning, autoDuringGame);
            return next != Current;
        }
    }

    /// <summary>The running job ended: "done" / "failed" leave the queue; "waiting" (yielded) stays where it is.</summary>
    public void Ended(QueueItem i, string state, string error = "")
    {
        lock (gate)
        {
            if (Current == i) Current = null;
            i.Speed = 0;
            if (state == "waiting") { i.State = "queued"; return; }
            i.State = state; i.Error = error;
            if (state is "done" or "failed" or "cancelled") { items.Remove(i); finished[i.AppId] = i; }
        }
    }

    public JsonArray Save() { lock (gate) return new JsonArray(items.Select(i => (JsonNode)i.Saved()).ToArray()); }

    /// <summary>Puts back a saved queue (in its order, paused ones paused), skipping what's already queued and what
    /// <paramref name="skip"/> says is done meanwhile (Valve's client may have finished it).</summary>
    public int Load(JsonArray? saved, Func<QueueItem, bool>? skip = null)
    {
        int n = 0;
        foreach (var e in saved ?? new JsonArray())
        {
            var i = QueueItem.Load(e);
            if (i == null || (skip?.Invoke(i) ?? false)) continue;
            if (Add(i)) n++;
        }
        return n;
    }

    public JsonObject Answer()
    {
        lock (gate)
        {
            var a = new JsonArray();
            int pos = 0;
            foreach (var i in items) a.Add(i.Answer(pos++));
            foreach (var i in finished.Values) a.Add(i.Answer(-1));
            long speed = Current?.Speed ?? 0;
            return new JsonObject { ["downloads"] = a, ["speed"] = speed, ["paused"] = items.Count > 0 && items.All(i => i.Paused) };
        }
    }
}

/// <summary>Automatic updates for installed games (Settings › Library: off / when idle / always). Steam's PICS change
/// numbers are checked now and then while Hearth runs; never during a game unless "always".</summary>
public static class AutoUpdate
{
    public const string Off = "off", Idle = "idle", Always = "always";
    public static readonly TimeSpan Every = TimeSpan.FromMinutes(20);

    public static string Normal(string? s) => s is Off or Always ? s : Idle;

    /// <summary>Is a check for updates due now?</summary>
    public static bool CheckDue(string policy, bool gameRunning, DateTime lastCheck, DateTime now) =>
        Normal(policy) != Off && (!gameRunning || Normal(policy) == Always) && now - lastCheck >= Every;

    /// <summary>May automatic update jobs run while a game is running?</summary>
    public static bool DuringGame(string policy) => Normal(policy) == Always;

    /// <summary>Which installed apps need an update: changed since the last check (or every one when Steam says a full
    /// update is needed) and their installed build isn't the branch's latest. <paramref name="latest"/> answers the
    /// branch's build from fresh app info (0: unknown, so no update).</summary>
    public static List<uint> Needing(IReadOnlyDictionary<uint, (uint Build, string Branch, bool Installed)> installed, ISet<uint> changed, bool full, Func<uint, string, uint> latest)
    {
        var list = new List<uint>();
        foreach (var (app, s) in installed)
        {
            if (!s.Installed || (!full && !changed.Contains(app))) continue;
            var l = latest(app, s.Branch);
            if (l != 0 && l != s.Build) list.Add(app);
        }
        list.Sort();
        return list;
    }
}

/// <summary>The download speed limit (Settings › Library; 0: none) as a token bucket: each chunk waits until its
/// bytes fit.</summary>
public sealed class RateLimiter
{
    readonly object gate = new();
    readonly Func<DateTime> clock;
    long limit;        // bytes per second, 0: none
    double tokens;
    DateTime last;

    public RateLimiter(Func<DateTime>? clock = null) { this.clock = clock ?? (() => DateTime.UtcNow); last = this.clock(); }
    public long BytesPerSecond { get { lock (gate) return limit; } set { lock (gate) { limit = Math.Max(0, value); tokens = Math.Min(tokens, limit); } } }

    /// <summary>How long to wait before sending <paramref name="bytes"/> (and takes them). Zero without a limit.</summary>
    public TimeSpan Take(long bytes)
    {
        lock (gate)
        {
            if (limit <= 0) return TimeSpan.Zero;
            var now = clock();
            tokens = Math.Min(limit, tokens + (now - last).TotalSeconds * limit);
            last = now;
            tokens -= bytes;
            return tokens >= 0 ? TimeSpan.Zero : TimeSpan.FromSeconds(-tokens / limit);
        }
    }

    public async Task Wait(long bytes, CancellationToken ct)
    {
        var d = Take(bytes);
        if (d > TimeSpan.Zero) await Task.Delay(d > TimeSpan.FromSeconds(30) ? TimeSpan.FromSeconds(30) : d, ct);
    }
}
