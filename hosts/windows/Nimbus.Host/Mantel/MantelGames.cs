using System.Text.Json;

namespace Nimbus.Host.Mantel;

/// <summary>
/// The launcher's Games section (your installed games from Ember's library, started the way Hearth's Play button
/// starts them: through the bridge's game.launch, so parental controls, play time and Quick Resume all apply) and its
/// Windows section (the programs that come with Windows, WindowsApps.cs).
/// </summary>
public sealed partial class MantelHost
{
    private async Task PushGamesAsync(MantelWindow to)
    {
        try
        {
            var lib = await _bridge.Games.GetAsync(false);
            // Pictures: https://nimbus-art.local/… (served to Mantel's windows like to Hearth's) or the store's web address.
            var games = lib.Games.Where(g => g.Installed && !g.Hidden && g.State == "installed")
                .Select(g => new { id = g.Id, name = g.Title, source = g.Source, cover = FirstArt(g.Art, "cover", "hero", "wide", "logo") })
                .ToList();
            Send(to, "mantel.games", games);
        }
        catch (Exception ex)
        {
            SystemBridge.Log("mantel: games: " + ex.Message);
            Send(to, "mantel.games", Array.Empty<object>());
        }
    }

    private static string? FirstArt(Dictionary<string, string>? art, params string[] kinds)
    {
        if (art is null) return null;
        foreach (var k in kinds) if (art.TryGetValue(k, out var v) && !string.IsNullOrEmpty(v)) return v;
        return null;
    }

    private async Task PushWindowsAppsAsync(MantelWindow to)
    {
        try { Send(to, "mantel.windowsApps", await WindowsApps.ListAsync()); }
        catch (Exception ex)
        {
            SystemBridge.Log("mantel: windows apps: " + ex.Message);
            Send(to, "mantel.windowsApps", Array.Empty<object>());
        }
    }

    /// <summary>Plays a game from the launcher; a problem (another game running, play time used up) shows as a notification.</summary>
    private void PlayGame(string id)
    {
        if (string.IsNullOrWhiteSpace(id)) return;
        var call = JsonSerializer.Serialize(new { id = 0, method = "game.launch", args = new { id } });
        var msg = JsonDocument.Parse(call).RootElement.Clone();
        _bridge.Handle(msg, reply: json =>
        {
            try
            {
                using var r = JsonDocument.Parse(json);
                if (r.RootElement.TryGetProperty("ok", out var ok) && ok.ValueKind == JsonValueKind.False)
                    Notifications.Add("Hearth", "Couldn't start the game", r.RootElement.TryGetProperty("error", out var e) ? e.GetString() ?? "" : "");
            }
            catch (Exception ex) { SystemBridge.Log("mantel: play: " + ex.Message); }
        });
    }
}
