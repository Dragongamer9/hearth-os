using System.Runtime.InteropServices;

namespace Nimbus.Host.Services;

/// <summary>Can Windows turn off with everything kept (hibernation)? Quick Resume's Shut down then hibernates when games are open.</summary>
public static class Hibernation
{
    [StructLayout(LayoutKind.Sequential)]
    private struct Caps
    {
        [MarshalAs(UnmanagedType.U1)] public bool PowerButtonPresent, SleepButtonPresent, LidPresent, S1, S2, S3, S4, S5, HiberFilePresent, FullWake, VideoDimPresent, ApmPresent, UpsPresent, ThermalControl, ProcessorThrottle;
        public byte ProcessorMinThrottle, ProcessorMaxThrottle;
        [MarshalAs(UnmanagedType.U1)] public bool FastSystemS4;
        [MarshalAs(UnmanagedType.ByValArray, SizeConst = 3)] public byte[] Spare2;
        [MarshalAs(UnmanagedType.U1)] public bool DiskSpinDown;
        [MarshalAs(UnmanagedType.ByValArray, SizeConst = 8)] public byte[] Spare3;
        [MarshalAs(UnmanagedType.U1)] public bool SystemBatteriesPresent, BatteriesAreShortTerm;
        [MarshalAs(UnmanagedType.ByValArray, SizeConst = 72)] public byte[] Rest;
    }
    [DllImport("powrprof.dll")] private static extern bool GetPwrCapabilities(out Caps caps);

    public static object Status()
    {
        try
        {
            // restartKeep is always false: Windows has no "hibernate, then restart", so Restart closes the games.
            if (GetPwrCapabilities(out var c) && c.S4 && c.HiberFilePresent) return new { supported = true, canSetUp = false, restartKeep = false, reason = (string?)null };
            return new { supported = false, canSetUp = false, restartKeep = false, reason = (string?)"Quick Resume keeps your games through sleep only: hibernation is off in Windows. Turn it on (powercfg /hibernate on, as administrator) and Shut down keeps them too." };
        }
        catch { return new { supported = false, canSetUp = false, restartKeep = false, reason = (string?)null }; }
    }
}
