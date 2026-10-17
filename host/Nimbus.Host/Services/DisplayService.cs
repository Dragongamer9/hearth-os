using System.Management;

namespace Nimbus.Host.Services;

public sealed record DisplayState(bool Supported, int Brightness, string Resolution, string Scale,
    int Width = 0, int Height = 0, int RefreshRate = 0, List<DisplayMode>? Modes = null,
    int ScalePercent = 0, int ScaleRecommended = 0, int[]? Scales = null);

/// <summary>
/// Brightness through WMI (built-in panels on laptops and handhelds).
/// External monitors need DDC/CI, which isn't implemented yet.
/// </summary>
public sealed class DisplayService
{
    private static readonly ManagementScope Scope = new(@"\\.\root\WMI");

    public async Task<DisplayState> GetAsync(Control ui)
    {
        var screen = Screen.FromControl(ui);
        var device = screen.DeviceName;
        int? brightness = await Task.Run(ReadBrightness);
        var (w, h, hz) = DisplayModes.Current(device);
        if (w == 0) (w, h) = (screen.Bounds.Width, screen.Bounds.Height);
        var modes = await Task.Run(() => DisplayModes.List(device));
        var sc = DisplayModes.Scale(device);
        int pct = sc?.Current ?? (int)Math.Round(ui.DeviceDpi / 96.0 * 100);
        return new DisplayState(brightness is not null, brightness ?? 0, $"{w} × {h}", $"{pct}%",
                                w, h, hz, modes, pct, sc?.Recommended ?? 0, sc?.Allowed);
    }

    public static string DeviceOf(Control ui) => Screen.FromControl(ui).DeviceName;

    public Task SetModeAsync(Control ui, int w, int h, int hz)
    {
        var device = DeviceOf(ui);
        return Task.Run(() => DisplayModes.SetMode(device, w, h, hz));
    }

    public Task SetScaleAsync(Control ui, int percent)
    {
        var device = DeviceOf(ui);
        return Task.Run(() => DisplayModes.SetScale(device, percent));
    }

    public async Task<int?> GetBrightnessAsync() => await Task.Run(ReadBrightness);

    public Task SetBrightnessAsync(int percent) => Task.Run(() =>
    {
        using var searcher = new ManagementObjectSearcher(Scope, new SelectQuery("WmiMonitorBrightnessMethods"));
        using var results = searcher.Get();
        bool any = false;
        foreach (ManagementObject o in results)
        {
            using (o) o.InvokeMethod("WmiSetBrightness", new object[] { (uint)1, (byte)Math.Clamp(percent, 0, 100) });
            any = true;
        }
        if (!any) throw new InvalidOperationException("This display doesn't support brightness control");
    });

    private static int? ReadBrightness()
    {
        try
        {
            using var searcher = new ManagementObjectSearcher(Scope, new SelectQuery("WmiMonitorBrightness"));
            using var results = searcher.Get();
            foreach (ManagementObject o in results)
                using (o) return Convert.ToInt32(o["CurrentBrightness"]);
        }
        catch (ManagementException) { /* class not present: no internal panel */ }
        return null;
    }
}
