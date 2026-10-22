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
///
/// In-game overlay
/// - The Xbox (Guide) button opens/closes the Nimbus overlay over whatever is in front (in the Nimbus UI
///   it opens Quick Access instead). Optionally, holding View for a second does the same.
/// - While the overlay is in front, buttons go to the overlay page.
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
    /// <summary>Hearth Input (controller layouts for games without controller support), set by SystemBridge.</summary>
    public HearthInput? Mapper { get; set; }
    /// <summary>Set by View + Menu: null = follow MouseMode, true/false = forced on/off until toggled again.</summary>
    private bool? _mouseOverride;
    public bool MouseActive { get; private set; }

    public ControllerBridge(IntPtr mainWindow, Func<bool> gameRunning, string mouseMode)
    {
        _mainWindow = mainWindow;
        _gameRunning = gameRunning;
        MouseMode = mouseMode;
        _thread = new Thread(SafeLoop) { IsBackground = true, Name = "Nimbus controller bridge" };
        _thread.Start();
    }

    private enum Target { Game, Ui, Overlay, Steam, Web, NimbusDialog, Other }   // Web = a store sign-in window

    /// <summary>The overlay window's handle (IntPtr.Zero when there is none).</summary>
    public Func<IntPtr> OverlayWindow { get; set; } = () => IntPtr.Zero;
    /// <summary>Holding View for a second toggles the overlay (off by default: some games use a held View).</summary>
    public bool HoldViewOpensOverlay { get; set; }
    /// <summary>In Desktop Mode the controller is a mouse everywhere except Ember's own windows and games.</summary>
    public Func<bool> DesktopMode { get; set; } = () => false;
    /// <summary>Menu (☰) on the desktop: Mantel's launcher.</summary>
    public event Action? DesktopMenu;
    /// <summary>Mantel (Desktop Mode): the Xbox button opens the Hearth menu instead of the overlay.</summary>
    public event Action? DesktopGuide;
    /// <summary>Mantel: a quick tap of View shows the controls.</summary>
    public event Action? DesktopHelp;
    /// <summary>Mantel decides whether it's showing (so the Xbox button and View act like on Hearth OS).</summary>
    public Func<bool> MantelActive { get; set; } = () => false;
    /// <summary>A Mantel panel (launcher, Quick Access, power menu, Hearth menu) is in front: the controller
    /// moves through it (D-pad, A, B) instead of being a mouse.</summary>
    public Func<bool> MantelPanelInFront { get; set; } = () => false;
    /// <summary>Full Mantel's desktop window (its icons): the controller treats it like any app's window.</summary>
    public Func<IntPtr, bool> MantelDesktopWindow { get; set; } = _ => false;
    /// <summary>Hearth's on-screen keyboard is up: the D-pad and left stick pick keys, A types, X is backspace, LB shift,
    /// Menu Enter, B or Y close (like hearth-pad on Hearth OS). RT still clicks, the right stick still moves the pointer.</summary>
    public Func<bool> KeyboardOpen { get; set; } = () => false;
    /// <summary>Raised on the bridge thread with a button name (up, down, left, right, a, x, lb, start, b, y) while the keyboard is up.</summary>
    public event Action<string>? KeyboardButton;
    /// <summary>Y in Mantel: Hearth's on-screen keyboard (raised on the bridge thread).</summary>
    public event Action? DesktopKeyboard;
    /// <summary>A short message for the screen ("Precise pointer on"), raised on the bridge thread.</summary>
    public event Action<string>? Toast;
    /// <summary>Hearth Settings › Controller › Pointer speed: 0.4 – 2 (1 = normal).</summary>
    public double PointerSpeed { get; set; } = 1.0;
    private bool _precise;
    private volatile bool _kbNow;
    private bool _viewHeld, _viewUsed, _altHeld;
    private DateTime _viewAt, _altUntil;
    /// <summary>Raised on the bridge thread when the overlay should open or close.</summary>
    public event Action? OverlayToggle;
    /// <summary>Raised on the bridge thread when the overlay is in front: button name, like <see cref="UiButton"/>.</summary>
    public event Action<string>? OverlayButton;

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

    /// <summary>An error in the loop (a window that went away, a handler on a window that's closing) would end the
    /// whole process from this thread; it's logged and the controller is read again instead.</summary>
    private void SafeLoop()
    {
        while (!_stop)
        {
            try { Loop(); }
            catch (Exception ex)
            {
                if (_stop) return;
                SystemBridge.Log("controller bridge: " + ex);
                try { Mapper?.Release(); } catch { }
                Thread.Sleep(500);
            }
        }
    }

    private void Loop()
    {
        ushort prev = 0;
        var held = new Dictionary<ushort, (DateTime first, DateTime last)>();
        bool suppressCombo = false;
        bool leftDown = false, rightDown = false, middleDown = false;
        double carryX = 0, carryY = 0, carryWheel = 0;
        bool guidePrev = false, viewFired = false;
        DateTime viewSince = DateTime.MaxValue;

        while (!_stop)
        {
            Thread.Sleep(10);
            var pad = ReadPad(out int connected);
            if (connected != _connected) { _connected = connected; ControllersChanged?.Invoke(connected); }
            if (connected == 0) { prev = 0; held.Clear(); SetMouse(false); Mapper?.Release(); continue; }

            var target = CurrentTarget();

            // Xbox button: Quick Access in the Nimbus UI, the overlay everywhere else.
            bool guide = (pad.Buttons & GUIDE) != 0;
            bool guideEdge = guide && !guidePrev;
            if (guide && !guidePrev)
            {
                if (target == Target.Ui) UiButton?.Invoke("guide");
                else if (DesktopMode() && MantelActive() && target != Target.Game && target != Target.Overlay) DesktopGuide?.Invoke();
                else OverlayToggle?.Invoke();
            }
            guidePrev = guide;
            pad.Buttons &= unchecked((ushort)~GUIDE);

            // Optional: hold View (alone) for a second.
            if (HoldViewOpensOverlay && pad.Buttons == BACK && target is not (Target.Ui or Target.Overlay))
            {
                if (viewSince == DateTime.MaxValue) viewSince = DateTime.UtcNow;
                if (!viewFired && DateTime.UtcNow - viewSince > TimeSpan.FromSeconds(1)) { viewFired = true; OverlayToggle?.Invoke(); }
            }
            else { viewSince = DateTime.MaxValue; viewFired = false; }

            // Hearth Input: while the running game (with a controller layout) is in front, or a layout is being
            // tried, the controller is a keyboard and mouse and nothing else here acts on it.
            if (Mapper is { } mapper)
            {
                bool mapped;
                try
                {
                    mapped = mapper.Feed(pad.Buttons, pad.LT, pad.RT, pad.LX, pad.LY, pad.RX, pad.RY,
                        gameFront: target == Target.Game && mapper.IsGameWindow(GetForegroundWindow()),
                        emberFront: target == Target.Ui, overlayFront: target == Target.Overlay, guidePressed: guideEdge);
                }
                catch (Exception ex) { mapped = false; SystemBridge.Log("input: " + ex.Message); }
                if (mapped)
                {
                    if (leftDown) { Mouse(MOUSEEVENTF_LEFTUP); leftDown = false; }
                    if (rightDown) { Mouse(MOUSEEVENTF_RIGHTUP); rightDown = false; }
                    if (middleDown) { Mouse(MOUSEEVENTF_MIDDLEUP); middleDown = false; }
                    prev = pad.Buttons; held.Clear(); SetMouse(false);
                    continue;
                }
            }

            if (target == Target.Game)
            {
                if (leftDown) { Mouse(MOUSEEVENTF_LEFTUP); leftDown = false; }
                if (rightDown) { Mouse(MOUSEEVENTF_RIGHTUP); rightDown = false; }
                if (middleDown) { Mouse(MOUSEEVENTF_MIDDLEUP); middleDown = false; }
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

            bool panel = MantelPanelInFront();
            bool mouse = panel ? false : _mouseOverride ?? (DesktopMode() && target is not (Target.Ui or Target.Overlay or Target.Game) ? true : MouseMode switch
            {
                "always" => true,
                "steam" => target is Target.Steam or Target.Web,
                _ => false,
            });
            SetMouse(mouse);

            ushort buttons = pad.Buttons;
            // Mantel, like hearth-pad: X / LT right-click, L3 middle-click, R3 precise pointer; the keyboard takes the D-pad.
            bool mantelDesk = DesktopMode() && MantelActive() && target is not (Target.Ui or Target.Overlay);
            bool kb = mantelDesk && KeyboardOpen();
            _kbNow = kb;
            if (mouse)
            {
                if (mantelDesk && !kb && (buttons & RTHUMB) != 0 && (prev & RTHUMB) == 0)
                {
                    _precise = !_precise;
                    Toast?.Invoke("Precise pointer " + (_precise ? "on" : "off"));
                }
                // Right stick → pointer. Curve gives fine control near the center and speed at the edge.
                double speed = MaxSpeed * Math.Clamp(PointerSpeed, 0.2, 3.0) * (_precise && mantelDesk ? 0.3 : 1.0);
                carryX += Curve(pad.RX) * speed;
                carryY += -Curve(pad.RY) * speed;
                int dx = (int)carryX, dy = (int)carryY;
                carryX -= dx; carryY -= dy;
                if (dx != 0 || dy != 0) Mouse(MOUSEEVENTF_MOVE, dx, dy);

                if (kb)
                {
                    // The keyboard is up: the left stick picks keys like the D-pad (below), A types.
                    if (pad.LY > StickDeadzone) buttons |= UP;
                    if (pad.LY < -StickDeadzone) buttons |= DOWN;
                    if (pad.LX < -StickDeadzone) buttons |= LEFT;
                    if (pad.LX > StickDeadzone) buttons |= RIGHT;
                }
                else
                {
                    // Left stick → scroll wheel.
                    carryWheel += Curve(pad.LY) * 30;
                    int wheel = (int)carryWheel;
                    if (wheel != 0) { carryWheel -= wheel; Mouse(MOUSEEVENTF_WHEEL, data: wheel); }
                }

                // A or RT = left button (hold to drag), LT (and X in Mantel) = right button, L3 in Mantel = middle.
                bool left = (!kb && (buttons & A) != 0) || pad.RT > TriggerPress;
                bool right = !kb && (pad.LT > TriggerPress || (mantelDesk && (buttons & X) != 0));
                bool middle = mantelDesk && !kb && (buttons & LTHUMB) != 0;
                if (left != leftDown) { Mouse(left ? MOUSEEVENTF_LEFTDOWN : MOUSEEVENTF_LEFTUP); leftDown = left; }
                if (right != rightDown) { Mouse(right ? MOUSEEVENTF_RIGHTDOWN : MOUSEEVENTF_RIGHTUP); rightDown = right; }
                if (middle != middleDown) { Mouse(middle ? MOUSEEVENTF_MIDDLEDOWN : MOUSEEVENTF_MIDDLEUP); middleDown = middle; }
                if (!kb) buttons &= unchecked((ushort)~A);   // A is the mouse button now
                if (mantelDesk && !kb) buttons &= unchecked((ushort)~X);   // and X the right one
            }
            else
            {
                if (leftDown) { Mouse(MOUSEEVENTF_LEFTUP); leftDown = false; }
                if (rightDown) { Mouse(MOUSEEVENTF_RIGHTUP); rightDown = false; }
                if (middleDown) { Mouse(MOUSEEVENTF_MIDDLEUP); middleDown = false; }
                // Left stick doubles as the D-pad when it isn't scrolling.
                if (pad.LY > StickDeadzone) buttons |= UP;
                if (pad.LY < -StickDeadzone) buttons |= DOWN;
                if (pad.LX < -StickDeadzone) buttons |= LEFT;
                if (pad.LX > StickDeadzone) buttons |= RIGHT;
            }

            var now = DateTime.UtcNow;
            // Mantel: LB/RB switch windows (Alt held a moment, like Hearth), View is a modifier and a tap shows the controls.
            if (_altHeld && now > _altUntil) { keybd_event(VK_MENU, 0, KEYEVENTF_KEYUP, UIntPtr.Zero); _altHeld = false; }
            bool viewNow = (buttons & BACK) != 0;
            if (viewNow && !_viewHeld) { _viewAt = now; _viewUsed = false; }
            if (!viewNow && _viewHeld && !_viewUsed && now - _viewAt < TimeSpan.FromMilliseconds(400) && DesktopMode() && MantelActive() && target == Target.Other) DesktopHelp?.Invoke();
            _viewHeld = viewNow;
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

    /// <summary>Same as pressing View + Menu: turns the controller mouse on or off until toggled again.</summary>
    public bool ToggleMouse()
    {
        _mouseOverride = !MouseActive;
        MouseChanged?.Invoke(_mouseOverride.Value);
        return _mouseOverride.Value;
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
        var fg = GetForegroundWindow();
        if (fg != IntPtr.Zero && fg == OverlayWindow()) return Target.Overlay;
        if (fg != IntPtr.Zero && fg == _mainWindow) return Target.Ui;
        if (_gameRunning()) return Target.Game;                          // games get the controller to themselves
        if (fg == IntPtr.Zero) return Target.Other;
        GetWindowThreadProcessId(fg, out var pid);
        if (pid == (uint)Environment.ProcessId) return Stores.SignInWindow.IsSignInWindow(fg) ? Target.Web : MantelDesktopWindow(fg) ? Target.Other : Target.NimbusDialog;
        if (IsSteam(pid)) return Target.Steam;
        // Hearth Browser in Game Mode: driven like a sign-in page (pointer, A Enter, Y the touch keyboard...).
        if (!DesktopMode() && Services.HearthBrowser.IsBrowserProcess(pid)) return Target.Web;
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
                         GUIDE = 0x0400, A = 0x1000, B = 0x2000, X = 0x4000, Y = 0x8000,
                         LTHUMB = 0x0040, RTHUMB = 0x0080;
    private static readonly ushort[] AllButtons = { UP, DOWN, LEFT, RIGHT, LB, RB, A, B, X, Y, START, BACK };
    private static bool IsDirection(ushort b) => b is UP or DOWN or LEFT or RIGHT;

    private void Dispatch(ushort b, Target target)
    {
        // Hearth's on-screen keyboard is up: buttons pick and type keys (KeyboardButton), whatever window is in front.
        if (_kbNow && target is not (Target.Ui or Target.Overlay or Target.Game))
        {
            var key = b switch
            {
                UP => "up", DOWN => "down", LEFT => "left", RIGHT => "right", A => "a", X => "x", LB => "lb", START => "start", B => "b", Y => "y", _ => null,
            };
            if (key is not null) KeyboardButton?.Invoke(key);
            return;
        }
        if (target is Target.Ui or Target.Overlay)
        {
            var name = b switch
            {
                A => "a", B => "b", X => "x", Y => "y", LB => "lb", RB => "rb", START => "start", BACK => "select",
                UP => "up", DOWN => "down", LEFT => "left", RIGHT => "right", _ => null,
            };
            if (name is not null) { if (target == Target.Overlay) OverlayButton?.Invoke(name); else UiButton?.Invoke(name); }
        }
        // Menu again while Mantel's launcher (or another panel) is up closes it, like on Hearth OS.
        else if (b == START && target == Target.NimbusDialog && DesktopMode() && MantelActive() && MantelPanelInFront()) DesktopMenu?.Invoke();
        else if (target is Target.Steam or Target.Web or Target.NimbusDialog) Press(b, target);
        // Desktop Mode, like Hearth's Mantel: Y opens the on-screen keyboard, B is Esc, LB/RB switch windows.
        else if (DesktopMode() && target == Target.Other)
        {
            if (_viewHeld && b != BACK)
            {
                _viewUsed = true;
                switch (b)
                {
                    case B: KeyCombo(VK_MENU, VK_F4); break;                        // View + B: close the window
                    case UP: KeyCombo(VK_LWIN, VK_UP); break;                       // View + D-pad: snap it
                    case DOWN: KeyCombo(VK_LWIN, VK_DOWN); break;
                    case LEFT: KeyCombo(VK_LWIN, VK_LEFT); break;
                    case RIGHT: KeyCombo(VK_LWIN, VK_RIGHT); break;
                }
                return;
            }
            switch (b)
            {
                case Y: if (MantelActive() && DesktopKeyboard is not null) DesktopKeyboard.Invoke(); else ShowKeyboard(); break;
                case B: Key(VK_ESCAPE); break;
                case START: DesktopMenu?.Invoke(); break;   // Mantel's launcher
                case LB or RB:
                    if (!_altHeld) { keybd_event(VK_MENU, 0, 0, UIntPtr.Zero); _altHeld = true; }
                    _altUntil = DateTime.UtcNow.AddMilliseconds(900);
                    Key(VK_TAB, shift: b == LB);
                    break;
                case UP: Key(VK_UP); break;
                case DOWN: Key(VK_DOWN); break;
                case LEFT: Key(VK_LEFT); break;
                case RIGHT: Key(VK_RIGHT); break;
            }
        }
        // Other apps: only the controller mouse (if on) acts; no keys are sent to them.
    }

    private const byte VK_MENU = 0x12, VK_F4 = 0x73, VK_LWIN = 0x5B, VK_F23 = 0x86, VK_F24 = 0x87;
    private const byte VK_RETURN = 0x0D, VK_ESCAPE = 0x1B, VK_TAB = 0x09, VK_SHIFT = 0x10,
                       VK_LEFT = 0x25, VK_UP = 0x26, VK_RIGHT = 0x27, VK_DOWN = 0x28;

    private static void Press(ushort button, Target target)
    {
        bool steam = target is Target.Steam or Target.Web;
        switch (button)
        {
            // Y: on-screen keyboard for typing into web pages (sign-in, Steam's login).
            case Y when steam: ShowKeyboard(); break;
            // Ember's own windows (Hearth Task Manager, Hearth Files): X = actions, Y = search / rename. F23 and F24
            // are keys no keyboard has, so the pages know they came from the controller (and open the keyboard).
            case X when target == Target.NimbusDialog: Key(VK_F23); break;
            case Y when target == Target.NimbusDialog: Key(VK_F24); break;
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

    private static void KeyCombo(byte mod, byte vk)
    {
        keybd_event(mod, 0, 0, UIntPtr.Zero);
        keybd_event(vk, 0, 0, UIntPtr.Zero);
        keybd_event(vk, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
        keybd_event(mod, 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
    }

    private static void Mouse(uint flags, int dx = 0, int dy = 0, int data = 0) =>
        mouse_event(flags, dx, dy, data, UIntPtr.Zero);

    // ------------------------------------------------------------------ XInput

    [StructLayout(LayoutKind.Sequential)]
    private struct XINPUT_GAMEPAD { public ushort wButtons; public byte bLeftTrigger, bRightTrigger; public short sThumbLX, sThumbLY, sThumbRX, sThumbRY; }
    [StructLayout(LayoutKind.Sequential)]
    private struct XINPUT_STATE { public uint dwPacketNumber; public XINPUT_GAMEPAD Gamepad; }

    [DllImport("xinput1_4.dll")] private static extern int XInputGetState(int index, out XINPUT_STATE state);
    // Undocumented but long-standing (ordinal 100): same as XInputGetState, plus the Guide button (0x0400).
    [DllImport("xinput1_4.dll", EntryPoint = "#100")] private static extern int XInputGetStateEx(int index, out XINPUT_STATE state);
    private static bool _noStateEx;

    private static int GetState(int i, out XINPUT_STATE s)
    {
        if (!_noStateEx)
        {
            try { return XInputGetStateEx(i, out s); }
            catch (EntryPointNotFoundException) { _noStateEx = true; }
        }
        return XInputGetState(i, out s);
    }

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
                if (GetState(i, out var s) != 0) continue;   // not connected
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
                       MOUSEEVENTF_RIGHTDOWN = 0x0008, MOUSEEVENTF_RIGHTUP = 0x0010, MOUSEEVENTF_MIDDLEDOWN = 0x0020, MOUSEEVENTF_MIDDLEUP = 0x0040,
                       MOUSEEVENTF_WHEEL = 0x0800;
    [DllImport("user32.dll")] private static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] private static extern void mouse_event(uint flags, int dx, int dy, int data, UIntPtr extra);
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);

    public void Dispose() { _stop = true; Mapper?.Release(); }
}
