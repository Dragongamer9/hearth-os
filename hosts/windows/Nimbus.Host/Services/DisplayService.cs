using System.Management;
using System.Runtime.InteropServices;

namespace Nimbus.Host.Services;

public sealed record DisplayState(bool Supported, int Brightness, string Resolution, string Scale,
    int Width = 0, int Height = 0, int RefreshRate = 0, List<DisplayMode>? Modes = null,
    int ScalePercent = 0, int ScaleRecommended = 0, int[]? Scales = null);

/// <summary>
/// Brightness through WMI (built-in panels on laptops and handhelds), and DDC/CI for external monitors
/// (most monitors accept brightness over the cable; some have DDC/CI switched off in their own menu).
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
        percent = Math.Clamp(percent, 0, 100);
        bool any = false;
        try
        {
            using var searcher = new ManagementObjectSearcher(Scope, new SelectQuery("WmiMonitorBrightnessMethods"));
            using var results = searcher.Get();
            foreach (ManagementObject o in results)
            {
                using (o) o.InvokeMethod("WmiSetBrightness", new object[] { (uint)1, (byte)percent });
                any = true;
            }
        }
        catch (ManagementException) { /* no built-in panel */ }
        any |= Ddc.SetAll(percent);
        if (!any) throw new InvalidOperationException("This display doesn't support brightness control (for a monitor, turn on DDC/CI in its own menu)");
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
        return Ddc.Get();
    }

    /// <summary>DDC/CI: brightness of external monitors through the video cable (dxva2).</summary>
    private static class Ddc
    {
        private delegate bool MonitorEnumProc(IntPtr hMon, IntPtr hdc, IntPtr rect, IntPtr data);
        [DllImport("user32.dll")] private static extern bool EnumDisplayMonitors(IntPtr hdc, IntPtr clip, MonitorEnumProc cb, IntPtr data);
        [DllImport("dxva2.dll")] private static extern bool GetNumberOfPhysicalMonitorsFromHMONITOR(IntPtr hMon, out uint count);
        [DllImport("dxva2.dll")] private static extern bool GetPhysicalMonitorsFromHMONITOR(IntPtr hMon, uint count, [Out] PHYSICAL_MONITOR[] monitors);
        [DllImport("dxva2.dll")] private static extern bool DestroyPhysicalMonitors(uint count, PHYSICAL_MONITOR[] monitors);
        [DllImport("dxva2.dll")] private static extern bool GetMonitorBrightness(IntPtr h, out uint min, out uint cur, out uint max);
        [DllImport("dxva2.dll")] private static extern bool SetMonitorBrightness(IntPtr h, uint value);
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        private struct PHYSICAL_MONITOR { public IntPtr hPhysicalMonitor; [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string szPhysicalMonitorDescription; }

        private static void Each(Func<IntPtr, bool> fn)
        {
            var hmons = new List<IntPtr>();
            EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, (h, _, _, _) => { hmons.Add(h); return true; }, IntPtr.Zero);
            foreach (var hm in hmons)
            {
                if (!GetNumberOfPhysicalMonitorsFromHMONITOR(hm, out uint n) || n == 0) continue;
                var arr = new PHYSICAL_MONITOR[n];
                if (!GetPhysicalMonitorsFromHMONITOR(hm, n, arr)) continue;
                try { foreach (var m in arr) if (!fn(m.hPhysicalMonitor)) return; }
                finally { DestroyPhysicalMonitors(n, arr); }
            }
        }

        public static int? Get()
        {
            int? result = null;
            try
            {
                Each(h =>
                {
                    if (GetMonitorBrightness(h, out uint min, out uint cur, out uint max) && max > min)
                    { result = (int)Math.Round(100.0 * (cur - min) / (max - min)); return false; }
                    return true;
                });
            }
            catch { }
            return result;
        }

        public static bool SetAll(int percent)
        {
            bool any = false;
            try
            {
                Each(h =>
                {
                    if (GetMonitorBrightness(h, out uint min, out _, out uint max) && max > min)
                        any |= SetMonitorBrightness(h, (uint)(min + (max - min) * percent / 100.0));
                    return true;
                });
            }
            catch { }
            return any;
        }
    }
}
