using System.Runtime.InteropServices;

namespace Nimbus.Host.Services;

public sealed record DisplayMode(int Width, int Height, int[] Rates);

/// <summary>
/// Resolution and refresh rate (EnumDisplaySettings / ChangeDisplaySettingsEx) and display scale for one monitor.
/// Windows has no public API for scale; this uses the same DisplayConfig calls the Settings app uses
/// (DISPLAYCONFIG_DEVICE_INFO_GET/SET_DPI_SCALE, values relative to the monitor's recommended scale).
/// </summary>
public static class DisplayModes
{
    public static readonly int[] ScaleSteps = { 100, 125, 150, 175, 200, 225, 250, 300, 350, 400, 450, 500 };

    // ------------------------------------------------------------------ resolution & refresh

    public static (int W, int H, int Hz) Current(string device)
    {
        var dm = NewDevMode();
        if (!EnumDisplaySettingsEx(device, ENUM_CURRENT_SETTINGS, ref dm, 0)) return (0, 0, 0);
        return ((int)dm.dmPelsWidth, (int)dm.dmPelsHeight, (int)dm.dmDisplayFrequency);
    }

    /// <summary>Every resolution the monitor offers at 32-bit color, biggest first, each with its refresh rates.</summary>
    public static List<DisplayMode> List(string device)
    {
        var map = new Dictionary<(int, int), SortedSet<int>>();
        var dm = NewDevMode();
        for (int i = 0; EnumDisplaySettingsEx(device, i, ref dm, 0); i++)
        {
            if (dm.dmBitsPerPel != 32 || dm.dmPelsWidth < 800 || dm.dmPelsHeight < 600) continue;
            var key = ((int)dm.dmPelsWidth, (int)dm.dmPelsHeight);
            if (!map.TryGetValue(key, out var rates)) map[key] = rates = new SortedSet<int>();
            if (dm.dmDisplayFrequency > 1) rates.Add((int)dm.dmDisplayFrequency);
            dm = NewDevMode();
        }
        return map.OrderByDescending(kv => kv.Key.Item1 * kv.Key.Item2).ThenByDescending(kv => kv.Key.Item1)
                  .Select(kv => new DisplayMode(kv.Key.Item1, kv.Key.Item2, kv.Value.Reverse().ToArray())).ToList();
    }

    public static void SetMode(string device, int w, int h, int hz)
    {
        var dm = NewDevMode();
        if (!EnumDisplaySettingsEx(device, ENUM_CURRENT_SETTINGS, ref dm, 0))
            throw new InvalidOperationException("Couldn't read the current display settings.");
        dm.dmPelsWidth = (uint)w;
        dm.dmPelsHeight = (uint)h;
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
        if (hz > 0) { dm.dmDisplayFrequency = (uint)hz; dm.dmFields |= DM_DISPLAYFREQUENCY; }

        int test = ChangeDisplaySettingsEx(device, ref dm, IntPtr.Zero, CDS_TEST, IntPtr.Zero);
        if (test != DISP_CHANGE_SUCCESSFUL) throw new InvalidOperationException($"This display doesn't support {w} × {h}{(hz > 0 ? $" at {hz} Hz" : "")}.");
        int r = ChangeDisplaySettingsEx(device, ref dm, IntPtr.Zero, CDS_UPDATEREGISTRY, IntPtr.Zero);
        if (r == DISP_CHANGE_RESTART) throw new InvalidOperationException("Windows needs a restart to apply this display mode.");
        if (r != DISP_CHANGE_SUCCESSFUL) throw new InvalidOperationException($"Windows couldn't change the display mode (code {r}).");
    }

    // ------------------------------------------------------------------ scale

    /// <summary>Current scale, the recommended one, and the scales this monitor allows (percent).</summary>
    public static (int Current, int Recommended, int[] Allowed)? Scale(string device)
    {
        if (FindSource(device) is not { } src) return null;
        var get = new DpiScaleGet
        {
            header = new Header { type = DEVICE_INFO_GET_DPI_SCALE, size = Marshal.SizeOf<DpiScaleGet>(), adapterId = src.adapter, id = src.id },
        };
        if (DisplayConfigGetDeviceInfo(ref get) != 0) return null;
        int rec = Math.Abs(get.minScaleRel);                     // index of the recommended step
        int cur = Math.Clamp(rec + get.curScaleRel, 0, ScaleSteps.Length - 1);
        int max = Math.Clamp(rec + get.maxScaleRel, 0, ScaleSteps.Length - 1);
        return (ScaleSteps[cur], ScaleSteps[Math.Min(rec, ScaleSteps.Length - 1)], ScaleSteps[..(max + 1)]);
    }

    public static void SetScale(string device, int percent)
    {
        if (FindSource(device) is not { } src) throw new InvalidOperationException("Couldn't find this display.");
        var info = Scale(device) ?? throw new InvalidOperationException("This display doesn't support changing the scale.");
        int target = Array.IndexOf(ScaleSteps, percent);
        if (target < 0 || !info.Allowed.Contains(percent)) throw new InvalidOperationException($"{percent}% isn't available on this display.");
        int rec = Array.IndexOf(ScaleSteps, info.Recommended);
        var set = new DpiScaleSet
        {
            header = new Header { type = DEVICE_INFO_SET_DPI_SCALE, size = Marshal.SizeOf<DpiScaleSet>(), adapterId = src.adapter, id = src.id },
            scaleRel = target - rec,
        };
        int r = DisplayConfigSetDeviceInfo(ref set);
        if (r != 0) throw new InvalidOperationException($"Windows couldn't change the scale (code {r}).");
    }

    private static (Luid adapter, uint id)? FindSource(string device)
    {
        if (GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, out uint np, out uint nm) != 0) return null;
        var paths = new PathInfo[np];
        var modes = new ModeInfo[nm];
        if (QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, ref np, paths, ref nm, modes, IntPtr.Zero) != 0) return null;
        for (int i = 0; i < np; i++)
        {
            var name = new SourceName
            {
                header = new Header { type = DEVICE_INFO_GET_SOURCE_NAME, size = Marshal.SizeOf<SourceName>(), adapterId = paths[i].sourceAdapter, id = paths[i].sourceId },
            };
            if (DisplayConfigGetDeviceInfo(ref name) != 0) continue;
            if (string.Equals(name.viewGdiDeviceName, device, StringComparison.OrdinalIgnoreCase))
                return (paths[i].sourceAdapter, paths[i].sourceId);
        }
        return null;
    }

    // ------------------------------------------------------------------ interop

    private const int ENUM_CURRENT_SETTINGS = -1;
    private const uint DM_PELSWIDTH = 0x80000, DM_PELSHEIGHT = 0x100000, DM_DISPLAYFREQUENCY = 0x400000;
    private const uint CDS_UPDATEREGISTRY = 0x1, CDS_TEST = 0x2;
    private const int DISP_CHANGE_SUCCESSFUL = 0, DISP_CHANGE_RESTART = 1;
    private const uint QDC_ONLY_ACTIVE_PATHS = 2;
    private const int DEVICE_INFO_GET_SOURCE_NAME = 1, DEVICE_INFO_GET_DPI_SCALE = -3, DEVICE_INFO_SET_DPI_SCALE = -4;

    private static DEVMODE NewDevMode() => new() { dmSize = (ushort)Marshal.SizeOf<DEVMODE>() };

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct DEVMODE
    {
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string dmDeviceName;
        public ushort dmSpecVersion, dmDriverVersion, dmSize, dmDriverExtra;
        public uint dmFields;
        public int dmPositionX, dmPositionY;
        public uint dmDisplayOrientation, dmDisplayFixedOutput;
        public short dmColor, dmDuplex, dmYResolution, dmTTOption, dmCollate;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string dmFormName;
        public ushort dmLogPixels;
        public uint dmBitsPerPel, dmPelsWidth, dmPelsHeight, dmDisplayFlags, dmDisplayFrequency;
        public uint dmICMMethod, dmICMIntent, dmMediaType, dmDitherType, dmReserved1, dmReserved2, dmPanningWidth, dmPanningHeight;
    }

    [StructLayout(LayoutKind.Sequential)] private struct Luid { public uint Low; public int High; }

    [StructLayout(LayoutKind.Sequential)]
    private struct Header { public int type; public int size; public Luid adapterId; public uint id; }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct SourceName
    {
        public Header header;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string viewGdiDeviceName;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct DpiScaleGet { public Header header; public int minScaleRel; public int curScaleRel; public int maxScaleRel; }

    [StructLayout(LayoutKind.Sequential)]
    private struct DpiScaleSet { public Header header; public int scaleRel; }

    // DISPLAYCONFIG_PATH_INFO (72 bytes): only the source part is used.
    [StructLayout(LayoutKind.Sequential, Size = 72)]
    private struct PathInfo { public Luid sourceAdapter; public uint sourceId; public uint sourceModeIdx; public uint sourceFlags; }

    [StructLayout(LayoutKind.Sequential, Size = 64)]
    private struct ModeInfo { public uint infoType; public uint id; public Luid adapterId; }

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern bool EnumDisplaySettingsEx(string device, int mode, ref DEVMODE dm, uint flags);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int ChangeDisplaySettingsEx(string device, ref DEVMODE dm, IntPtr hwnd, uint flags, IntPtr lParam);
    [DllImport("user32.dll")] private static extern int GetDisplayConfigBufferSizes(uint flags, out uint numPaths, out uint numModes);
    [DllImport("user32.dll")]
    private static extern int QueryDisplayConfig(uint flags, ref uint numPaths, [Out] PathInfo[] paths, ref uint numModes, [Out] ModeInfo[] modes, IntPtr topology);
    [DllImport("user32.dll", EntryPoint = "DisplayConfigGetDeviceInfo")] private static extern int DisplayConfigGetDeviceInfo(ref SourceName info);
    [DllImport("user32.dll", EntryPoint = "DisplayConfigGetDeviceInfo")] private static extern int DisplayConfigGetDeviceInfo(ref DpiScaleGet info);
    [DllImport("user32.dll", EntryPoint = "DisplayConfigSetDeviceInfo")] private static extern int DisplayConfigSetDeviceInfo(ref DpiScaleSet info);
}
