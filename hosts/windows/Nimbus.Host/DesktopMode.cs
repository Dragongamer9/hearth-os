using System.Diagnostics;
using System.Runtime.InteropServices;

namespace Nimbus.Host;

/// <summary>
/// Ember's Desktop Mode, like Hearth's on Linux: the normal Windows desktop (Explorer: taskbar, Start, tray,
/// notifications), with a quick way back to Game Mode (Ember fullscreen).
///
/// Going to the desktop: Explorer only becomes the full desktop when it's the shell Windows is set to use, so
/// the per-user Shell value is pointed back at Explorer for the moment Explorer starts, then set to Ember again
/// (Ember stays the shell for the next sign-in). Ember's window hides; the controller works as a mouse.
/// Back to Game Mode: the "Game Mode" desktop shortcut, the tray icon, Ctrl+Alt+G, or the Xbox button's
/// overlay (Back to Ember). Explorer is asked to close the way its own "Exit Explorer" does, then Ember
/// comes back fullscreen.
///
/// When Ember isn't the shell (run as an app), Explorer is already there: Desktop Mode just gets Ember out
/// of the way, and the same ways back bring it to the front.
/// </summary>
public sealed class DesktopMode : IDisposable
{
    private readonly ShellForm _ui;
    private NotifyIcon? _tray;
    private EventWaitHandle? _signal;
    private Thread? _signalThread;
    private volatile bool _stop;

    /// <summary>Signalled by "Ember.exe --game-mode" (the desktop shortcut) to bring Game Mode back.</summary>
    public const string SignalName = @"Local\EmberGameMode";

    public bool Active { get; private set; }
    private bool _busy;
    /// <summary>Runs once the desktop is up (Mantel starts here) and before it's left (Mantel stops).</summary>
    public Func<Task>? DesktopShown { get; set; }
    public Action? DesktopLeaving { get; set; }
    public event Action<bool>? Changed;

    public DesktopMode(ShellForm ui)
    {
        _ui = ui;
        try
        {
            _signal = new EventWaitHandle(false, EventResetMode.AutoReset, SignalName);
            _signalThread = new Thread(() =>
            {
                while (!_stop)
                {
                    if (_signal.WaitOne(500) && !_stop) _ui.BeginInvoke((MethodInvoker)(() => Leave()));
                }
            }) { IsBackground = true, Name = "Ember game mode signal" };
            _signalThread.Start();
        }
        catch (Exception ex) { SystemBridge.Log("desktop mode signal: " + ex.Message); }
    }

    /// <summary>Asks a running Ember to go back to Game Mode (used by the --game-mode shortcut).</summary>
    public static bool SignalRunning()
    {
        try { using var e = EventWaitHandle.OpenExisting(SignalName); return e.Set(); }
        catch { return false; }
    }

    // ------------------------------------------------------------------ switching

    public async void Enter()
    {
        if (Active || _busy) return;
        _busy = true;
        try
        {
            SystemBridge.Log("desktop mode: entering");
            Active = true;
            _ui.Hide();
            ShowTray();
            Changed?.Invoke(true);
            // Off the UI thread: waiting for Explorer's taskbar can take a few seconds.
            if (Program.IsShell && !ExplorerShellRunning()) await Task.Run(StartExplorerAsDesktop);
            await Task.Run(EnsureShortcut);
            if (DesktopShown is not null) await DesktopShown();
        }
        catch (Exception ex) { SystemBridge.Log("desktop mode: " + ex.Message); }
        finally { _busy = false; }
    }

    public async void Leave()
    {
        if (_busy) return;
        if (!Active) { _ui.ShowFullscreen(); return; }
        _busy = true;
        try
        {
            SystemBridge.Log("desktop mode: leaving");
            _ui.ShowFullscreen();   // in front right away; Explorer closes behind it
            try { DesktopLeaving?.Invoke(); } catch (Exception ex) { SystemBridge.Log("desktop mode: " + ex.Message); }
            if (Program.IsShell) await Task.Run(CloseExplorer);
            Active = false;
            HideTray();
            _ui.ShowFullscreen();
            Changed?.Invoke(false);
        }
        catch (Exception ex) { SystemBridge.Log("desktop mode: " + ex.Message); }
        finally { _busy = false; }
    }

    /// <summary>Explorer becomes the desktop only if it's the configured shell: point the per-user Shell value
    /// at it while it starts, then put Ember back (so Ember is still what starts at the next sign-in).</summary>
    private static void StartExplorerAsDesktop()
    {
        string? ours = ShellRegistration.Current();
        try
        {
            ShellRegistration.SetRaw("explorer.exe");
            ShellForm.StartExplorer();
            var until = DateTime.UtcNow.AddSeconds(15);
            while (DateTime.UtcNow < until && !ExplorerShellRunning()) Thread.Sleep(200);
            if (!ExplorerShellRunning()) SystemBridge.Log("desktop mode: Explorer's taskbar didn't appear within 15 s");
            Thread.Sleep(800);   // let Explorer finish reading the value before it changes back
        }
        finally
        {
            if (ours is null) ShellRegistration.Unregister(); else ShellRegistration.SetRaw(ours);
        }
    }

    /// <summary>Closes Explorer the way its own "Exit Explorer" does (so it saves its state), and makes sure
    /// it's gone after a few seconds. File Explorer windows close with it.</summary>
    private static void CloseExplorer()
    {
        var tray = FindWindow("Shell_TrayWnd", null);
        if (tray != IntPtr.Zero) PostMessage(tray, WM_USER + 436, IntPtr.Zero, IntPtr.Zero);
        var until = DateTime.UtcNow.AddSeconds(5);
        while (DateTime.UtcNow < until && Explorers().Any()) Thread.Sleep(150);
        foreach (var p in Explorers()) { try { p.Kill(); } catch { } p.Dispose(); }
    }

    private static IEnumerable<Process> Explorers()
    {
        int session = Process.GetCurrentProcess().SessionId;
        return Process.GetProcessesByName("explorer").Where(p => { try { return p.SessionId == session; } catch { return false; } });
    }

    public static bool ExplorerShellRunning() => FindWindow("Shell_TrayWnd", null) != IntPtr.Zero;

    // ------------------------------------------------------------------ ways back

    private void ShowTray()
    {
        if (_tray is null)
        {
            var menu = new ContextMenuStrip();
            menu.Items.Add("Return to Game Mode", null, (_, _) => Leave());
            _tray = new NotifyIcon
            {
                Text = "Ember: return to Game Mode (Ctrl+Alt+G)",
                Icon = Icon.ExtractAssociatedIcon(Environment.ProcessPath!) ?? SystemIcons.Application,
                ContextMenuStrip = menu,
            };
            _tray.DoubleClick += (_, _) => Leave();
            _tray.MouseClick += (_, e) => { if (e.Button == MouseButtons.Left) Leave(); };
        }
        _tray.Visible = true;   // shows once Explorer's tray exists (NotifyIcon re-adds itself when the taskbar appears)
    }

    private void HideTray() { if (_tray is not null) _tray.Visible = false; }

    /// <summary>A "Game Mode" shortcut on the desktop that brings Ember back.</summary>
    private static void EnsureShortcut()
    {
        try
        {
            var desktop = Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory);
            var lnk = Path.Combine(desktop, "Game Mode.lnk");
            if (File.Exists(lnk)) return;
            var shellType = Type.GetTypeFromProgID("WScript.Shell");
            if (shellType is null) return;
            dynamic shell = Activator.CreateInstance(shellType)!;
            dynamic sc = shell.CreateShortcut(lnk);
            sc.TargetPath = Environment.ProcessPath;
            sc.Arguments = "--game-mode";
            sc.Description = "Return to Ember's Game Mode";
            sc.IconLocation = Environment.ProcessPath + ",0";
            sc.Save();
        }
        catch (Exception ex) { SystemBridge.Log("desktop mode shortcut: " + ex.Message); }
    }

    public void Dispose()
    {
        _stop = true;
        if (_tray is not null) { _tray.Visible = false; _tray.Dispose(); }
        _signal?.Dispose();
    }

    private const int WM_USER = 0x0400;
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr FindWindow(string? cls, string? title);
    [DllImport("user32.dll")] private static extern bool PostMessage(IntPtr hWnd, int msg, IntPtr wParam, IntPtr lParam);
}
