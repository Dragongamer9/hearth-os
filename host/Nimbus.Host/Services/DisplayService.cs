using System.Management;

namespace Nimbus.Host.Services;

public sealed record DisplayState(bool Supported, int Brightness, string Resolution, string Scale);

/// <summary>
/// Brightness through WMI (built-in panels on laptops and handhelds).
/// External monitors need DDC/CI, which isn't implemented yet.
/// </summary>
public sealed class DisplayService
{
    private static readonly ManagementScope Scope = new(@"\\.\root\WMI");

    public async Task<DisplayState> GetAsync(Control ui)
    {
        var bounds = Screen.FromControl(ui).Bounds;
        string resolution = $"{bounds.Width} × {bounds.Height}";
        string scale = $"{Math.Round(ui.DeviceDpi / 96.0 * 100)}%";
        int? brightness = await Task.Run(ReadBrightness);
        return new DisplayState(brightness is not null, brightness ?? 0, resolution, scale);
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
