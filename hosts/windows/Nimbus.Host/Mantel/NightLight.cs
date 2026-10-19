using System.Runtime.InteropServices;

namespace Nimbus.Host.Mantel;

/// <summary>
/// Hearth's night light on Windows: like wlsunset on Hearth OS (3700 K from 19:30 to 07:00, fading over half an
/// hour), the screen is warmed through the display's gamma ramp while Mantel runs. Windows' own Night light has
/// no API apps may use, so this is Mantel's own (it sits on top of Windows' if both are on). Some drivers or
/// screens refuse gamma changes: then <see cref="Available"/> turns false and Quick Access says so.
/// </summary>
internal sealed class NightLight : IDisposable
{
    private readonly System.Windows.Forms.Timer _timer = new() { Interval = 60_000 };
    private bool _on;
    private double _applied = -1;
    public bool Available { get; private set; } = true;

    public NightLight() { _timer.Tick += (_, _) => Apply(); }

    /// <summary>On: warms the screen whenever it's evening or night. Off: puts the normal colors back.</summary>
    public bool Enabled
    {
        get => _on;
        set
        {
            if (_on == value && (_on ? _timer.Enabled : _applied <= 0)) return;
            _on = value;
            if (value) { _timer.Start(); Apply(); }
            else { _timer.Stop(); Set(0); }
        }
    }

    // 3700 K against 6500 K (the screen's own white), per channel.
    private const double WarmG = 0.78, WarmB = 0.59;

    /// <summary>0 in the day, 1 at night, in between during the 30 minutes around 19:30 and 07:00.</summary>
    private static double Warmth(DateTime now)
    {
        double t = now.TimeOfDay.TotalMinutes, dusk = 19 * 60 + 30, dawn = 7 * 60, fade = 30;
        if (t >= dusk - fade / 2 && t < dusk + fade / 2) return (t - (dusk - fade / 2)) / fade;
        if (t >= dawn - fade / 2 && t < dawn + fade / 2) return 1 - (t - (dawn - fade / 2)) / fade;
        return t >= dusk || t < dawn ? 1 : 0;
    }

    private void Apply() => Set(_on ? Warmth(DateTime.Now) : 0);

    private void Set(double k)
    {
        if (Math.Abs(k - _applied) < 0.01) return;
        bool any = false, failed = false;
        var ramp = new ushort[768];
        double g = 1 - (1 - WarmG) * k, b = 1 - (1 - WarmB) * k;
        for (int i = 0; i < 256; i++)
        {
            int v = i * 257;
            ramp[i] = (ushort)v;
            ramp[256 + i] = (ushort)Math.Round(v * g);
            ramp[512 + i] = (ushort)Math.Round(v * b);
        }
        foreach (var screen in Screen.AllScreens)
        {
            var dc = CreateDC(screen.DeviceName, null, null, IntPtr.Zero);
            if (dc == IntPtr.Zero) continue;
            try { if (SetDeviceGammaRamp(dc, ramp)) any = true; else failed = true; }
            finally { DeleteDC(dc); }
        }
        if (k > 0 && failed && !any) { Available = false; SystemBridge.Log("night light: the display refused the gamma change"); }
        else if (any) Available = true;
        _applied = k;
    }

    public void Dispose()
    {
        _timer.Stop();
        if (_applied > 0) { _on = false; Set(0); }
        _timer.Dispose();
    }

    [DllImport("gdi32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr CreateDC(string? driver, string? device, string? output, IntPtr initData);
    [DllImport("gdi32.dll")] private static extern bool DeleteDC(IntPtr dc);
    [DllImport("gdi32.dll")] private static extern bool SetDeviceGammaRamp(IntPtr dc, ushort[] ramp);
}
