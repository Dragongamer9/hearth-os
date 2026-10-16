using System.Diagnostics;
using System.Runtime.InteropServices;

namespace Nimbus.Host.Input;

/// <summary>
/// Reads Xbox-style controllers through XInput (exact Xbox button names, works whichever window is active).
///
/// Buttons
/// - Nimbus UI in front: presses go to the UI (more reliable than the browser's gamepad API, which reports
///   some controllers' buttons in a different order).
/// - Steam windows and Nimbus's own pop-ups (file picker): buttons become keys.
///     A = Enter   B = Esc   D-pad / left stick = move (Tab in Steam's web dialogs)   LB / RB = Shift+Tab / Tab
///
/// Controller mouse
/// - Right stick moves the pointer, A or RT = left click (hold to drag), LT = right click, left stick scrolls.
/// - On automatically per <see cref="MouseMode"/> ("off", "steam" = while a Steam window is in front,
///   "always" = everywhere), and View + Menu together toggles it on/off at any time.
///
/// Nothing is sent while a game is running: games get the controller to themselves.
/// </summary>
public sealed class ControllerBridge : IDisposable
{
    private readonly IntPtr _mainWindow;
    private readonly Func<bool> _gameRunning;
    private readonly Thread _thread;
    private volatile bool _stop;
    private readonly HashSet<uint> _steamPids = new();
    private DateTime _steamPidsAt;

    /// <summary>"off", "steam" or "always".</summary>
    public string MouseMode { get; set; } = "steam";
    /// <summary>Set by View + Menu: null = follow MouseMode, true/false = forced on/off until toggled again.</summary>
    private bool? _mouseOverride;
    public bool MouseActive { get; private set; }

    public ControllerBridge(IntPtr mainWindow, Func<bool> gameRunning, string mouseMode)
    {
        _mainWindow = mainWindow;
        _gameRunning = gameRunning;
        MouseMode = mouseMode;
        _thread = new Thread(Loop) { IsBackground = true, Name = "Nimbus controller bridge" };
        _thread.Start();
    }

    private enum Target { Game, Ui, Steam, Web, NimbusDialog, Other }   // Web = a store sign-in window

    /// <summary>Raised on the bridge thread when the Nimbus UI is in front: button name (a, b, x, y, lb, rb, start, select, up, down, left, right).</summary>
    public event Action<string>? UiButton;
    /// <summary>Raised when the number of connected XInput controllers changes.</summary>
    public event Action<int>? ControllersChanged;
    /// <summary>Raised when the controller mouse turns on or off (true = on). Also raised for the View + Menu toggle.</summary>
    public event Action<bool>? MouseChanged;

    private int _connected = -1;
    /// <summary>Connected XInput controllers right now.</summary>
    public int Connected => Math.Max(0, _connected);

    // ------------------------------------------------------------------ main loop

    private void Loop()
    {
        ushort prev = 0;
        var held = new Dictionary<ushort, (DateTime first, DateTime last)>();
        bool suppressCombo = false;
        bool leftDown = false, rightDown = false;
        double carryX = 0, carryY = 0, carryWheel = 0;

        while (!_stop)
        {
            Thread.Sleep(10);
            var pad = ReadPad(out int connected);
            if (connected != _connected) { _connected = connected; ControllersChanged?.Invoke(connected); }
            if (connected == 0) { prev = 0; held.Clear(); SetMouse(false); continue; }

            var target = CurrentTarget();
            if (target == Target.Game)
            {
                if (leftDown) { Mouse(MOUSEEVENTF_LEFTUP); leftDown = false; }
                if (rightDown) { Mouse(MOUSEEVENTF_RIGHTUP); rightDown = false; }
                prev = pad.Buttons; held.Clear(); SetMouse(false);
                continue;
            }

            // View + Menu together toggles the controller mouse.
            bool combo = (pad.Buttons & (START | BACK)) == (START | BACK);
            if (combo && !suppressCombo)
            {
                suppressCombo = true;
                _mouseOverride = !(MouseActive);
                MouseChanged?.Invoke(_mouseOverride.Value);
            }
            if (suppressCombo)
            {
                if ((pad.Buttons & (START | BACK)) == 0) suppressCombo = false;
                pad.Buttons &= unchecked((ushort)~(START | BACK));   // don't also act as Menu / View
                prev &= unchecked((ushort)~(START | BACK));
            }

            bool mouse = _mouseOverride ?? MouseMode switch
            {
                "always" => true,
                "steam" => target is Target.Steam or Target.Web,
                _ => false,
            };
            SetMouse(mouse);

            ushort buttons = pad.Buttons;
            if (mouse)
            {
                // Right stick → pointer. Curve gives fine control near the center and speed at the edge.
                carryX += Curve(pad.RX) * MaxSpeed;
                carryY += -Curve(pad.RY) * MaxSpeed;
                int dx = (int)carryX, dy = (int)carryY;
                carryX -= dx; carryY -= dy;
                if (dx != 0 || dy != 0) Mouse(MOUSEEVENTF_MOVE, dx, dy);

                // Left stick → scroll wheel.
                carryWheel += Curve(pad.LY) * 30;
                int wheel = (int)carryWheel;
                if (wheel != 0) { carryWheel -= wheel; Mouse(MOUSEEVENTF_WHEEL, data: wheel); }

                // A or RT = left button (hold to drag), LT = right button.
                bool left = (buttons & A) != 0 || pad.RT > TriggerPress;
                bool right = pad.LT > TriggerPress;
                if (left != leftDown) { Mouse(left ? MOUSEEVENTF_LEFTDOWN : MOUSEEVENTF_LEFTUP); leftDown = left; }
                if (right != rightDown) { Mouse(right ? MOUSEEVENTF_RIGHTDOWN : MOUSEEVENTF_RIGHTUP); rightDown = right; }
                buttons &= unchecked((ushort)~A);   // A is the mouse button now
            }
            else
            {
                if (leftDown) { Mouse(MOUSEEVENTF_LEFTUP); leftDown = false; }
                if (rightDown) { Mouse(MOUSEEVENTF_RIGHTUP); rightDown = false; }
                // Left stick doubles as the D-pad when it isn't scrolling.
                if (pad.LY > StickDeadzone) buttons |= UP;
                if (pad.LY < -StickDeadzone) buttons |= DOWN;
                if (pad.LX < -StickDeadzone) buttons |= LEFT;
                if (pad.LX > StickDeadzone) buttons |= RIGHT;
            }

            var now = DateTime.UtcNow;
            foreach (var b in AllButtons)
            {
                bool down = (buttons & b) != 0, was = (prev & b) != 0;
                if (down && !was) { held[b] = (now, now); Dispatch(b, target); }
                else if (down && was && IsDirection(b) && held.TryGetValue(b, out var h)
                         && now - h.first > TimeSpan.FromMilliseconds(380) && now - h.last > TimeSpan.FromMilliseconds(110))
                {
                    held[b] = (h.first, now);
                    Dispatch(b, target);
                }
                else if (!down) held.Remove(b);
            }
            prev = buttons;
        }
    }

    private void SetMouse(bool on)
    {
        if (MouseActive == on) return;
        MouseActive = on;
        if (_mouseOverride is null) MouseChanged?.Invoke(on);
    }

    /// <summary>Changing the setting clears a View + Menu override.</summary>
    public void SetMouseMode(string mode) { MouseMode = mode; _mouseOverride = null; }

    private const double MaxSpeed = 18;          // pixels per 10 ms at full tilt
    private const byte TriggerPress = 100;

    private static double Curve(short v)
    {
        double n = v / 32767.0;
        double a = Math.Abs(n);
        const double dead = 0.18;
        if (a < dead) return 0;
        double t = (a - dead) / (1 - dead);
        return Math.Sign(n) * t * t;
    }

    // ------------------------------------------------------------------ which window is in front

    private Target CurrentTarget()
    {
        if (_gameRunning()) return Target.Game;                          // games get the controller to themselves
        var fg = GetForegroundWindow();
        if (fg == IntPtr.Zero) return Target.Other;
        if (fg == _mainWindow) return Target.Ui;
        GetWindowThreadProcessId(fg, out var pid);
        if (pid == (uint)Environment.ProcessId) return Stores.SignInWindow.IsSignInWindow(fg) ? Target.Web : Target.NimbusDialog;
        if (IsSteam(pid)) return Target.Steam;
        return Target.Other;
    }

    private bool IsSteam(uint pid)
    {
        if (DateTime.UtcNow - _steamPidsAt > TimeSpan.FromSeconds(3))
        {
            _steamPids.Clear();
            foreach (var name in new[] { "steam", "steamwebhelper" })
                foreach (var p in Process.GetProcessesByName(name)) { _steamPids.Add((uint)p.Id); p.Dispose(); }
            _steamPidsAt = DateTime.UtcNow;
        }
        return _steamPids.Contains(pid);
    }

    // ------------------------------------------------------------------ button → UI / key

    private const ushort UP = 0x0001, DOWN = 0x0002, LEFT = 0x0004, RIGHT = 0x0008,
                         START = 0x0010, BACK = 0x0020, LB = 0x0100, RB = 0x0200,
                         A = 0x1000, B = 0x2000, X = 0x4000, Y = 0x8000;
    private static readonly ushort[] AllButtons = { UP, DOWN, LEFT, RIGHT, LB, RB, A, B, X, Y, START, BACK };
    private static bool IsDirection(ushort b) => b is UP or DOWN or LEFT or RIGHT;

    private void Dispatch(ushort b, Target target)
    {
        if (target == Target.Ui)
        {
            var name = b switch
            {
                A => "a", B => "b", X => "x", Y => "y", LB => "lb", RB => "rb", START => "start", BACK => "select",
                UP => "up", DOWN => "down", LEFT => "left", RIGHT => "right", _ => null,
            };
            if (name is not null) UiButton?.Invoke(name);
        }
        else if (target is Target.Steam or Target.Web or Target.NimbusDialog) Press(b, target);
        // Other apps: only the controller mouse (if on) acts; no keys are sent to them.
    }

    private const byte VK_RETURN = 0x0D, VK_ESCAPE = 0x1B, VK_TAB = 0x09, VK_SHIFT = 0x10,
                       VK_LEFT = 0x25, VK_UP = 0x26, VK_RIGHT = 0x27, VK_DOWN = 0x28;

    private static void Press(ushort button, Target target)
    {
        bool steam = target is Target.Steam or Target.Web;
        switch (button)
        {
            // Y: on-screen keyboard for typing into web pages (sign-in, Steam's login).
            case Y when steam: ShowKeyboard(); break;
            case A: Key(VK_RETURN); break;
            case B: Key(VK_ESCAPE); break;
            case START: Key(VK_RETURN); break;
            case RB: Key(VK_TAB); break;
            case LB: Key(VK_TAB, shift: true); break;
            // Steam's dialogs are web pages: move between buttons with Tab.
            case RIGHT or DOWN when steam: Key(VK_TAB); break;
            case LEFT or UP when steam: Key(VK_TAB, shift: true); break;
            case UP: Key(VK_UP); break;
            case DOWN: Key(VK_DOWN); break;
            case LEFT: Key(VK_LEFT); break;
            case RIGHT: Key(VK_RIGHT); break;
        }
    }

    private static void ShowKeyboard()
    {
        var t = new Thread(Stores.TouchKeyboard.Toggle) { IsBackground = true };
        t.SetApartmentState(ApartmentState.STA);   // the touch keyboard's COM interface wants STA
        t.Start();
    }

    private static void Key(byte vk, bool shift = false)
    {
        if (shift) keybd_event(VK_SHIFT, 0, 0, UIntPtr.Zero);
        keybd_event(vk, 0, 0, UIntPtr.Zero);
        keybd_event(vk, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
        if (shift) keybd_event(VK_SHIFT, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
    }

    private static void Mouse(uint flags, int dx = 0, int dy = 0, int data = 0) =>
        mouse_event(flags, dx, dy, data, UIntPtr.Zero);

    // ------------------------------------------------------------------ XInput

    [StructLayout(LayoutKind.Sequential)]
    private struct XINPUT_GAMEPAD { public ushort wButtons; public byte bLeftTrigger, bRightTrigger; public short sThumbLX, sThumbLY, sThumbRX, sThumbRY; }
    [StructLayout(LayoutKind.Sequential)]
    private struct XINPUT_STATE { public uint dwPacketNumber; public XINPUT_GAMEPAD Gamepad; }

    [DllImport("xinput1_4.dll")] private static extern int XInputGetState(int index, out XINPUT_STATE state);

    private struct Pad { public ushort Buttons; public byte LT, RT; public short LX, LY, RX, RY; }

    private static bool _xinputMissing;
    private const short StickDeadzone = 16000;

    /// <summary>All connected controllers merged: buttons combined, the stick/trigger pushed furthest wins.</summary>
    private static Pad ReadPad(out int connected)
    {
        connected = 0;
        var pad = new Pad();
        if (_xinputMissing) return pad;
        try
        {
            for (int i = 0; i < 4; i++)
            {
                if (XInputGetState(i, out var s) != 0) continue;   // not connected
                connected++;
                var g = s.Gamepad;
                pad.Buttons |= g.wButtons;
                pad.LT = Math.Max(pad.LT, g.bLeftTrigger);
                pad.RT = Math.Max(pad.RT, g.bRightTrigger);
                if (Math.Abs((int)g.sThumbLX) > Math.Abs((int)pad.LX)) pad.LX = g.sThumbLX;
                if (Math.Abs((int)g.sThumbLY) > Math.Abs((int)pad.LY)) pad.LY = g.sThumbLY;
                if (Math.Abs((int)g.sThumbRX) > Math.Abs((int)pad.RX)) pad.RX = g.sThumbRX;
                if (Math.Abs((int)g.sThumbRY) > Math.Abs((int)pad.RY)) pad.RY = g.sThumbRY;
            }
        }
        catch (DllNotFoundException) { _xinputMissing = true; }
        catch (EntryPointNotFoundException) { _xinputMissing = true; }
        return pad;
    }

    // ------------------------------------------------------------------ Win32

    private const uint KEYEVENTF_KEYUP = 0x0002;
    private const uint MOUSEEVENTF_MOVE = 0x0001, MOUSEEVENTF_LEFTDOWN = 0x0002, MOUSEEVENTF_LEFTUP = 0x0004,
                       MOUSEEVENTF_RIGHTDOWN = 0x0008, MOUSEEVENTF_RIGHTUP = 0x0010, MOUSEEVENTF_WHEEL = 0x0800;
    [DllImport("user32.dll")] private static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] private static extern void mouse_event(uint flags, int dx, int dy, int data, UIntPtr extra);
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);

    public void Dispose() => _stop = true;
}
