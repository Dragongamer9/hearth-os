using Nimbus.Host.Library;

namespace Nimbus.Host.Stores;

/// <summary>
/// The store accounts you sign into from Library › Stores (Epic, GOG, Xbox; Steam signs in through Steam itself),
/// their game lists, and the Epic/GOG installs that Nimbus runs.
/// </summary>
public sealed class StoreAccounts
{
    public EpicStore Epic { get; } = new();
    public GogStore Gog { get; } = new();
    public XboxStore Xbox { get; } = new();
    public InstallQueue Installs { get; } = new();

    private readonly LibraryStore _store;
    private readonly Dictionary<string, string?> _errors = new();
    private readonly HashSet<string> _busy = new();
    private readonly object _lock = new();

    /// <summary>An account or its game list changed: the UI reloads the library and the Stores page.</summary>
    public event Action? Changed;

    private static readonly TimeSpan Stale = TimeSpan.FromHours(12);

    public StoreAccounts(LibraryStore store) => _store = store;

    public static string DefaultRoot => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), "Games");
    public string InstallRoot
    {
        get => _store.Read(d => d.InstallRoot) is { Length: > 0 } r ? r : DefaultRoot;
        set => _store.Update(d => d.InstallRoot = value);
    }

    private OwnedCache? OwnedOf(string store) => store switch
    {
        "epic" => Epic.Owned, "gog" => Gog.Owned, "xbox" => Xbox.Owned, _ => null,
    };

    private bool SignedIn(string store) => store switch
    {
        "epic" => Epic.SignedIn, "gog" => Gog.SignedIn, "xbox" => Xbox.SignedIn, _ => false,
    };

    // ------------------------------------------------------------------ status

    public List<AccountDto> Status(IReadOnlyDictionary<string, int> installedCounts, int steamOwned)
    {
        int Inst(string s) => installedCounts.TryGetValue(s, out var n) ? n : 0;
        AccountDto Dto(string s, bool signedIn, string? name, string? avatar, int owned, long fetched, bool tool, string? note)
        {
            lock (_lock) return new AccountDto(s, signedIn, name, avatar, owned, Inst(s), fetched, _busy.Contains(s),
                                                _errors.TryGetValue(s, out var e) ? e : null, tool, note);
        }
        var steam = SteamScanner.Account();
        var gog = Gog.Profile();
        return new List<AccountDto>
        {
            Dto("steam", steam.Name is not null, steam.Name, steam.AvatarFile is not null ? $"https://{LibraryService.ArtHost}/profile/steam" : null,
                steamOwned, 0, true, SteamScanner.SteamPath() is null ? "Steam isn't installed on this PC." : null),
            Dto("epic", Epic.SignedIn, Epic.DisplayName(), null, Epic.Owned.Games.Count, Epic.Owned.Fetched,
                HelperTools.IsReady(HelperTools.Legendary), null),
            Dto("gog", Gog.SignedIn, gog.Name, gog.Avatar, Gog.Owned.Games.Count, Gog.Owned.Fetched,
                HelperTools.IsReady(HelperTools.Gogdl), null),
            Dto("xbox", Xbox.SignedIn, Xbox.Gamertag, Xbox.Gamerpic, Xbox.Owned.Games.Count, Xbox.Owned.Fetched, true, null),
        };
    }

    // ------------------------------------------------------------------ sign in / out / refresh

    public async Task<bool> SignInAsync(string store, IWin32Window? owner)
    {
        SetError(store, null);
        bool ok = store switch
        {
            "epic" => await Epic.SignInAsync(owner),
            "gog" => await Gog.SignInAsync(owner),
            "xbox" => await Xbox.SignInAsync(owner),
            _ => throw new ArgumentException("Unknown store"),
        };
        if (!ok) return false;
        SystemBridge.Log($"accounts: signed in to {store}");
        Changed?.Invoke();
        _ = RefreshAsync(store);     // fetch the game list in the background
        return true;
    }

    public async Task SignOutAsync(string store)
    {
        switch (store)
        {
            case "epic": await Epic.SignOutAsync(); break;
            case "gog": Gog.SignOut(); break;
            case "xbox": Xbox.SignOut(); break;
            default: throw new ArgumentException("Unknown store");
        }
        SetError(store, null);
        SystemBridge.Log($"accounts: signed out of {store}");
        Changed?.Invoke();
    }

    /// <summary>Downloads the store's game list again. Errors are kept and shown on the Stores page.</summary>
    public async Task RefreshAsync(string store)
    {
        if (!SignedIn(store)) return;
        lock (_lock) { if (!_busy.Add(store)) return; }
        Changed?.Invoke();
        try
        {
            await (store switch
            {
                "epic" => Epic.RefreshOwnedAsync(),
                "gog" => Gog.RefreshOwnedAsync(),
                "xbox" => Xbox.RefreshOwnedAsync(),
                _ => Task.CompletedTask,
            });
            SetError(store, null);
            SystemBridge.Log($"accounts: {store} list has {OwnedOf(store)?.Games.Count} games");
        }
        catch (Exception ex)
        {
            SetError(store, ex.Message);
            SystemBridge.Log($"accounts: {store} refresh FAILED {ex.Message}");
        }
        finally
        {
            lock (_lock) _busy.Remove(store);
            Changed?.Invoke();
        }
    }

    /// <summary>At startup and on rescans: refresh lists older than 12 hours, in the background.</summary>
    public void RefreshStale()
    {
        foreach (var s in new[] { "epic", "gog", "xbox" })
            if (SignedIn(s) && DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() - (OwnedOf(s)?.Fetched ?? 0) > Stale.TotalMilliseconds)
                _ = RefreshAsync(s);
    }

    private void SetError(string store, string? error) { lock (_lock) _errors[store] = error; }

    // ------------------------------------------------------------------ games for the library

    /// <summary>Epic games installed by legendary and GOG games installed by gogdl.</summary>
    public List<Game> Installed(string store) => store switch
    {
        "epic" => Epic.Installed(),
        "gog" => Gog.Installed(_store.Read(d => new Dictionary<string, string>(d.GogInstalls))),
        _ => new(),
    };

    /// <summary>Games on the account that aren't installed (the library shows them with an Install button).</summary>
    public List<Game> NotInstalled(string store, ISet<string> installedIds)
    {
        var owned = SignedIn(store) ? OwnedOf(store) : null;
        if (owned is null) return new();
        var list = new List<Game>();
        foreach (var o in owned.Games)
        {
            var id = store + ":" + o.Id;
            if (installedIds.Contains(id)) continue;
            var g = new Game
            {
                Id = id,
                Title = o.Title,
                Source = store,
                Added = o.Added,
                StoreLastPlayed = o.LastPlayed,
                Installed = false,
                State = "notInstalled",
                Note = o.Note,
                InstallUri = o.Note is not null ? null : store switch
                {
                    "epic" => "nimbus-epic:" + o.Id,
                    "gog" => "nimbus-gog:" + o.Id,
                    "xbox" => "ms-windows-store://pdp/?PFN=" + o.Id,
                    _ => null,
                },
            };
            foreach (var (k, v) in o.Art) g.Art[k] = v;
            list.Add(g);
        }
        return list;
    }

    /// <summary>Art from the account's list, for games found installed by the store's own launcher.</summary>
    public void AddArt(Game g)
    {
        var owned = OwnedOf(g.Source);
        if (owned is null || g.Art.Count > 0) return;
        var key = g.Id[(g.Source.Length + 1)..];
        if (owned.Games.FirstOrDefault(o => o.Id == key) is { } o)
            foreach (var (k, v) in o.Art) g.Art[k] = v;
    }

    // ------------------------------------------------------------------ installs

    public void StartInstall(Game g)
    {
        var uri = g.InstallUri ?? "";
        var root = InstallRoot;
        if (uri.StartsWith("nimbus-epic:"))
        {
            var app = uri["nimbus-epic:".Length..];
            if (!Epic.SignedIn) throw new InvalidOperationException("Sign in to Epic Games first (Library › Stores).");
            Installs.Enqueue(new InstallJob { GameId = g.Id, Title = g.Title, Work = (job, ct) => Epic.InstallAsync(job, app, root, ct) });
        }
        else if (uri.StartsWith("nimbus-gog:"))
        {
            var id = uri["nimbus-gog:".Length..];
            if (!Gog.SignedIn) throw new InvalidOperationException("Sign in to GOG first (Library › Stores).");
            Installs.Enqueue(new InstallJob
            {
                GameId = g.Id, Title = g.Title,
                Work = async (job, ct) =>
                {
                    var dir = await Gog.InstallAsync(job, id, root, ct);
                    _store.Update(d => d.GogInstalls[id] = dir);
                },
            });
        }
        else throw new InvalidOperationException("Ember can't install this game itself.");
    }

    public async Task UninstallAsync(Game g)
    {
        if (!g.CanUninstall) throw new InvalidOperationException($"Uninstall {g.Title} from its store app.");
        if (g.Source == "epic") { await Epic.UninstallAsync(g.Id["epic:".Length..]); return; }
        if (g.Source == "gog")
        {
            var id = g.Id["gog:".Length..];
            var dir = _store.Read(d => d.GogInstalls.TryGetValue(id, out var p) ? p : null);
            // Only ever delete a folder that is clearly this game's (it holds GOG's goggame-<id>.info).
            if (dir is not null && Directory.Exists(dir) && File.Exists(Path.Combine(dir, $"goggame-{id}.info")))
                await Task.Run(() => Directory.Delete(dir, recursive: true));
            _store.Update(d => d.GogInstalls.Remove(id));
            return;
        }
        throw new InvalidOperationException($"Uninstall {g.Title} from its store app.");
    }
}
