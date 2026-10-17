using Microsoft.Win32;

namespace Nimbus.Host;

/// <summary>
/// What Settings › About and › System show: Windows edition, device, processor, graphics, memory, storage,
/// uptime. Read from the registry and .NET; anything missing is left empty. Same shape as the Linux host.
/// </summary>
static class SystemInfo
{
    static string Reg(string key, string name)
    {
        try { return (Registry.LocalMachine.OpenSubKey(key)?.GetValue(name) as string ?? "").Trim(); }
        catch { return ""; }
    }

    static string Gpus()
    {
        const string cls = @"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";
        var names = new List<string>();
        try
        {
            using var root = Registry.LocalMachine.OpenSubKey(cls);
            foreach (var sub in root?.GetSubKeyNames() ?? Array.Empty<string>())
            {
                if (!sub.All(char.IsDigit)) continue;
                var desc = (root!.OpenSubKey(sub)?.GetValue("DriverDesc") as string ?? "").Trim();
                if (desc.Length > 0 && !desc.Contains("Basic Display") && !names.Contains(desc)) names.Add(desc);
            }
        }
        catch { }
        return string.Join(", ", names);
    }

    public static object Get()
    {
        const string nt = @"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
        const string bios = @"HARDWARE\DESCRIPTION\System\BIOS";
        var build = Reg(nt, "CurrentBuild");
        var product = Reg(nt, "ProductName");
        // Windows 11 still reports "Windows 10" in ProductName; the build number tells them apart.
        if (int.TryParse(build, out var b) && b >= 22000) product = product.Replace("Windows 10", "Windows 11");
        var sysDrive = Path.GetPathRoot(Environment.SystemDirectory) ?? "C:\\";
        object? storage = null;
        try { var d = new DriveInfo(sysDrive); storage = new { path = sysDrive, totalBytes = d.TotalSize, freeBytes = d.AvailableFreeSpace }; } catch { }
        var vendor = Reg(bios, "SystemManufacturer");
        var model = Reg(bios, "SystemProductName");
        if (vendor.StartsWith("ASUSTeK", StringComparison.OrdinalIgnoreCase)) vendor = "ASUS";
        return new
        {
            os = product,
            osVersion = string.Join(" ", new[] { Reg(nt, "DisplayVersion"), build.Length > 0 ? $"(build {build})" : "" }.Where(s => s.Length > 0)),
            osPretty = product,
            device = string.Join(" ", new[] { vendor, model }.Where(s => s.Length > 0)),
            cpu = Reg(@"HARDWARE\DESCRIPTION\System\CentralProcessor\0", "ProcessorNameString"),
            cores = Environment.ProcessorCount,
            gpu = Gpus(),
            memoryBytes = GC.GetGCMemoryInfo().TotalAvailableMemoryBytes,
            storage,
            uptimeSec = Environment.TickCount64 / 1000,
            kernel = Environment.OSVersion.VersionString,
            hostname = Environment.MachineName,
        };
    }
}
