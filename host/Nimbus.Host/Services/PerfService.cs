using System.Diagnostics;
using System.Runtime.InteropServices;

namespace Nimbus.Host.Services;

public sealed record PerfStats(int Cpu, int? Gpu, double RamUsedGb, double RamTotalGb, double? VramUsedGb);

/// <summary>
/// Live CPU, GPU and memory use for the Performance panel. CPU and RAM come from Win32; GPU use and video
/// memory come from Windows' "GPU Engine" and "GPU Adapter Memory" performance counters (what Task Manager shows).
/// </summary>
public sealed class PerfService
{
    private readonly object _lock = new();
    private ulong _idle, _kernel, _user;
    private Dictionary<string, PerformanceCounter> _gpu = new();
    private Dictionary<string, PerformanceCounter> _vram = new();
    private DateTime _instancesAt = DateTime.MinValue;
    private bool _gpuBroken;

    public PerfStats Sample()
    {
        lock (_lock)
        {
            int cpu = Cpu();
            var mem = new MEMORYSTATUSEX { dwLength = (uint)Marshal.SizeOf<MEMORYSTATUSEX>() };
            GlobalMemoryStatusEx(ref mem);
            const double Gb = 1024.0 * 1024 * 1024;
            var (gpu, vram) = Gpu();
            return new PerfStats(cpu, gpu, Math.Round((mem.ullTotalPhys - mem.ullAvailPhys) / Gb, 1), Math.Round(mem.ullTotalPhys / Gb, 1), vram);
        }
    }

    private int Cpu()
    {
        if (!GetSystemTimes(out var idle, out var kernel, out var user)) return 0;
        ulong i = idle - _idle, k = kernel - _kernel, u = user - _user;   // kernel time includes idle time
        bool first = _kernel == 0;
        _idle = idle; _kernel = kernel; _user = user;
        if (first || k + u == 0) return 0;
        return (int)Math.Clamp(Math.Round(100.0 * (k + u - i) / (k + u)), 0, 100);
    }

    private (int?, double?) Gpu()
    {
        if (_gpuBroken) return (null, null);
        try
        {
            // Engines come and go with processes; refresh the list every few seconds.
            if (DateTime.UtcNow - _instancesAt > TimeSpan.FromSeconds(5))
            {
                _instancesAt = DateTime.UtcNow;
                _gpu = Refresh(_gpu, "GPU Engine", "Utilization Percentage", n => n.Contains("engtype_3D", StringComparison.OrdinalIgnoreCase));
                _vram = Refresh(_vram, "GPU Adapter Memory", "Dedicated Usage", _ => true);
            }
            // Task Manager style: per adapter, add up every process's 3D engine use; show the busiest adapter.
            var perAdapter = new Dictionary<string, double>();
            foreach (var (name, c) in _gpu)
            {
                float v;
                try { v = c.NextValue(); } catch { continue; }
                var luid = Luid(name);
                perAdapter[luid] = perAdapter.GetValueOrDefault(luid) + v;
            }
            int? gpu = perAdapter.Count == 0 ? null : (int)Math.Clamp(Math.Round(perAdapter.Values.Max()), 0, 100);
            double vram = 0;
            foreach (var c in _vram.Values) { try { vram = Math.Max(vram, c.NextValue()); } catch { } }
            return (gpu, _vram.Count == 0 ? null : Math.Round(vram / (1024.0 * 1024 * 1024), 1));
        }
        catch (Exception ex)
        {
            SystemBridge.Log("perf: GPU counters unavailable: " + ex.Message);
            _gpuBroken = true;
            return (null, null);
        }
    }

    private static Dictionary<string, PerformanceCounter> Refresh(Dictionary<string, PerformanceCounter> old, string category, string counter, Func<string, bool> keep)
    {
        var names = new PerformanceCounterCategory(category).GetInstanceNames().Where(keep).ToHashSet();
        var next = new Dictionary<string, PerformanceCounter>();
        foreach (var (n, c) in old) { if (names.Contains(n)) next[n] = c; else c.Dispose(); }
        foreach (var n in names)
            if (!next.ContainsKey(n)) next[n] = new PerformanceCounter(category, counter, n, readOnly: true);
        return next;
    }

    // "pid_1234_luid_0x00000000_0x0000D1A4_phys_0_eng_0_engtype_3D" → "0x00000000_0x0000D1A4"
    private static string Luid(string instance)
    {
        int i = instance.IndexOf("luid_", StringComparison.OrdinalIgnoreCase);
        if (i < 0) return "";
        var rest = instance[(i + 5)..];
        int j = rest.IndexOf("_phys", StringComparison.OrdinalIgnoreCase);
        return j > 0 ? rest[..j] : rest;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MEMORYSTATUSEX
    {
        public uint dwLength, dwMemoryLoad;
        public ulong ullTotalPhys, ullAvailPhys, ullTotalPageFile, ullAvailPageFile, ullTotalVirtual, ullAvailVirtual, ullAvailExtendedVirtual;
    }
    [DllImport("kernel32.dll")] private static extern bool GlobalMemoryStatusEx(ref MEMORYSTATUSEX m);
    [DllImport("kernel32.dll")] private static extern bool GetSystemTimes(out ulong idle, out ulong kernel, out ulong user);
}
