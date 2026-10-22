using System.Runtime.InteropServices;
using Microsoft.Win32;

namespace Nimbus.Host;

/// <summary>
/// Command-line tools that come with Ember (hearthfetch, hosts/hearthfetch) are in its bin folder. That folder goes on
/// your PATH (your account only), so `hearthfetch` works in any new terminal; the programs Ember starts get it at once.
/// The PATH keeps its kind (expandable entries like %USERPROFILE% stay as they are).
/// </summary>
internal static class CliPath
{
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr SendMessageTimeout(IntPtr hwnd, uint msg, UIntPtr wParam, string lParam, uint flags, uint timeout, out UIntPtr result);

    public static void Ensure()
    {
        try
        {
            var bin = Path.Combine(AppContext.BaseDirectory, "bin");
            if (!File.Exists(Path.Combine(bin, "hearthfetch.exe"))) return;
            bool Same(string p)
            {
                try { return string.Equals(Path.GetFullPath(Environment.ExpandEnvironmentVariables(p.Trim())).TrimEnd('\\'), bin.TrimEnd('\\'), StringComparison.OrdinalIgnoreCase); }
                catch { return false; }
            }
            // This process (and what it starts from now on).
            var mine = Environment.GetEnvironmentVariable("Path") ?? "";
            if (!mine.Split(';', StringSplitOptions.RemoveEmptyEntries).Any(Same)) Environment.SetEnvironmentVariable("Path", mine.TrimEnd(';') + ";" + bin);
            // Your account's PATH (new terminals).
            using var key = Registry.CurrentUser.OpenSubKey("Environment", writable: true);
            if (key is null) return;
            var kind = key.GetValueNames().Contains("Path", StringComparer.OrdinalIgnoreCase) ? key.GetValueKind("Path") : RegistryValueKind.ExpandString;
            // A PATH stored some other way (not text) is left alone: reading it as empty and writing it back would
            // wipe every folder on it.
            if (kind is not (RegistryValueKind.String or RegistryValueKind.ExpandString)) { SystemBridge.Log("hearthfetch: your PATH isn't stored as text, left as it is"); return; }
            var raw = key.GetValue("Path", "", RegistryValueOptions.DoNotExpandEnvironmentNames) as string ?? "";
            var parts = raw.Split(';', StringSplitOptions.RemoveEmptyEntries).ToList();
            if (parts.Any(Same)) return;
            parts.Add(bin);
            key.SetValue("Path", string.Join(";", parts), kind == RegistryValueKind.String ? RegistryValueKind.String : RegistryValueKind.ExpandString);
            // Tell the desktop the environment changed (terminals opened from Explorer then see it).
            SendMessageTimeout((IntPtr)0xFFFF, 0x001A /* WM_SETTINGCHANGE */, UIntPtr.Zero, "Environment", 0x0002 /* SMTO_ABORTIFHUNG */, 2000, out _);
            SystemBridge.Log("hearthfetch: Ember's bin folder is on your PATH now");
        }
        catch (Exception ex) { SystemBridge.Log("hearthfetch: couldn't add Ember's bin folder to your PATH: " + ex.Message); }
    }
}
