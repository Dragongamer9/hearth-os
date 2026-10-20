namespace Nimbus.Host.Mantel;

/// <summary>
/// The notification history for the clock's panel (DeskState.history + ClockPanel.qml on Hearth OS), for this
/// session. Windows only lets packaged (MSIX) apps read other apps' notifications (UserNotificationListener), and
/// Ember is a plain program, so what's kept here is what reaches Ember: its own messages (game installs, updates,
/// play-time warnings) and, in full Mantel where Ember hosts the tray, the tray icons' balloon notifications.
/// Do Not Disturb (MantelState.Dnd) keeps them from popping up; they still go into the history.
/// </summary>
internal static class Notifications
{
    public sealed record Note(int Key, string App, string Summary, string Body, string? Icon, long Time);

    private static readonly List<Note> History = new();
    private static int _seq;
    public static int Unseen { get; private set; }
    /// <summary>The history changed (raised on the caller's thread).</summary>
    public static event Action? Changed;

    public static List<Note> All { get { lock (History) return History.ToList(); } }

    /// <summary>Adds one (newest first, 100 at most). Returns false for an empty one.</summary>
    public static bool Add(string app, string summary, string body, string? icon = null)
    {
        summary = (summary ?? "").Trim(); body = (body ?? "").Trim();
        if (summary.Length == 0 && body.Length == 0) return false;
        lock (History)
        {
            // The same message again within a few seconds (a page asking twice): once is enough.
            var last = History.FirstOrDefault();
            long now = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            if (last is not null && last.Summary == summary && last.Body == body && now - last.Time < 5000) return false;
            History.Insert(0, new Note(++_seq, app ?? "", summary, body, icon, now));
            if (History.Count > 100) History.RemoveRange(100, History.Count - 100);
            Unseen++;
        }
        Changed?.Invoke();
        return true;
    }

    public static void Forget(int key) { lock (History) History.RemoveAll(n => n.Key == key); Changed?.Invoke(); }
    public static void Clear() { lock (History) { History.Clear(); Unseen = 0; } Changed?.Invoke(); }
    public static void Seen() { if (Unseen == 0) return; Unseen = 0; Changed?.Invoke(); }
}
