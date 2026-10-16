using System.Globalization;
using System.Text.RegularExpressions;

namespace Nimbus.Host.Stores;

/// <summary>An Epic or GOG install that Nimbus runs itself (through legendary or gogdl).</summary>
public sealed class InstallJob
{
    public required string GameId { get; init; }      // "epic:Fortnite", "gog:1207658924"
    public required string Title { get; init; }
    public string State { get; set; } = "queued";     // queued | downloading | installing
    public double? Progress { get; set; }
    public long? BytesDone { get; set; }
    public long? BytesTotal { get; set; }
    public long Speed { get; set; }
    internal CancellationTokenSource Cts { get; } = new();
    internal Func<InstallJob, CancellationToken, Task> Work { get; init; } = (_, _) => Task.CompletedTask;

    // ---- progress lines from legendary / gogdl (both log in the same style) ----
    private static readonly Regex Percent = new(@"= Progress: ([\d.]+)%?(?: \((\d+)/(\d+)\)| (\d+)/(\d+))?", RegexOptions.Compiled);
    private static readonly Regex Speedy = new(@"\+ Download\s+- ([\d.]+) MiB/s", RegexOptions.Compiled);
    private static readonly Regex DlSize = new(@"Download size: ([\d.]+) MiB", RegexOptions.Compiled);

    /// <summary>Reads one output line. <paramref name="bytesInLine"/>: the x/y numbers are bytes (gogdl), not chunks (legendary).</summary>
    public void ReadLine(string line, bool bytesInLine)
    {
        Match m;
        if ((m = Percent.Match(line)).Success)
        {
            State = "downloading";
            double pct = double.Parse(m.Groups[1].Value, CultureInfo.InvariantCulture);
            Progress = Math.Clamp(pct / 100, 0, 1);
            if (bytesInLine && m.Groups[4].Success)
            {
                BytesDone = long.Parse(m.Groups[4].Value, CultureInfo.InvariantCulture);
                BytesTotal = long.Parse(m.Groups[5].Value, CultureInfo.InvariantCulture);
            }
            else if (BytesTotal is { } t) BytesDone = (long)(t * Progress.Value);
        }
        else if ((m = Speedy.Match(line)).Success)
            Speed = (long)(double.Parse(m.Groups[1].Value, CultureInfo.InvariantCulture) * 1024 * 1024);
        else if (!bytesInLine && (m = DlSize.Match(line)).Success)
            BytesTotal = (long)(double.Parse(m.Groups[1].Value, CultureInfo.InvariantCulture) * 1024 * 1024);
    }
}

/// <summary>Runs Epic/GOG installs one at a time, like a store's download queue.</summary>
public sealed class InstallQueue
{
    private readonly List<InstallJob> _jobs = new();
    private readonly object _lock = new();
    private bool _running;

    /// <summary>(game id, error). Error is null when it worked, "cancelled" when you cancelled it.</summary>
    public event Action<string, string?>? Finished;

    public InstallJob? Get(string id) { lock (_lock) return _jobs.FirstOrDefault(j => j.GameId == id); }
    public List<InstallJob> Active() { lock (_lock) return _jobs.ToList(); }

    public void Enqueue(InstallJob job)
    {
        lock (_lock)
        {
            if (_jobs.Any(j => j.GameId == job.GameId)) throw new InvalidOperationException($"{job.Title} is already in the download queue.");
            _jobs.Add(job);
            if (_running) return;
            _running = true;
        }
        _ = Task.Run(PumpAsync);
    }

    public bool Cancel(string id)
    {
        InstallJob? job;
        lock (_lock)
        {
            job = _jobs.FirstOrDefault(j => j.GameId == id);
            if (job is null) return false;
            if (job.State == "queued") _jobs.Remove(job);
        }
        job.Cts.Cancel();
        if (job.State == "queued") Finished?.Invoke(id, "cancelled");
        return true;
    }

    private async Task PumpAsync()
    {
        while (true)
        {
            InstallJob? job;
            lock (_lock)
            {
                job = _jobs.FirstOrDefault();
                if (job is null) { _running = false; return; }
            }
            string? error = null;
            try
            {
                job.State = "downloading";
                await job.Work(job, job.Cts.Token);
            }
            catch (OperationCanceledException) { error = "cancelled"; }
            catch (Exception ex) { error = ex.Message; SystemBridge.Log($"install {job.GameId} FAILED {ex.Message}"); }
            lock (_lock) _jobs.Remove(job);
            SystemBridge.Log($"install {job.GameId} finished: {error ?? "ok"}");
            try { Finished?.Invoke(job.GameId, error); } catch { }
        }
    }
}
