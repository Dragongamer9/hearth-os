using System.Runtime.InteropServices;

namespace Nimbus.Host.Services;

public sealed record AudioDevice(string Id, string Name, bool IsDefault);
public sealed record AudioState(int Volume, bool Muted, List<AudioDevice> Devices);

/// <summary>
/// Master volume, mute and output-device selection through Windows Core Audio (COM).
/// Switching the default device uses IPolicyConfig: undocumented, but it's what Windows'
/// own sound settings use and has been stable since Windows 7.
/// </summary>
public sealed class AudioService
{
    private const int eRender = 0, DEVICE_STATE_ACTIVE = 1, CLSCTX_ALL = 23, STGM_READ = 0;
    private static readonly Guid IID_IAudioEndpointVolume = new("5CDF2C82-841E-4546-9722-0CF74078229A");
    private static PropertyKey PKEY_Device_FriendlyName = new(new Guid("a45c254e-df1c-4efd-8020-67d146a850e0"), 14);

    private static IMMDeviceEnumerator Enumerator() => (IMMDeviceEnumerator)new MMDeviceEnumeratorComObject();

    private static IAudioEndpointVolume? DefaultVolume()
    {
        try
        {
            Enumerator().GetDefaultAudioEndpoint(eRender, 1 /* eMultimedia */, out var dev);
            var iid = IID_IAudioEndpointVolume;
            dev.Activate(ref iid, CLSCTX_ALL, IntPtr.Zero, out var obj);
            return (IAudioEndpointVolume)obj;
        }
        catch (COMException) { return null; } // no output device at all
    }

    public AudioState Get()
    {
        int volume = 0; bool muted = false;
        var v = DefaultVolume();
        if (v is not null)
        {
            v.GetMasterVolumeLevelScalar(out var level);
            v.GetMute(out muted);
            volume = (int)Math.Round(level * 100);
        }
        return new AudioState(volume, muted, Devices());
    }

    public void SetVolume(int percent)
    {
        var v = DefaultVolume() ?? throw new InvalidOperationException("No sound output device");
        v.SetMasterVolumeLevelScalar(Math.Clamp(percent, 0, 100) / 100f, IntPtr.Zero);
    }

    public void SetMute(bool mute)
    {
        var v = DefaultVolume() ?? throw new InvalidOperationException("No sound output device");
        v.SetMute(mute, IntPtr.Zero);
    }

    public void SetDefault(string id)
    {
        var policy = (IPolicyConfig)new PolicyConfigComObject();
        for (int role = 0; role < 3; role++) // console, multimedia, communications
            Marshal.ThrowExceptionForHR(policy.SetDefaultEndpoint(id, role));
    }

    private static List<AudioDevice> Devices()
    {
        var list = new List<AudioDevice>();
        var en = Enumerator();
        string? defaultId = null;
        try { en.GetDefaultAudioEndpoint(eRender, 1, out var def); def.GetId(out defaultId); } catch (COMException) { }

        en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE, out var col);
        col.GetCount(out var count);
        for (uint i = 0; i < count; i++)
        {
            col.Item(i, out var dev);
            dev.GetId(out var id);
            list.Add(new AudioDevice(id, FriendlyName(dev) ?? "Audio device", id == defaultId));
        }
        return list;
    }

    private static string? FriendlyName(IMMDevice dev)
    {
        dev.OpenPropertyStore(STGM_READ, out var store);
        store.GetValue(ref PKEY_Device_FriendlyName, out var pv);
        try { return pv.vt == 31 /* VT_LPWSTR */ ? Marshal.PtrToStringUni(pv.p) : null; }
        finally { PropVariantClear(ref pv); }
    }

    // ------------------------------------------------------------------ interop

    [DllImport("ole32.dll")] private static extern int PropVariantClear(ref PropVariant pv);

    [StructLayout(LayoutKind.Sequential)]
    private struct PropertyKey { public Guid fmtid; public int pid; public PropertyKey(Guid f, int p) { fmtid = f; pid = p; } }

    [StructLayout(LayoutKind.Sequential)]
    private struct PropVariant { public ushort vt; public ushort r1, r2, r3; public IntPtr p; public IntPtr p2; }

    [ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")] private class MMDeviceEnumeratorComObject { }
    [ComImport, Guid("870AF99C-171D-4F9E-AF0D-E63DF40C2BC9")] private class PolicyConfigComObject { }

    [ComImport, Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IMMDeviceEnumerator
    {
        void EnumAudioEndpoints(int dataFlow, int stateMask, out IMMDeviceCollection devices);
        void GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice device);
        void GetDevice([MarshalAs(UnmanagedType.LPWStr)] string id, out IMMDevice device);
        void RegisterEndpointNotificationCallback(IntPtr client);
        void UnregisterEndpointNotificationCallback(IntPtr client);
    }

    [ComImport, Guid("0BD7A1BE-7A1A-44DB-8397-CC5392387B5E"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IMMDeviceCollection
    {
        void GetCount(out uint count);
        void Item(uint index, out IMMDevice device);
    }

    [ComImport, Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IMMDevice
    {
        void Activate(ref Guid iid, int clsCtx, IntPtr activationParams, [MarshalAs(UnmanagedType.IUnknown)] out object iface);
        void OpenPropertyStore(int access, out IPropertyStore store);
        void GetId([MarshalAs(UnmanagedType.LPWStr)] out string id);
        void GetState(out int state);
    }

    [ComImport, Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IPropertyStore
    {
        void GetCount(out uint count);
        void GetAt(uint index, out PropertyKey key);
        void GetValue(ref PropertyKey key, out PropVariant value);
        void SetValue(ref PropertyKey key, ref PropVariant value);
        void Commit();
    }

    // Vtable order matters; methods after GetMute are listed so the layout is complete.
    [ComImport, Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IAudioEndpointVolume
    {
        void RegisterControlChangeNotify(IntPtr notify);
        void UnregisterControlChangeNotify(IntPtr notify);
        void GetChannelCount(out uint count);
        void SetMasterVolumeLevel(float levelDb, IntPtr eventContext);
        void SetMasterVolumeLevelScalar(float level, IntPtr eventContext);
        void GetMasterVolumeLevel(out float levelDb);
        void GetMasterVolumeLevelScalar(out float level);
        void SetChannelVolumeLevel(uint channel, float levelDb, IntPtr eventContext);
        void SetChannelVolumeLevelScalar(uint channel, float level, IntPtr eventContext);
        void GetChannelVolumeLevel(uint channel, out float levelDb);
        void GetChannelVolumeLevelScalar(uint channel, out float level);
        void SetMute([MarshalAs(UnmanagedType.Bool)] bool mute, IntPtr eventContext);
        void GetMute([MarshalAs(UnmanagedType.Bool)] out bool mute);
        void GetVolumeStepInfo(out uint step, out uint stepCount);
        void VolumeStepUp(IntPtr eventContext);
        void VolumeStepDown(IntPtr eventContext);
        void QueryHardwareSupport(out uint mask);
        void GetVolumeRange(out float minDb, out float maxDb, out float incrementDb);
    }

    [ComImport, Guid("F8679F50-850A-41CF-9C72-430F290290C8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IPolicyConfig
    {
        [PreserveSig] int GetMixFormat([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr format);
        [PreserveSig] int GetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, int defaultFormat, IntPtr format);
        [PreserveSig] int ResetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id);
        [PreserveSig] int SetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr endpointFormat, IntPtr mixFormat);
        [PreserveSig] int GetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, int defaultPeriod, IntPtr defPeriod, IntPtr minPeriod);
        [PreserveSig] int SetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr period);
        [PreserveSig] int GetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr mode);
        [PreserveSig] int SetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr mode);
        [PreserveSig] int GetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr key, IntPtr value);
        [PreserveSig] int SetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr key, IntPtr value);
        [PreserveSig] int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string id, int role);
        [PreserveSig] int SetEndpointVisibility([MarshalAs(UnmanagedType.LPWStr)] string id, int visible);
    }
}
