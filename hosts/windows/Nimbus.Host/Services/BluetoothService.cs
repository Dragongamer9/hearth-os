using Windows.Devices.Enumeration;

namespace Nimbus.Host.Services;

public sealed record BtDevice(string Id, string Name, string Kind, bool Connected, bool Paired);

/// <summary>
/// Lists, discovers, pairs and removes Bluetooth devices (Classic and LE) through
/// Windows.Devices.Enumeration association endpoints.
/// </summary>
public sealed class BluetoothService : IDisposable
{
    private const string ClassicProtocol = "{e0cbf06c-cd8b-4647-bb8a-263b43f0f974}";
    private const string LeProtocol = "{bb7bb05e-5972-42b5-94fc-76eaa7084d49}";
    private const string PropConnected = "System.Devices.Aep.IsConnected";
    private const string PropCategory = "System.Devices.Aep.Category";
    private static readonly string[] Props = { PropConnected, PropCategory, "System.Devices.Aep.IsPresent" };
    private static readonly string[] ListProps = { PropConnected, PropCategory };

    private DeviceWatcher? _watcher;
    private System.Threading.Timer? _stopTimer;
    private TaskCompletionSource<bool>? _pinConfirm;

    // Raised on background threads.
    public event Action<BtDevice>? Found;
    public event Action? ScanDone;
    /// <summary>(pin, needsConfirm). needsConfirm=false means "type this PIN on the device".</summary>
    public event Action<string, bool>? Pin;

    private static string Aqs(bool paired) =>
        $"(System.Devices.Aep.ProtocolId:=\"{ClassicProtocol}\" OR System.Devices.Aep.ProtocolId:=\"{LeProtocol}\") " +
        $"AND System.Devices.Aep.IsPaired:=System.StructuredQueryType.Boolean#{(paired ? "True" : "False")}";

    private static BtDevice ToDevice(DeviceInformation d, bool paired)
    {
        bool connected = d.Properties.TryGetValue(PropConnected, out var c) && c is true;
        string kind = d.Properties.TryGetValue(PropCategory, out var k) && k is string[] cats && cats.Length > 0 ? cats[0] : "";
        return new BtDevice(d.Id, d.Name, kind, connected, paired);
    }

    public async Task<List<BtDevice>> PairedAsync()
    {
        // Query Classic and LE separately with the documented selectors, each with its own timeout,
        // so one slow stack can't hold up the whole list.
        var selectors = new[]
        {
            Windows.Devices.Bluetooth.BluetoothDevice.GetDeviceSelectorFromPairingState(true),
            Windows.Devices.Bluetooth.BluetoothLEDevice.GetDeviceSelectorFromPairingState(true),
        };
        var results = await Task.WhenAll(selectors.Select(async sel =>
        {
            var op = DeviceInformation.FindAllAsync(sel, ListProps, DeviceInformationKind.AssociationEndpoint).AsTask();
            var done = await Task.WhenAny(op, Task.Delay(TimeSpan.FromSeconds(8)));
            return done == op ? op.Result.ToList() : new List<DeviceInformation>();
        }));
        var list = results.SelectMany(r => r);

        // Dual-mode devices show up once per protocol; keep one row per name, preferring the connected one.
        return list.Where(d => !string.IsNullOrWhiteSpace(d.Name))
                   .Select(d => ToDevice(d, true))
                   .GroupBy(d => d.Name)
                   .Select(g => g.OrderByDescending(d => d.Connected).First())
                   .OrderByDescending(d => d.Connected).ThenBy(d => d.Name)
                   .ToList();
    }

    public void StartScan(int seconds)
    {
        StopScan();
        var w = DeviceInformation.CreateWatcher(Aqs(false), Props, DeviceInformationKind.AssociationEndpoint);
        w.Added += (_, d) => { if (!string.IsNullOrWhiteSpace(d.Name)) Found?.Invoke(ToDevice(d, false)); };
        w.Updated += (_, _) => { };              // must be subscribed or Added stops firing
        w.EnumerationCompleted += (_, _) => { };
        w.Stopped += (_, _) => ScanDone?.Invoke();
        _watcher = w;
        w.Start();
        // On a timer thread: an error stopping the watcher (it stopped by itself just then) must not end Ember.
        _stopTimer = new System.Threading.Timer(_ => { try { StopScan(); } catch (Exception ex) { SystemBridge.Log("bt: stop scan: " + ex.Message); } }, null, Math.Clamp(seconds, 5, 60) * 1000, Timeout.Infinite);
    }

    public void StopScan()
    {
        _stopTimer?.Dispose();
        _stopTimer = null;
        var w = _watcher;
        _watcher = null;
        if (w is { Status: DeviceWatcherStatus.Started or DeviceWatcherStatus.EnumerationCompleted })
            w.Stop();
    }

    public async Task<(bool Ok, string? Error)> PairAsync(string id)
    {
        var d = await DeviceInformation.CreateFromIdAsync(id, Props, DeviceInformationKind.AssociationEndpoint);
        if (d.Pairing.IsPaired) return (true, null);
        if (!d.Pairing.CanPair) return (false, "This device can't be paired right now");

        var custom = d.Pairing.Custom;
        custom.PairingRequested += OnPairingRequested;
        try
        {
            var r = await custom.PairAsync(
                DevicePairingKinds.ConfirmOnly | DevicePairingKinds.DisplayPin | DevicePairingKinds.ConfirmPinMatch,
                DevicePairingProtectionLevel.Default);
            return r.Status switch
            {
                DevicePairingResultStatus.Paired or DevicePairingResultStatus.AlreadyPaired => (true, null),
                DevicePairingResultStatus.AuthenticationTimeout or DevicePairingResultStatus.ConnectionRejected => (false, "The device didn't respond. Is it still in pairing mode?"),
                DevicePairingResultStatus.RejectedByHandler or DevicePairingResultStatus.PairingCanceled => (false, "Pairing was cancelled"),
                _ => (false, r.Status.ToString()),
            };
        }
        finally
        {
            custom.PairingRequested -= OnPairingRequested;
            _pinConfirm = null;
        }
    }

    private async void OnPairingRequested(DeviceInformationCustomPairing sender, DevicePairingRequestedEventArgs e)
    {
        // async void on Windows' thread: an error here (the device went away mid-way) would end Ember.
        try { await AnswerPairingAsync(e); }
        catch (Exception ex) { SystemBridge.Log("bt: pairing: " + ex.Message); }
    }

    private async Task AnswerPairingAsync(DevicePairingRequestedEventArgs e)
    {
        switch (e.PairingKind)
        {
            case DevicePairingKinds.ConfirmOnly:
                e.Accept();
                break;

            case DevicePairingKinds.DisplayPin:
                Pin?.Invoke(e.Pin, false);
                e.Accept();
                break;

            case DevicePairingKinds.ConfirmPinMatch:
                // Ask the user in the UI; Windows waits on the deferral until we answer.
                var deferral = e.GetDeferral();
                try
                {
                    var tcs = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
                    _pinConfirm = tcs;
                    Pin?.Invoke(e.Pin, true);
                    var answered = await Task.WhenAny(tcs.Task, Task.Delay(TimeSpan.FromSeconds(60)));
                    if (answered == tcs.Task && tcs.Task.Result) e.Accept();
                }
                finally { deferral.Complete(); }   // always, or Windows keeps waiting
                break;
        }
    }

    public void ConfirmPin(bool accept) => _pinConfirm?.TrySetResult(accept);

    public async Task<(bool Ok, string? Error)> UnpairAsync(string id)
    {
        var d = await DeviceInformation.CreateFromIdAsync(id, Props, DeviceInformationKind.AssociationEndpoint);
        var r = await d.Pairing.UnpairAsync();
        return r.Status is DeviceUnpairingResultStatus.Unpaired or DeviceUnpairingResultStatus.AlreadyUnpaired
            ? (true, null) : (false, r.Status.ToString());
    }

    public void Dispose() => StopScan();
}
