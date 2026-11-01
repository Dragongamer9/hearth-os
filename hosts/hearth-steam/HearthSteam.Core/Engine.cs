using System.Text.Json.Nodes;

namespace HearthSteam;

/// <summary>Why the engine said no. Hearth falls back to the normal Steam client for every code.</summary>
public static class Codes
{
    public const string NotReady = "not_ready";           // still starting, bootstrapping, or failed to start
    public const string ClassicRunning = "classic_running"; // the normal Steam client is running: it does the work
    public const string SignedOut = "signed_out";
    public const string Vac = "vac";                       // VAC games always go through the normal client
    public const string SteamError = "steam_error";        // steamclient answered with an error
    public const string Unsupported = "unsupported";
    public const string BadRequest = "bad_request";
    public const string Busy = "busy";
}

public sealed class EngineException : Exception
{
    public string Code { get; }
    public EngineException(string code, string message) : base(message) { Code = code; }
}

/// <summary>What the socket API asks of Steam's engine. SteamEngine is the real one (steamclient.so through
/// OpenSteamworks); tests use a fake. Methods throw EngineException to answer no.</summary>
public interface IEngine
{
    /// <summary>{engine: starting|bootstrapping|ready|unavailable, reason, signedIn, account, steamId, signIn:{...}}</summary>
    JsonObject Status();
    /// <summary>Starts a sign-in with name and password. The values go to Steam's servers through steamclient and
    /// are dropped right after: never stored, never logged. Answers {needs: null|"device"|"email"|"confirm"}.</summary>
    Task<JsonObject> SignInPassword(string user, string password, bool remember);
    Task<JsonObject> SignInCode(string code, string kind);
    /// <summary>Starts a QR sign-in: {url, svg}. Status() carries the new code when Steam replaces it.</summary>
    Task<JsonObject> SignInQr();
    void SignInCancel();
    Task<JsonObject> SignOut();
    JsonObject Library();
    JsonObject Install(uint appId, string? library);
    JsonObject Uninstall(uint appId);
    JsonObject Update(uint appId);
    JsonObject Downloads();
    Task<JsonObject> Launch(uint appId);
    JsonObject Running();
    /// <summary>Lets go of Steam (the normal client is about to start): the service exits after answering.</summary>
    void Release();
}

/// <summary>What only the Open engine (SteamKit2, no steamclient) answers on top of IEngine.</summary>
public interface IOpenEngine : IEngine
{
    /// <summary>How a first direct start went: remembered, so a game that failed always starts through Valve's client.</summary>
    JsonObject LaunchResult(uint appId, bool ok);
    /// <summary>Checks every file of an installed game against its manifest and fixes what's wrong.</summary>
    JsonObject Verify(uint appId);
    /// <summary>Stops a queued or running download (what's done is kept; it can be resumed).</summary>
    JsonObject Cancel(uint appId);
    /// <summary>Valve's client is about to run: stop downloads (kept, resumed when it has closed) and write nothing
    /// into the libraries meanwhile. The engine stays signed in, so sign-in and the library keep working.</summary>
    JsonObject Standby();
    /// <summary>Pauses one queued or running download (or all with null): what's done is kept.</summary>
    JsonObject Pause(uint? appId);
    JsonObject Resume(uint? appId);
    /// <summary>Moves a download "up", "down" or to the "top" of the queue.</summary>
    JsonObject Move(uint appId, string to);
    /// <summary>Hearth's settings (automatic updates off/idle/always, speed limit in KB/s, 0 none) and whether a game is
    /// running; null leaves one as it is.</summary>
    JsonObject Config(string? autoUpdate, long? limitKBps, bool? gameRunning);
}
