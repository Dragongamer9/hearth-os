using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;
using Nimbus.Host.Library;

namespace Nimbus.Host.Stores;

/// <summary>
/// Xbox / Microsoft account sign-in (Microsoft account → Xbox Live), used to list the PC games you've played
/// on Xbox and your gamertag and gamerpic. Installing Xbox games still goes through the Xbox app / Microsoft
/// Store, because Microsoft only allows those apps to install them.
/// </summary>
public sealed class XboxStore
{
    // The public client id that Xbox apps and many open-source Xbox tools use, with Microsoft's desktop redirect page.
    private const string ClientId = "000000004C12AE6F";
    private const string Scope = "service::user.auth.xboxlive.com::MBI_SSL";
    private const string Redirect = "https://login.live.com/oauth20_desktop.srf";
    public static readonly string LoginUrl =
        $"https://login.live.com/oauth20_authorize.srf?client_id={ClientId}&response_type=code&display=touch" +
        $"&scope={Uri.EscapeDataString(Scope)}&redirect_uri={Uri.EscapeDataString(Redirect)}";

    private static string CachePath => Path.Combine(HelperTools.DataDir, "cache", "xbox-titles.json");

    private sealed class Saved
    {
        public string RefreshToken { get; set; } = "";
        public string? Gamertag { get; set; }
        public string? Xuid { get; set; }
        public string? Gamerpic { get; set; }
    }

    // Xbox Live wants the property names exactly as written (the web default would camelCase them).
    private static readonly JsonSerializerOptions Plain = new();

    private Saved? _saved;
    private (string Uhs, string Token, string Xuid, DateTimeOffset NotAfter)? _xsts;
    private readonly SemaphoreSlim _lock = new(1, 1);

    public OwnedCache Owned { get; private set; } = OwnedCache.Load(CachePath);

    public XboxStore()
    {
        var raw = SecureStore.Load("xbox");
        if (raw is not null) try { _saved = JsonSerializer.Deserialize<Saved>(raw); } catch { }
    }

    public bool SignedIn => _saved is not null;
    public string? Gamertag => _saved?.Gamertag;
    public string? Gamerpic => _saved?.Gamerpic;

    public async Task<bool> SignInAsync(IWin32Window? owner)
    {
        string? error = null;
        var code = await SignInWindow.RunAsync(owner, "Sign in with your Microsoft account (Xbox)", LoginUrl, onStarting: url =>
        {
            if (!url.StartsWith(Redirect, StringComparison.OrdinalIgnoreCase)) return null;
            error = SignInWindow.Query(url, "error_description") ?? SignInWindow.Query(url, "error");
            return SignInWindow.Query(url, "code") ?? "";
        });
        if (error is not null) throw new InvalidOperationException("Microsoft sign-in failed: " + error);
        if (string.IsNullOrEmpty(code)) return false;

        var tokens = await MsaTokenAsync(new() { ["grant_type"] = "authorization_code", ["code"] = code });
        _saved = new Saved { RefreshToken = tokens.Refresh };
        _xsts = null;
        await XstsAsync(tokens.Access);
        await LoadProfileAsync();
        Persist();
        return true;
    }

    public void SignOut()
    {
        _saved = null; _xsts = null;
        SecureStore.Delete("xbox");
        Owned = new OwnedCache();
        try { File.Delete(CachePath); } catch { }
    }

    private void Persist() { if (_saved is not null) SecureStore.Save("xbox", JsonSerializer.Serialize(_saved)); }

    /// <summary>renewing: from the saved refresh token, where a refusal means the sign-in expired (SignInExpiry);
    /// otherwise it's a new sign-in's code. No connection throws HttpRequestException: never an expiry.</summary>
    private async Task<(string Access, string Refresh)> MsaTokenAsync(Dictionary<string, string> form, bool renewing = false)
    {
        form["client_id"] = ClientId;
        form["redirect_uri"] = Redirect;
        form["scope"] = Scope;
        using var resp = await HelperTools.Http.PostAsync("https://login.live.com/oauth20_token.srf", new FormUrlEncodedContent(form));
        var body = await resp.Content.ReadAsStringAsync();
        JsonDocument? doc = null;
        try { doc = JsonDocument.Parse(body); } catch (JsonException) { }   // a captive portal's page, say
        using var parsed = doc;
        var access = doc?.RootElement.Str("access_token"); var refresh = doc?.RootElement.Str("refresh_token");
        if (!resp.IsSuccessStatusCode || access is null || refresh is null)
        {
            var status = (int)resp.StatusCode;
            if (renewing && SignInExpiry.TokenRefused(status, body)) { SignInExpiry.Mark("xbox"); throw new StoreExpiredException("xbox"); }
            throw new InvalidOperationException(renewing ? $"Microsoft didn't renew your sign-in just now (HTTP {status}). Try again in a little while."
                : "Microsoft didn't accept the sign-in. Try again. " + (doc?.RootElement.Str("error_description") ?? ""));
        }
        return (access, refresh);
    }

    /// <summary>Microsoft account token → Xbox user token → Xbox Live (XSTS) token.</summary>
    private async Task XstsAsync(string msaAccess)
    {
        string? userToken = null;
        // The ticket format depends on how the token was issued; try the known forms.
        foreach (var ticket in new[] { msaAccess, "t=" + msaAccess, "d=" + msaAccess })
        {
            using var req = new HttpRequestMessage(HttpMethod.Post, "https://user.auth.xboxlive.com/user/authenticate")
            {
                Content = JsonContent.Create(new
                {
                    RelyingParty = "http://auth.xboxlive.com",
                    TokenType = "JWT",
                    Properties = new { AuthMethod = "RPS", SiteName = "user.auth.xboxlive.com", RpsTicket = ticket },
                }, options: Plain),
            };
            req.Headers.Add("x-xbl-contract-version", "1");
            using var resp = await HelperTools.Http.SendAsync(req);
            if (!resp.IsSuccessStatusCode) continue;
            using var doc = JsonDocument.Parse(await resp.Content.ReadAsStringAsync());
            userToken = doc.RootElement.Str("Token");
            if (userToken is not null) break;
        }
        if (userToken is null) throw new InvalidOperationException("Xbox Live didn't accept this Microsoft account.");

        using var xreq = new HttpRequestMessage(HttpMethod.Post, "https://xsts.auth.xboxlive.com/xsts/authorize")
        {
            Content = JsonContent.Create(new
            {
                RelyingParty = "http://xboxlive.com",
                TokenType = "JWT",
                Properties = new { UserTokens = new[] { userToken }, SandboxId = "RETAIL" },
            }, options: Plain),
        };
        xreq.Headers.Add("x-xbl-contract-version", "1");
        using var xresp = await HelperTools.Http.SendAsync(xreq);
        var xbody = await xresp.Content.ReadAsStringAsync();
        if (!xresp.IsSuccessStatusCode)
        {
            var err = xbody.Contains("2148916233") ? "This Microsoft account doesn't have an Xbox profile yet. Create one at xbox.com, then try again."
                    : xbody.Contains("2148916238") ? "This is a child account. An adult needs to add it to a Microsoft family first."
                    : $"Xbox Live sign-in failed ({(int)xresp.StatusCode}).";
            throw new InvalidOperationException(err);
        }
        using var xdoc = JsonDocument.Parse(xbody);
        var x = xdoc.RootElement;
        var xui = x.Obj("DisplayClaims").Arr("xui").FirstOrDefault();
        var token = x.Str("Token"); var uhs = xui.Str("uhs"); var xuid = xui.Str("xid");
        if (token is null || uhs is null || xuid is null) throw new InvalidOperationException("Xbox Live sign-in failed (no profile in the response).");
        var notAfter = DateTimeOffset.TryParse(x.Str("NotAfter"), System.Globalization.CultureInfo.InvariantCulture, System.Globalization.DateTimeStyles.AssumeUniversal, out var na) ? na : DateTimeOffset.UtcNow.AddHours(4);
        _xsts = (uhs, token, xuid, notAfter);
        if (_saved is not null) { _saved.Xuid = xuid; _saved.Gamertag ??= xui.Str("gtg"); }
    }

    private async Task EnsureXstsAsync()
    {
        if (_saved is null) throw new InvalidOperationException("Sign in to Xbox first.");
        if (_xsts is { } x && x.NotAfter > DateTimeOffset.UtcNow.AddMinutes(5)) return;
        await _lock.WaitAsync();
        try
        {
            if (_xsts is { } y && y.NotAfter > DateTimeOffset.UtcNow.AddMinutes(5)) return;
            var tokens = await MsaTokenAsync(new() { ["grant_type"] = "refresh_token", ["refresh_token"] = _saved.RefreshToken }, renewing: true);
            _saved.RefreshToken = tokens.Refresh;   // Microsoft rotates refresh tokens
            Persist();
            SignInExpiry.Clear("xbox");
            await XstsAsync(tokens.Access);
        }
        finally { _lock.Release(); }
    }

    private async Task<JsonDocument> GetAsync(string url, string contract)
    {
        await EnsureXstsAsync();
        var x = _xsts!.Value;
        using var req = new HttpRequestMessage(HttpMethod.Get, url);
        req.Headers.Authorization = new AuthenticationHeaderValue("XBL3.0", $"x={x.Uhs};{x.Token}");
        req.Headers.Add("x-xbl-contract-version", contract);
        req.Headers.AcceptLanguage.ParseAdd("en-US");
        using var resp = await HelperTools.Http.SendAsync(req);
        resp.EnsureSuccessStatusCode();
        return JsonDocument.Parse(await resp.Content.ReadAsStreamAsync());
    }

    private async Task LoadProfileAsync()
    {
        try
        {
            using var doc = await GetAsync($"https://profile.xboxlive.com/users/xuid({_xsts!.Value.Xuid})/profile/settings?settings=GameDisplayPicRaw,Gamertag,ModernGamertag", "2");
            foreach (var s in doc.RootElement.Arr("profileUsers").FirstOrDefault().Arr("settings"))
            {
                var v = s.Str("value");
                if (string.IsNullOrEmpty(v)) continue;
                switch (s.Str("id"))
                {
                    case "GameDisplayPicRaw": _saved!.Gamerpic = J.Https(v); break;
                    case "ModernGamertag": _saved!.Gamertag = v; break;
                    case "Gamertag": _saved!.Gamertag ??= v; break;
                }
            }
        }
        catch (Exception ex) { SystemBridge.Log("xbox profile: " + ex.Message); }
    }

    /// <summary>PC games you've played with this Xbox profile (title history), with Xbox art.</summary>
    public async Task RefreshOwnedAsync()
    {
        await EnsureXstsAsync();
        if (_saved!.Gamerpic is null) { await LoadProfileAsync(); Persist(); }
        using var doc = await GetAsync($"https://titlehub.xboxlive.com/users/xuid({_xsts!.Value.Xuid})/titles/titlehistory/decoration/image,detail?maxItems=1000", "2");
        var cache = new OwnedCache { Fetched = Time.NowMs };
        foreach (var t in doc.RootElement.Arr("titles"))
        {
            var pfn = t.Str("pfn"); var name = t.Str("name");
            if (pfn is null || name is null || t.Str("type") != "Game") continue;
            if (!t.Arr("devices").Any(d => d.GetString() == "PC")) continue;
            var o = new OwnedGame { Id = pfn, Title = name, LastPlayed = J.Ms(t.Obj("titleHistory").Str("lastTimePlayed")) };
            var imgs = t.Arr("images").Select(i => (Type: i.Str("type") ?? "", Url: i.Str("url"))).Where(i => i.Url is not null).ToList();
            string? Img(params string[] types) => types.Select(ty => imgs.FirstOrDefault(i => i.Type == ty).Url).FirstOrDefault(u => u is not null);
            if ((Img("Poster", "BoxArt") ?? t.Str("displayImage")) is { } c) o.Art["cover"] = J.Https(c);
            if (Img("SuperHeroArt", "TitledHeroArt", "BrandedKeyArt") is { } h) o.Art["hero"] = J.Https(h);
            if (Img("BrandedKeyArt", "TitledHeroArt", "SuperHeroArt") is { } w) o.Art["wide"] = J.Https(w);
            if (Img("Logo") is { } l) o.Art["logo"] = J.Https(l);
            cache.Games.Add(o);
        }
        cache.Save(CachePath);
        Owned = cache;
    }
}
