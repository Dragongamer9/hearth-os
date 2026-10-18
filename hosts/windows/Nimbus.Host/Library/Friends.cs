using System.Collections.Concurrent;
using System.Text.RegularExpressions;

namespace Nimbus.Host.Library;

/// <summary>
/// Steam friends: who's online and what they're playing. The list comes from the Steam client on this PC (its
/// cached friends in localconfig.vdf, no Steam Web API key needed); each friend's state from their public Steam
/// Community profile (?xml=1), fetched in the background and kept for a minute. Port of hearth-core's friends.rs.
/// </summary>
public static class Friends
{
    private sealed record Status(long At, string State, string? Game, string? Avatar, string? Name);
    private static readonly ConcurrentDictionary<long, Status> Cache = new();
    private static int _fetching;
    /// <summary>States came in: the screen asks again.</summary>
    public static event Action? Changed;

    private static string? Tag(string xml, string t)
    {
        var m = Regex.Match(xml, $"<{t}>(.*?)</{t}>", RegexOptions.Singleline);
        if (!m.Success) return null;
        var v = m.Groups[1].Value.Trim();
        if (v.StartsWith("<![CDATA[")) v = v[9..^3].Trim();
        return v.Length == 0 ? null : v;
    }

    private static List<(long Id, string Name, string? Avatar)> Known()
    {
        var acct = SteamScanner.AccountDir();
        if (acct is null) return new();
        long.TryParse(Path.GetFileName(acct), out var mine);
        var friends = Vdf.Obj(Vdf.Load(Path.Combine(acct, "config", "localconfig.vdf")), "UserLocalConfigStore", "friends");
        var list = new List<(long, string, string?)>();
        if (friends is null) return list;
        foreach (var (k, v) in friends)
        {
            if (!long.TryParse(k, out var acc) || acc == mine || acc == 0 || v is not Dictionary<string, object> f) continue;
            if (Vdf.Str(f, "name") is not { } name) continue;
            var hash = Vdf.Str(f, "avatar");
            list.Add((SteamScanner.Id64Base + acc, name, hash is { Length: >= 20 } && hash.Any(c => c != '0') ? hash : null));
        }
        return list;
    }

    private static async Task Refresh(List<long> ids)
    {
        if (ids.Count == 0 || Interlocked.Exchange(ref _fetching, 1) == 1) return;
        try
        {
            using var gate = new SemaphoreSlim(4);
            await Task.WhenAll(ids.Select(async id =>
            {
                await gate.WaitAsync();
                try
                {
                    var x = await Stores.HelperTools.Http.GetStringAsync($"https://steamcommunity.com/profiles/{id}/?xml=1");
                    var game = Tag(Tag(x, "inGameInfo") ?? "", "gameName");
                    Cache[id] = new Status(Time.NowMs, game is not null ? "ingame" : Tag(x, "onlineState") ?? "unknown", game, Tag(x, "avatarMedium"), Tag(x, "steamID"));
                }
                catch { Cache[id] = new Status(Time.NowMs, "unknown", null, null, null); }
                finally { gate.Release(); }
            }));
        }
        finally { Interlocked.Exchange(ref _fetching, 0); }
        Changed?.Invoke();
    }

    public static object List()
    {
        var now = Time.NowMs;
        var stale = new List<long>();
        var outList = Known().Select(f =>
        {
            Cache.TryGetValue(f.Id, out var st);
            if (st is null || now - st.At > 60_000) stale.Add(f.Id);
            var avatar = st?.Avatar ?? (f.Avatar is { } h ? $"https://avatars.steamstatic.com/{h}_medium.jpg" : null);
            return new { id = f.Id.ToString(), name = st?.Name ?? f.Name, avatar, state = st?.State ?? "unknown", game = st?.Game };
        }).ToList();
        _ = Refresh(stale);
        int Rank(string s) => s switch { "ingame" => 0, "online" => 1, "unknown" => 2, _ => 3 };
        var sorted = outList.OrderBy(f => Rank(f.state)).ThenBy(f => f.name, StringComparer.CurrentCultureIgnoreCase).ToList();
        return new { friends = sorted, online = sorted.Count(f => Rank(f.state) < 2), signedIn = SteamScanner.AccountDir() is not null };
    }
}
