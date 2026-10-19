using System.Runtime.InteropServices;

namespace Nimbus.Host.Mantel;

/// <summary>
/// Types what Hearth's on-screen keyboard (mantel.html?part=keyboard) asks for, like hearth-pad does on Hearth OS:
/// characters go in as Unicode (so they come out right whatever keyboard layout Windows uses), special keys
/// (Enter, Backspace, Tab, Esc, arrows) and anything with Ctrl or Alt as key presses on the US layout.
/// </summary>
internal static class KeyTyper
{
    private const byte VK_SHIFT = 0x10, VK_CONTROL = 0x11, VK_MENU = 0x12;
    private static readonly Dictionary<string, byte> Special = new()
    {
        ["space"] = 0x20, ["bksp"] = 0x08, ["tab"] = 0x09, ["enter"] = 0x0D, ["esc"] = 0x1B,
        ["left"] = 0x25, ["up"] = 0x26, ["right"] = 0x27, ["down"] = 0x28,
    };
    private static readonly Dictionary<char, byte> Oem = new()
    {
        ['`'] = 0xC0, ['-'] = 0xBD, ['='] = 0xBB, ['['] = 0xDB, [']'] = 0xDD, ['\\'] = 0xDC,
        [';'] = 0xBA, ['\''] = 0xDE, [','] = 0xBC, ['.'] = 0xBE, ['/'] = 0xBF,
    };

    /// <summary>key: the key's name on the keyboard (its unshifted character, or space/bksp/tab/enter/esc/arrows);
    /// ch: the character it types now ("" for special keys); shift/ctrl/alt: the one-shot modifiers.</summary>
    public static void Type(string key, string ch, bool shift, bool ctrl, bool alt)
    {
        if (Special.TryGetValue(key, out var vk)) { Press(vk, shift, ctrl, alt); return; }
        if (string.IsNullOrEmpty(ch)) return;
        if (!ctrl && !alt) { Unicode(ch); return; }
        // Ctrl / Alt shortcuts (Ctrl+C, Alt+F4…): the key itself, with Shift if the character needs it.
        char k = key.Length == 1 ? key[0] : ch[0];
        bool needShift = ch.Length == 1 && (char.IsUpper(ch[0]) || (ch[0] != k && !char.IsLetter(ch[0])));
        if (k is >= 'a' and <= 'z') Press((byte)char.ToUpperInvariant(k), needShift, ctrl, alt);
        else if (k is >= '0' and <= '9') Press((byte)k, needShift, ctrl, alt);
        else if (Oem.TryGetValue(k, out var o)) Press(o, needShift, ctrl, alt);
        else Unicode(ch);
    }

    private static void Press(byte vk, bool shift, bool ctrl, bool alt)
    {
        var list = new List<INPUT>();
        if (ctrl) list.Add(Key(VK_CONTROL, false));
        if (alt) list.Add(Key(VK_MENU, false));
        if (shift) list.Add(Key(VK_SHIFT, false));
        list.Add(Key(vk, false));
        list.Add(Key(vk, true));
        if (shift) list.Add(Key(VK_SHIFT, true));
        if (alt) list.Add(Key(VK_MENU, true));
        if (ctrl) list.Add(Key(VK_CONTROL, true));
        Send(list);
    }

    private static void Unicode(string text)
    {
        var list = new List<INPUT>();
        foreach (var c in text)
        {
            list.Add(new INPUT { type = INPUT_KEYBOARD, U = new InputUnion { ki = new KEYBDINPUT { wScan = c, dwFlags = KEYEVENTF_UNICODE } } });
            list.Add(new INPUT { type = INPUT_KEYBOARD, U = new InputUnion { ki = new KEYBDINPUT { wScan = c, dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP } } });
        }
        Send(list);
    }

    private static INPUT Key(byte vk, bool up)
    {
        uint flags = up ? KEYEVENTF_KEYUP : 0;
        if (vk is >= 0x21 and <= 0x28) flags |= KEYEVENTF_EXTENDEDKEY;   // the arrow keys (and Page Up/Down, Home, End)
        return new INPUT { type = INPUT_KEYBOARD, U = new InputUnion { ki = new KEYBDINPUT { wVk = vk, dwFlags = flags } } };
    }

    private static void Send(List<INPUT> inputs)
    {
        if (inputs.Count == 0) return;
        var arr = inputs.ToArray();
        if (SendInput((uint)arr.Length, arr, Marshal.SizeOf<INPUT>()) != arr.Length)
            SystemBridge.Log("keyboard: Windows didn't take every key (an admin window in front?)");
    }

    private const uint INPUT_KEYBOARD = 1, KEYEVENTF_EXTENDEDKEY = 0x1, KEYEVENTF_KEYUP = 0x2, KEYEVENTF_UNICODE = 0x4;

    [StructLayout(LayoutKind.Sequential)]
    private struct INPUT { public uint type; public InputUnion U; }

    [StructLayout(LayoutKind.Explicit)]
    private struct InputUnion
    {
        [FieldOffset(0)] public MOUSEINPUT mi;
        [FieldOffset(0)] public KEYBDINPUT ki;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MOUSEINPUT { public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr dwExtraInfo; }

    [StructLayout(LayoutKind.Sequential)]
    private struct KEYBDINPUT { public ushort wVk, wScan; public uint dwFlags, time; public IntPtr dwExtraInfo; }

    [DllImport("user32.dll", SetLastError = true)] private static extern uint SendInput(uint count, INPUT[] inputs, int size);
}
