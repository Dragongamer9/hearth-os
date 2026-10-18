using System.Diagnostics;

namespace Nimbus.Host;

internal static class Program
{
    /// <summary>True when Windows started Nimbus as the user's shell (instead of explorer.exe).</summary>
    public static bool IsShell { get; private set; }

    /// <summary>Set before closing to tell the watchdog why (e.g. installing an update).</summary>
    public static int ExitCode { get; set; } = CleanExit;

    [STAThread]
    private static int Main(string[] args)
    {
        // Switch to Hearth OS: a step that needs administrator rights (started by Ember after Windows' prompt).
        if (args.Contains("--admin", StringComparer.OrdinalIgnoreCase)) return Switch.AdminTasks.Run(args);

        // The desktop's "Game Mode" shortcut: ask the running Ember to come back, or start it.
        if (args.Contains("--game-mode", StringComparer.OrdinalIgnoreCase) && DesktopMode.SignalRunning()) return 0;

        bool shell = args.Contains("--shell", StringComparer.OrdinalIgnoreCase);
        bool child = args.Contains("--child", StringComparer.OrdinalIgnoreCase);

        // As the shell, run the real UI in a child process and watch it. If it crashes we restart it,
        // and if it keeps crashing we start Explorer so the user is never left on a black screen.
        if (shell && !child) return Watchdog();

        using var mutex = new Mutex(true, @"Local\NimbusShell", out bool first);
        if (!first) return 0;

        IsShell = shell;
        ApplicationConfiguration.Initialize();

        if (shell)
        {
            Application.SetUnhandledExceptionMode(UnhandledExceptionMode.CatchException);
            Application.ThreadException += (_, e) => SystemBridge.Log("UNHANDLED " + e.Exception);
        }

        // --windowed: run in a normal window for development instead of fullscreen.
        bool windowed = args.Contains("--windowed", StringComparer.OrdinalIgnoreCase);
        Application.Run(new ShellForm(windowed));
        return ExitCode;
    }

    /// <summary>Exit code the UI uses when it leaves on purpose (sign out, return to desktop).</summary>
    public const int CleanExit = 0;

    private static int Watchdog()
    {
        var exe = Environment.ProcessPath!;
        var crashes = new Queue<DateTime>();
        while (true)
        {
            int code;
            try
            {
                using var p = Process.Start(new ProcessStartInfo(exe, "--shell --child") { UseShellExecute = false })!;
                p.WaitForExit();
                code = p.ExitCode;
            }
            catch (Exception ex)
            {
                SystemBridge.Log("watchdog could not start Nimbus: " + ex.Message);
                code = -1;
            }

            if (code == CleanExit) return 0;
            if (code == Updates.UpdateService.UpdateExitCode) return 0; // the update script starts the new version

            SystemBridge.Log($"watchdog: Nimbus exited with code {code}");
            crashes.Enqueue(DateTime.UtcNow);
            while (crashes.Count > 0 && DateTime.UtcNow - crashes.Peek() > TimeSpan.FromMinutes(2)) crashes.Dequeue();
            if (crashes.Count >= 3 && Updates.UpdateService.TryRollback(asShell: true))
            {
                SystemBridge.Log("watchdog: new version keeps crashing, rolling back to the previous one");
                return 0;
            }
            if (crashes.Count >= 3)
            {
                // Something is badly wrong: give the user a normal Windows desktop for this session.
                SystemBridge.Log("watchdog: too many crashes, starting Explorer");
                ShellForm.StartExplorer();
                return 1;
            }
            Thread.Sleep(1000);
        }
    }
}
