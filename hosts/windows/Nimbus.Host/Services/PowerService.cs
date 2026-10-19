using System.Runtime.InteropServices;

namespace Nimbus.Host.Services;

public sealed record BatteryState(bool Present, int? Pct, bool Charging);
/// <summary>SecondsLeft: Windows' estimate of the battery time left (-1 when it has none, e.g. while charging).</summary>
public sealed record PowerInfo(bool Present, int? Pct, bool Charging, string Mode, int SecondsLeft = -1);

/// <summary>Battery status and the Windows power mode slider (efficiency / balanced / performance).</summary>
public sealed class PowerService
{
    // Power-mode overlay schemes (the slider in Windows Settings › Power).
    private static readonly Guid Efficiency = new("961cc777-2547-4f9d-8174-7d86181b8a7a");
    private static readonly Guid Balanced = Guid.Empty;
    private static readonly Guid Performance = new("ded574b5-45a0-4f42-8737-46345c09c238");

    [DllImport("powrprof.dll")] private static extern uint PowerGetEffectiveOverlayScheme(out Guid overlay);
    [DllImport("powrprof.dll")] private static extern uint PowerSetActiveOverlayScheme(Guid overlay);

    public BatteryState Battery()
    {
        var ps = SystemInformation.PowerStatus;
        bool present = !ps.BatteryChargeStatus.HasFlag(BatteryChargeStatus.NoSystemBattery)
                       && ps.BatteryChargeStatus != BatteryChargeStatus.Unknown;
        int? pct = present && ps.BatteryLifePercent is >= 0f and <= 1f ? (int)Math.Round(ps.BatteryLifePercent * 100) : null;
        return new BatteryState(present, pct, ps.PowerLineStatus == PowerLineStatus.Online);
    }

    public string Mode()
    {
        try
        {
            if (PowerGetEffectiveOverlayScheme(out var g) != 0) return "balanced";
            if (g == Efficiency) return "efficiency";
            if (g == Performance) return "performance";
        }
        catch (EntryPointNotFoundException) { }
        return "balanced";
    }

    public void SetMode(string mode)
    {
        var g = mode switch { "efficiency" => Efficiency, "performance" => Performance, _ => Balanced };
        uint rc = PowerSetActiveOverlayScheme(g);
        if (rc != 0) throw new InvalidOperationException($"Windows refused the power mode change (error {rc}). It only works with the Balanced power plan.");
    }

    public PowerInfo Get()
    {
        var b = Battery();
        int left = -1;
        try { left = b.Present && !b.Charging ? SystemInformation.PowerStatus.BatteryLifeRemaining : -1; } catch { }
        return new PowerInfo(b.Present, b.Pct, b.Charging, Mode(), left);
    }
}
