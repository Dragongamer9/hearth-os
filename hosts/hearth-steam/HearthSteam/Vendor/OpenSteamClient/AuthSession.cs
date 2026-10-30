// Adapted from OpenSteamClient (c#-remake), OpenSteamworks.Client/Login/LoginManager.cs and LoginPoll.cs.
// Copyright (c) 2023 Onni Kukkonen. MIT licensed: see hosts/hearth-steam/LICENSE.OpenSteamClient.
// Hearth's changes: only the credentials, Steam Guard and QR flows; no account list, no config, no logging of
// anything typed. The refresh token Steam answers with goes straight to steamclient (LogOnWithToken) and is dropped.

using System.Net;
using System.Security.Cryptography;
using System.Text;
using OpenSteamClient.Logging;
using OpenSteamworks;
using OpenSteamworks.Data.Enums;
using OpenSteamworks.Messaging;
using OpenSteamworks.Messaging.SharedConnection;
using OpenSteamworks.Protobuf;

namespace HearthSteam.Vendor;

internal sealed class AuthSession : IDisposable
{
    readonly ISteamClient client;
    readonly ILoggerFactory loggerFactory;
    Connection? conn;
    ulong clientId;
    Google.Protobuf.ByteString? requestId;
    float interval = 5;
    ulong steamId;
    volatile bool polling;

    /// <summary>A new QR challenge URL (the first, and each time Steam replaces it).</summary>
    public event Action<string>? ChallengeUrl;
    /// <summary>Steam accepted the sign-in: (account name, refresh token). The token is for steamclient only.</summary>
    public event Action<string, string>? Token;
    public event Action<EResult>? Failed;

    public AuthSession(ISteamClient client, ILoggerFactory loggerFactory) { this.client = client; this.loggerFactory = loggerFactory; }

    static CAuthentication_DeviceDetails Device()
    {
        var d = new CAuthentication_DeviceDetails { GamingDeviceType = 0, OsType = (int)EOSType.Linux, PlatformType = EAuthTokenPlatformType.SteamClient };
        try { d.DeviceFriendlyName = Dns.GetHostName(); } catch (Exception) { d.DeviceFriendlyName = "Hearth"; }
        return d;
    }

    Connection Conn() => conn ??= new Connection(new SharedConnectionTransport(client, loggerFactory));

    /// <summary>Returns what Steam needs next: null (nothing), "device", "email" or "confirm" (approve in the
    /// Steam app).</summary>
    public async Task<(EResult result, string? needs)> StartCredentials(string username, string password, bool remember)
    {
        var c = Conn();
        ProtoMsg<CAuthentication_GetPasswordRSAPublicKey_Request> rsaRequest = new("Authentication.GetPasswordRSAPublicKey#1", true);
        rsaRequest.Body.AccountName = username;
        var rsaResp = await c.SendServiceMethod<CAuthentication_GetPasswordRSAPublicKey_Response>(rsaRequest);
        if (rsaResp.Header.Eresult != (int)EResult.OK) return ((EResult)rsaResp.Header.Eresult, null);

        string encrypted;
        using (var rsa = RSA.Create())
        {
            rsa.ImportParameters(new RSAParameters { Modulus = Convert.FromHexString(rsaResp.Body.PublickeyMod), Exponent = Convert.FromHexString(rsaResp.Body.PublickeyExp) });
            encrypted = Convert.ToBase64String(rsa.Encrypt(Encoding.UTF8.GetBytes(password), RSAEncryptionPadding.Pkcs1));
        }

        ProtoMsg<CAuthentication_BeginAuthSessionViaCredentials_Request> begin = new("Authentication.BeginAuthSessionViaCredentials#1", true);
        var device = Device();
        begin.Body.DeviceDetails = device;
        begin.Body.AccountName = username;
        begin.Body.EncryptedPassword = encrypted;
        begin.Body.EncryptionTimestamp = rsaResp.Body.Timestamp;
        begin.Body.RememberLogin = remember;
        begin.Body.Persistence = remember ? ESessionPersistence.Persistent : ESessionPersistence.Ephemeral;
        begin.Body.PlatformType = EAuthTokenPlatformType.SteamClient;
        begin.Body.WebsiteId = "Client";
        begin.Body.DeviceFriendlyName = device.DeviceFriendlyName;

        var resp = await c.SendServiceMethod<CAuthentication_BeginAuthSessionViaCredentials_Response>(begin);
        if (resp.Header.Eresult != (int)EResult.OK) return ((EResult)resp.Header.Eresult, null);
        steamId = resp.Body.Steamid;
        string? needs = null;
        var kinds = resp.Body.AllowedConfirmations.Select(a => a.ConfirmationType).ToList();
        if (kinds.Contains(EAuthSessionGuardType.DeviceCode)) needs = "device";
        else if (kinds.Contains(EAuthSessionGuardType.EmailCode)) needs = "email";
        else if (kinds.Contains(EAuthSessionGuardType.DeviceConfirmation) || kinds.Contains(EAuthSessionGuardType.EmailConfirmation)) needs = "confirm";
        StartPolling(resp.Body.ClientId, resp.Body.RequestId, resp.Body.Interval);
        return (EResult.OK, needs);
    }

    public async Task<EResult> SubmitCode(string code, string kind)
    {
        if (!polling || conn == null) throw new InvalidOperationException("No sign-in is waiting for a code.");
        ProtoMsg<CAuthentication_UpdateAuthSessionWithSteamGuardCode_Request> m = new("Authentication.UpdateAuthSessionWithSteamGuardCode#1", true);
        m.Body.ClientId = clientId;
        m.Body.CodeType = kind == "email" ? EAuthSessionGuardType.EmailCode : EAuthSessionGuardType.DeviceCode;
        m.Body.Code = code;
        m.Body.Steamid = steamId;
        var resp = await conn.SendServiceMethod<CAuthentication_UpdateAuthSessionWithSteamGuardCode_Response>(m);
        return (EResult)resp.Header.Eresult;
    }

    public async Task<string> StartQr()
    {
        var c = Conn();
        ProtoMsg<CAuthentication_BeginAuthSessionViaQR_Request> begin = new("Authentication.BeginAuthSessionViaQR#1", true);
        begin.Body.DeviceDetails = Device();
        var resp = await c.SendServiceMethod<CAuthentication_BeginAuthSessionViaQR_Response>(begin);
        if (resp.Header.Eresult != (int)EResult.OK) throw new EngineException(Codes.SteamError, $"Steam answered {(EResult)resp.Header.Eresult}.");
        StartPolling(resp.Body.ClientId, resp.Body.RequestId, resp.Body.Interval);
        return resp.Body.ChallengeUrl;
    }

    void StartPolling(ulong id, Google.Protobuf.ByteString request, float every)
    {
        clientId = id; requestId = request; interval = every <= 0 ? 5 : every;
        polling = true;
        new Thread(Poll) { IsBackground = true, Name = "steam-auth-poll" }.Start();
    }

    async void Poll()
    {
        try
        {
            while (polling && conn != null)
            {
                ProtoMsg<CAuthentication_PollAuthSessionStatus_Request> m = new("Authentication.PollAuthSessionStatus#1", true);
                m.Body.ClientId = clientId;
                m.Body.RequestId = requestId;
                var r = await conn.SendServiceMethod<CAuthentication_PollAuthSessionStatus_Response>(m);
                if (!polling) break;
                if (r.Header.Eresult != (int)EResult.OK) { polling = false; Failed?.Invoke((EResult)r.Header.Eresult); break; }
                if (r.Body.HasNewClientId) clientId = r.Body.NewClientId;
                if (r.Body.HasNewChallengeUrl) ChallengeUrl?.Invoke(r.Body.NewChallengeUrl);
                if (r.Body.HasRefreshToken) { polling = false; Token?.Invoke(r.Body.AccountName, r.Body.RefreshToken); break; }
                Thread.Sleep((int)(interval * 1000));
            }
        }
        catch (Exception) { polling = false; Failed?.Invoke(EResult.Failure); }
    }

    public void Dispose()
    {
        polling = false;
        try { conn?.Dispose(); } catch (Exception) { }
        conn = null;
    }
}
