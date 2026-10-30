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
