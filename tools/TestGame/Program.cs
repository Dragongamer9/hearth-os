// A stand-in "game" for testing Nimbus's launch → running → exit flow on PCs without real games.
ApplicationConfiguration.Initialize();
var form = new Form
{
    Text = "Nimbus Test Game",
    BackColor = Color.FromArgb(20, 40, 60),
    ClientSize = new Size(900, 500),
    StartPosition = FormStartPosition.CenterScreen,
};
var started = DateTime.Now;
var label = new Label
{
    Dock = DockStyle.Fill,
    ForeColor = Color.White,
    Font = new Font("Segoe UI", 28, FontStyle.Bold),
    TextAlign = ContentAlignment.MiddleCenter,
    Text = "Nimbus Test Game\nrunning",
};
form.Controls.Add(label);
var timer = new System.Windows.Forms.Timer { Interval = 1000 };
timer.Tick += (_, _) => label.Text = $"Nimbus Test Game\nrunning for {(int)(DateTime.Now - started).TotalSeconds}s";
timer.Start();
Application.Run(form);
