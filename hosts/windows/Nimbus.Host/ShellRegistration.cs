using Microsoft.Win32;

namespace Nimbus.Host;

/// <summary>
/// Per-user shell registration: HKCU\Software\Microsoft\Windows NT\CurrentVersion\Winlogon\Shell.
/// Winlogon starts this instead of explorer.exe for the current user only. Other accounts,
/// and Safe Mode, keep using Explorer. Deleting the value restores Explorer.
/// </summary>
public static class ShellRegistration
{
    private const string KeyPath = @"Software\Microsoft\Windows NT\CurrentVersion\Winlogon";

    public static string? Current()
    {
        using var k = Registry.CurrentUser.OpenSubKey(KeyPath);
        return k?.GetValue("Shell") as string;
    }

    /// <summary>True when the per-user shell is set to Nimbus.</summary>
    public static bool IsNimbus() => Current()?.Contains("Nimbus", StringComparison.OrdinalIgnoreCase) == true;

    public static void Register(string exePath)
    {
        using var k = Registry.CurrentUser.CreateSubKey(KeyPath);
        k.SetValue("Shell", $"\"{exePath}\" --shell", RegistryValueKind.String);
    }

    /// <summary>Writes the Shell value as given (Desktop Mode points it at Explorer for a moment).</summary>
    public static void SetRaw(string value)
    {
        using var k = Registry.CurrentUser.CreateSubKey(KeyPath);
        k.SetValue("Shell", value, RegistryValueKind.String);
    }

    public static void Unregister()
    {
        using var k = Registry.CurrentUser.OpenSubKey(KeyPath, writable: true);
        if (k?.GetValue("Shell") is not null) k.DeleteValue("Shell");
    }
}
