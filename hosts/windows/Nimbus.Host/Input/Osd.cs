namespace Nimbus.Host.Input;

/// <summary>
/// A small "toast" that floats over every window (including Steam's) without taking focus,
/// used for things like "Controller mouse on".
/// </summary>
public sealed class Osd : Form
{
    private static Osd? _current;
    /// <summary>Set while Mantel runs: messages look like Hearth's (Help.qml's toast: a glass pill at the top of the screen).</summary>
    public static bool HearthStyle { get; set; }
    private readonly System.Windows.Forms.Timer _timer = new() { Interval = 1800 };

    private Osd(string text)
    {
        FormBorderStyle = FormBorderStyle.None;
        ShowInTaskbar = false;
        TopMost = true;
        StartPosition = FormStartPosition.Manual;
        BackColor = Color.FromArgb(29, 37, 49);
        ForeColor = Color.FromArgb(233, 238, 244);
        Font = new Font("Segoe UI", 13f, FontStyle.Bold);
        Padding = new Padding(18, 10, 18, 10);
        var area = Screen.PrimaryScreen?.WorkingArea ?? new Rectangle(0, 0, 1280, 800);
        if (HearthStyle)
        {
            // Hearth's colors (Theme.glass, Theme.text), 14 px text, 44 tall, fully round, 24 below the top.
            BackColor = Color.FromArgb(28, 21, 18);
            ForeColor = Color.FromArgb(244, 236, 227);
            Font = new Font("Segoe UI", 10.5f, FontStyle.Regular);
            var l = new Label { Text = text, AutoSize = true, BackColor = Color.Transparent };
            Controls.Add(l);
            int h = (int)Math.Round(44 * DeviceDpi / 96.0), pad = (int)Math.Round(18 * DeviceDpi / 96.0);
            Size = new Size(l.PreferredWidth + 2 * pad, h);
            l.Location = new Point(pad, (h - l.PreferredHeight) / 2);
            var scr = Screen.PrimaryScreen?.Bounds ?? area;
            Location = new Point(scr.Left + (scr.Width - Width) / 2, scr.Top + (int)Math.Round(24 * DeviceDpi / 96.0));
            using var path = new System.Drawing.Drawing2D.GraphicsPath();
            path.AddArc(0, 0, h, h, 90, 180);
            path.AddArc(Width - h, 0, h, h, 270, 180);
            path.CloseFigure();
            Region = new Region(path);
        }
        else
        {
            var label = new Label { Text = text, AutoSize = true, Location = new Point(18, 12) };
            Controls.Add(label);
            Size = new Size(label.PreferredWidth + 36, label.PreferredHeight + 24);
            Location = new Point(area.Left + (area.Width - Width) / 2, area.Bottom - Height - 60);
        }
        _timer.Tick += (_, _) => Close();
        _timer.Start();
    }

    protected override bool ShowWithoutActivation => true;

    protected override CreateParams CreateParams
    {
        get
        {
            var cp = base.CreateParams;
            cp.ExStyle |= 0x08000000 /* WS_EX_NOACTIVATE */ | 0x00000080 /* WS_EX_TOOLWINDOW */ | 0x00000008 /* WS_EX_TOPMOST */;
            return cp;
        }
    }

    /// <summary>Shows a message for ~2 seconds. Call on the UI thread.</summary>
    public static void Show(string text)
    {
        _current?.Close();
        _current = new Osd(text);
        _current.FormClosed += (_, _) => _current = null;
        _current.Show();
    }

    protected override void Dispose(bool disposing)
    {
        if (disposing) _timer.Dispose();
        base.Dispose(disposing);
    }
}
