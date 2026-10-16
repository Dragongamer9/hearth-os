using Windows.Devices.Radios;

namespace Nimbus.Host.Services;

/// <summary>Wi-Fi and Bluetooth radio on/off via Windows.Devices.Radios.</summary>
public sealed class RadioService
{
    private bool _accessChecked;
    private bool _allowed;
    private readonly HashSet<string> _hooked = new();

    /// <summary>Raised (on a background thread) when any radio changes state.</summary>
    public event Action? Changed;

    private async Task<IReadOnlyList<Radio>> RadiosAsync()
    {
        if (!_accessChecked)
        {
            _allowed = await Radio.RequestAccessAsync() == RadioAccessStatus.Allowed;
            _accessChecked = true;
        }
        if (!_allowed)
            throw new InvalidOperationException("Windows denied access to the radios. Check Windows Settings › Privacy & security › Radios.");

        var radios = await Radio.GetRadiosAsync();
        foreach (var r in radios)
        {
            if (_hooked.Add($"{r.Kind}:{r.Name}"))
                r.StateChanged += (_, _) => Changed?.Invoke();
        }
        return radios;
    }

    /// <summary>null when the PC has no radio of that kind.</summary>
    public async Task<bool?> GetAsync(RadioKind kind)
    {
        var r = (await RadiosAsync()).FirstOrDefault(x => x.Kind == kind);
        return r is null ? null : r.State == RadioState.On;
    }

    public async Task SetAsync(RadioKind kind, bool on)
    {
        var r = (await RadiosAsync()).FirstOrDefault(x => x.Kind == kind)
                ?? throw new InvalidOperationException(kind == RadioKind.WiFi ? "No Wi-Fi adapter found." : "No Bluetooth adapter found.");
        var result = await r.SetStateAsync(on ? RadioState.On : RadioState.Off);
        if (result != RadioAccessStatus.Allowed)
            throw new InvalidOperationException($"Windows refused to change the {(kind == RadioKind.WiFi ? "Wi-Fi" : "Bluetooth")} radio ({result}).");
    }
}
