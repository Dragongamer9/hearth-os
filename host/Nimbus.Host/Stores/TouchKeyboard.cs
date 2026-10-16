using System.Diagnostics;
using System.Runtime.InteropServices;

namespace Nimbus.Host.Stores;

/// <summary>
/// Shows or hides an on-screen keyboard for typing into web sign-in pages with a controller or touch.
/// Uses the Windows touch keyboard when it's available, otherwise the classic On-Screen Keyboard
/// (osk.exe), which works even when Explorer isn't running and can be used with the controller mouse.
/// </summary>
public static class TouchKeyboard
{
    [ComImport, Guid("37c994e7-432b-4834-a2f7-dce1f13b834b"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface ITipInvocation { void Toggle(IntPtr hwnd); }

    [DllImport("user32.dll")] private static extern IntPtr GetDesktopWindow();

    private static readonly Guid TipInvocationClsid = new("4ce576fa-83dc-4F88-951c-9d0782b4e376");

    public static void Toggle()
    {
        // As the shell (no Explorer) the touch keyboard often can't appear, so go straight to osk.exe.
        bool explorer = Process.GetProcessesByName("explorer").Length > 0;
        if (explorer && TryTouchKeyboard()) return;
        ToggleOsk();
    }

    private static bool TryTouchKeyboard()
    {
        try
        {
            var tabtip = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.CommonProgramFiles),
                                      "microsoft shared", "ink", "TabTip.exe");
            if (Process.GetProcessesByName("TabTip").Length == 0 && File.Exists(tabtip))
            {
                Process.Start(new ProcessStartInfo(tabtip) { UseShellExecute = true })?.Dispose();
                Thread.Sleep(400);
            }
            var type = Type.GetTypeFromCLSID(TipInvocationClsid);
            if (type is null) return false;
            var tip = (ITipInvocation)Activator.CreateInstance(type)!;
            tip.Toggle(GetDesktopWindow());
            Marshal.ReleaseComObject(tip);
            return true;
        }
        catch (Exception ex)
        {
            SystemBridge.Log("touch keyboard unavailable: " + ex.Message);
            return false;
        }
    }

    private static void ToggleOsk()
    {
        var running = Process.GetProcessesByName("osk");
        if (running.Length > 0)
        {
            foreach (var p in running) { try { p.CloseMainWindow(); } catch { } p.Dispose(); }
            return;
        }
        try { Process.Start(new ProcessStartInfo("osk.exe") { UseShellExecute = true })?.Dispose(); }
        catch (Exception ex) { SystemBridge.Log("osk failed: " + ex.Message); }
    }
}
