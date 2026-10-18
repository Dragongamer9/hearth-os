using System.Runtime.InteropServices;

namespace Nimbus.Host.Services;

/// <summary>Can Windows turn off with everything kept (hibernation)? For Quick Resume's "Turn off and keep games".</summary>
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
            if (GetPwrCapabilities(out var c) && c.S4 && c.HiberFilePresent) return new { supported = true, canSetUp = false, reason = (string?)null };
            return new { supported = false, canSetUp = false, reason = (string?)"Hibernation is off in Windows. Turn it on (powercfg /hibernate on as administrator) to keep Quick Resume games when the PC turns off." };
        }
        catch { return new { supported = false, canSetUp = false, reason = (string?)null }; }
    }
}
