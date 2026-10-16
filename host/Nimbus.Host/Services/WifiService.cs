using System.Diagnostics;
using Windows.Devices.WiFi;
using Windows.Networking.Connectivity;
using Windows.Security.Credentials;

namespace Nimbus.Host.Services;

public sealed record WifiNetwork(string Ssid, int Bars, bool Secure, bool Connected, bool Saved);

/// <summary>Scan, connect, disconnect and forget Wi-Fi networks via Windows.Devices.WiFi.</summary>
public sealed class WifiService
{
    private WiFiAdapter? _adapter;
    private readonly Dictionary<string, WiFiAvailableNetwork> _last = new();

    /// <summary>Raised (on a background thread) when connectivity changes.</summary>
    public event Action? Changed;

    public WifiService()
    {
        NetworkInformation.NetworkStatusChanged += _ => Changed?.Invoke();
    }

    private async Task<WiFiAdapter> AdapterAsync()
    {
        if (_adapter is not null) return _adapter;

        var access = await WiFiAdapter.RequestAccessAsync();
        if (access != WiFiAccessStatus.Allowed)
            throw new InvalidOperationException(
                "Windows denied Wi-Fi access. Turn on Windows Settings › Privacy & security › Location › \"Let desktop apps access your location\" (Windows needs it for Wi-Fi scanning).");

        var adapters = await WiFiAdapter.FindAllAdaptersAsync();
        _adapter = adapters.FirstOrDefault() ?? throw new InvalidOperationException("No Wi-Fi adapter found.");
        return _adapter;
    }

    public async Task<bool> HasAdapterAsync()
    {
        try { await AdapterAsync(); return true; } catch { return false; }
    }

    /// <summary>The connected SSID (or null) and its signal bars.</summary>
    public async Task<(string? Ssid, int Bars)> CurrentAsync()
    {
        try
        {
            var a = await AdapterAsync();
            var p = await a.NetworkAdapter.GetConnectedProfileAsync();
            if (p is { IsWlanConnectionProfile: true })
                return (p.WlanConnectionProfileDetails.GetConnectedSsid(), p.GetSignalBars() ?? 0);
        }
        catch { /* no adapter / no access: report not connected */ }
        return (null, 0);
    }

    public async Task<List<WifiNetwork>> ScanAsync()
    {
        var a = await AdapterAsync();
        await a.ScanAsync();
        var (current, _) = await CurrentAsync();
        var saved = SavedProfiles();

        // One entry per SSID (an access point can broadcast on several bands), keep the strongest.
        _last.Clear();
        foreach (var n in a.NetworkReport.AvailableNetworks)
        {
            if (string.IsNullOrEmpty(n.Ssid)) continue; // hidden networks
            if (!_last.TryGetValue(n.Ssid, out var existing) || n.SignalBars > existing.SignalBars)
                _last[n.Ssid] = n;
        }

        return _last.Values
            .Select(n => new WifiNetwork(n.Ssid, n.SignalBars, IsSecure(n), n.Ssid == current, saved.Contains(n.Ssid)))
            .OrderByDescending(n => n.Connected).ThenByDescending(n => n.Saved).ThenByDescending(n => n.Bars)
            .ToList();
    }

    public async Task<(bool Ok, string? Error)> ConnectAsync(string ssid, string? password)
    {
        var a = await AdapterAsync();
        if (!_last.TryGetValue(ssid, out var network))
        {
            await ScanAsync();
            if (!_last.TryGetValue(ssid, out network)) return (false, "That network is out of range");
        }

        WiFiConnectionResult result = string.IsNullOrEmpty(password)
            ? await a.ConnectAsync(network, WiFiReconnectionKind.Automatic)
            : await a.ConnectAsync(network, WiFiReconnectionKind.Automatic, new PasswordCredential { Password = password });

        return result.ConnectionStatus switch
        {
            WiFiConnectionStatus.Success => (true, null),
            WiFiConnectionStatus.InvalidCredential => (false, "The password is incorrect"),
            WiFiConnectionStatus.Timeout => (false, "The connection timed out"),
            WiFiConnectionStatus.NetworkNotAvailable => (false, "That network is out of range"),
            WiFiConnectionStatus.AccessRevoked => (false, "Wi-Fi access was revoked"),
            WiFiConnectionStatus.UnsupportedAuthenticationProtocol => (false, "This network's security type isn't supported"),
            _ => (false, "Couldn't connect"),
        };
    }

    public async Task DisconnectAsync() => (await AdapterAsync()).Disconnect();

    /// <summary>Deletes the saved profile. There's no WinRT API for this, so it uses netsh.</summary>
    public async Task ForgetAsync(string ssid)
    {
        var psi = new ProcessStartInfo("netsh") { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true };
        foreach (var arg in new[] { "wlan", "delete", "profile", $"name={ssid}" }) psi.ArgumentList.Add(arg);
        using var p = Process.Start(psi) ?? throw new InvalidOperationException("Couldn't start netsh");
        await p.WaitForExitAsync();
        if (p.ExitCode != 0) throw new InvalidOperationException($"Couldn't forget {ssid}");
    }

    private static HashSet<string> SavedProfiles() =>
        NetworkInformation.GetConnectionProfiles()
            .Where(p => p.IsWlanConnectionProfile)
            .Select(p => p.ProfileName)
            .ToHashSet();

    private static bool IsSecure(WiFiAvailableNetwork n) =>
        n.SecuritySettings.NetworkAuthenticationType is not (NetworkAuthenticationType.Open80211 or NetworkAuthenticationType.None);
}
