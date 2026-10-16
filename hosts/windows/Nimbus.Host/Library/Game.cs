namespace Nimbus.Host.Library;

/// <summary>A game found by one of the scanners, with everything needed to show and launch it.</summary>
public sealed class Game
{
    public required string Id { get; init; }          // "steam:570", "epic:Fortnite", "gog:1207658924", "xbox:<family>", "manual:<guid>"
    public required string Title { get; init; }
    public required string Source { get; init; }      // steam | epic | gog | xbox | manual
    public long? SizeBytes { get; init; }
    public long Added { get; init; }                  // unix ms
    public string? InstallPath { get; init; }

    // Store-provided stats (Steam only for now). Nimbus merges these with its own.
    public int StoreMinutes { get; init; }
    public long StoreLastPlayed { get; init; }        // unix ms

    /// <summary>kind (cover / hero / logo / wide) → local image file.</summary>
    public Dictionary<string, string> Art { get; } = new();

    /// <summary>Folders whose processes count as "this game is running".</summary>
    public List<string> ProcessDirs { get; } = new();

    // How to launch: exactly one of these is set.
    public string? LaunchUri { get; init; }
    public string? Exe { get; init; }
    public string? Args { get; init; }
    public string? WorkDir { get; init; }
    public string? XboxFamily { get; init; }
    /// <summary>"legendary": started through legendary (Epic games Nimbus installed).</summary>
    public string? LaunchVia { get; init; }
    /// <summary>Nimbus installed it (Epic via legendary, GOG via gogdl), so Nimbus can uninstall it.</summary>
    public bool CanUninstall { get; init; }
    /// <summary>Shown on the game page, e.g. why it can't be installed from Nimbus.</summary>
    public string? Note { get; init; }

    // Install state. Installed games have State "installed"; Steam can also report
    // "downloading", "queued", "paused", "updating"; owned-but-not-installed games are "notInstalled".
    public bool Installed { get; init; } = true;
    public string State { get; init; } = "installed";
    public double? Progress { get; init; }          // 0..1 while downloading
    public long? BytesDone { get; init; }
    public long? BytesTotal { get; init; }
    /// <summary>steam://install/…, a Store link, or nimbus-epic:/nimbus-gog: for installs Nimbus runs itself.</summary>
    public string? InstallUri { get; init; }
}

public static class Time
{
    public static long NowMs => DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
    public static long Ms(DateTime utc) => new DateTimeOffset(DateTime.SpecifyKind(utc, DateTimeKind.Utc)).ToUnixTimeMilliseconds();
}
