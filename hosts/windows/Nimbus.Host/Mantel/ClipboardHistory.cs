using System.Drawing.Imaging;
using Microsoft.Win32;
using Windows.ApplicationModel.DataTransfer;
using WinClipboard = Windows.ApplicationModel.DataTransfer.Clipboard;

namespace Nimbus.Host.Mantel;

/// <summary>
/// Windows' clipboard history for Hearth's picker (Clipboard.qml on Hearth OS; mantel.html?part=clip here):
/// Windows keeps the history itself (Windows+V is its own picker and stays so); Ember reads it through
/// Windows.ApplicationModel.DataTransfer.Clipboard (GetHistoryItemsAsync, open to desktop apps since Windows 10
/// 1809), pastes an item by making it the clipboard's content and typing Ctrl+V into the window you were in,
/// forgets one, or clears everything. Settings › Clipboard turns the history on or off (Windows' own switch).
/// Call from the UI thread (WinRT's clipboard wants an STA thread).
/// </summary>
internal static class ClipboardHistory
{
    private static readonly Dictionary<string, ClipboardHistoryItem> Items = new();
    private const string Key = @"Software\Microsoft\Clipboard";

    /// <summary>Windows' "Clipboard history" switch (Settings › System › Clipboard).</summary>
    public static bool Enabled
    {
        get
        {
            try { return WinClipboard.IsHistoryEnabled(); }
            catch
            {
                try { using var k = Registry.CurrentUser.OpenSubKey(Key); return k?.GetValue("EnableClipboardHistory") is int v && v != 0; }
                catch { return false; }
            }
        }
    }

    /// <summary>Whether a policy (your organization) turned clipboard history off for everyone.</summary>
    private static bool BlockedByPolicy
    {
        get
        {
            try { using var k = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\Policies\Microsoft\Windows\System"); return k?.GetValue("AllowClipboardHistory") is int v && v == 0; }
            catch { return false; }
        }
    }

    public static object Status() => new { available = !BlockedByPolicy, enabled = Enabled, policy = BlockedByPolicy };

    public static object SetEnabled(bool on)
    {
        if (BlockedByPolicy) throw new InvalidOperationException("Your organization turned clipboard history off on this PC");
        using (var k = Registry.CurrentUser.CreateSubKey(Key)) k.SetValue("EnableClipboardHistory", on ? 1 : 0, RegistryValueKind.DWord);
        return Status();
    }

    /// <summary>What's in the history, newest first: [{id, kind: text|image|binary, text, image (data URL), dims, format}].</summary>
    public static async Task<object> ListAsync()
    {
        if (BlockedByPolicy) return new { available = false, enabled = false, items = Array.Empty<object>() };
        ClipboardHistoryItemsResult r;
        try { r = await WinClipboard.GetHistoryItemsAsync().AsTask(); }
        catch (Exception ex) { SystemBridge.Log("clipboard: " + ex.Message); return new { available = false, enabled = Enabled, items = Array.Empty<object>() }; }
        if (r.Status != ClipboardHistoryItemsResultStatus.Success)
            return new { available = true, enabled = r.Status != ClipboardHistoryItemsResultStatus.ClipboardHistoryDisabled, denied = r.Status == ClipboardHistoryItemsResultStatus.AccessDenied, items = Array.Empty<object>() };
        var outp = new List<object>();
        lock (Items) Items.Clear();
        foreach (var item in r.Items.Take(60))
        {
            lock (Items) Items[item.Id] = item;
            var c = item.Content;
            try
            {
                if (c.Contains(StandardDataFormats.Text))
                {
                    var text = await c.GetTextAsync().AsTask();
                    outp.Add(new { id = item.Id, kind = "text", text = text.Length > 2000 ? text[..2000] : text });
                }
                else if (c.Contains(StandardDataFormats.Bitmap))
                {
                    var (url, dims) = await ThumbAsync(c);
                    outp.Add(new { id = item.Id, kind = "image", image = url, dims });
                }
                else if (c.Contains(StandardDataFormats.StorageItems))
                {
                    var files = await c.GetStorageItemsAsync().AsTask();
                    outp.Add(new { id = item.Id, kind = "text", text = string.Join("  ", files.Select(f => f.Name)) });
                }
                else outp.Add(new { id = item.Id, kind = "binary", format = c.AvailableFormats.FirstOrDefault() ?? "data", size = "" });
            }
            catch (Exception ex) { SystemBridge.Log("clipboard item: " + ex.Message); }
        }
        return new { available = true, enabled = true, items = outp };
    }

    /// <summary>A picture's thumbnail (up to 480×200, as Clipboard.qml shows it) and its size in pixels.</summary>
    private static async Task<(string? Url, string Dims)> ThumbAsync(DataPackageView c)
    {
        var reference = await c.GetBitmapAsync().AsTask();
        using var s = await reference.OpenReadAsync().AsTask();
        if (s.Size == 0 || s.Size > 64L * 1024 * 1024) return (null, "");
        using var reader = new Windows.Storage.Streams.DataReader(s.GetInputStreamAt(0));
        await reader.LoadAsync((uint)s.Size).AsTask();
        var bytes = new byte[s.Size];
        reader.ReadBytes(bytes);
        using var ms = new MemoryStream(bytes);
        using var img = Image.FromStream(ms);
        double k = Math.Min(1, Math.Min(480.0 / img.Width, 200.0 / img.Height));
        using var small = new Bitmap(img, new Size(Math.Max(1, (int)(img.Width * k)), Math.Max(1, (int)(img.Height * k))));
        using var outp = new MemoryStream();
        small.Save(outp, ImageFormat.Png);
        return ("data:image/png;base64," + Convert.ToBase64String(outp.ToArray()), $"{img.Width} × {img.Height}");
    }

    /// <summary>Makes an item the clipboard's content (then the caller types Ctrl+V). False if it's gone.</summary>
    public static bool Select(string id)
    {
        ClipboardHistoryItem? item;
        lock (Items) Items.TryGetValue(id, out item);
        if (item is null) return false;
        return WinClipboard.SetHistoryItemAsContent(item) == SetHistoryItemAsContentStatus.Success;
    }

    public static void Delete(string id)
    {
        ClipboardHistoryItem? item;
        lock (Items) { Items.TryGetValue(id, out item); Items.Remove(id); }
        if (item is not null) WinClipboard.DeleteItemFromHistory(item);
    }

    public static void Clear() { lock (Items) Items.Clear(); WinClipboard.ClearHistory(); }
}
