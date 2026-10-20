using System.Globalization;
using System.Text.RegularExpressions;

namespace Nimbus.Host.Stores;

/// <summary>An Epic or GOG install that Nimbus runs itself (through legendary or gogdl).</summary>
public sealed class InstallJob
{
    public required string GameId { get; init; }      // "epic:Fortnite", "gog:1207658924"
    public required string Title { get; init; }
    public string State { get; set; } = "queued";     // queued | downloading | installing | paused
    public double? Progress { get; set; }
    public long? BytesDone { get; set; }
    public long? BytesTotal { get; set; }
    public long Speed { get; set; }
    internal CancellationTokenSource Cts { get; set; } = new();
    /// <summary>Stopped on purpose (Pause, or another download started first): kept in the queue, not finished.
    /// The value is the state it goes back to: "paused" or "queued".</summary>
    internal string? StopTo { get; set; }
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
        bool waiting;
        lock (_lock)
        {
            job = _jobs.FirstOrDefault(j => j.GameId == id);
            if (job is null) return false;
            waiting = job.State is "queued" or "paused";
            if (waiting) _jobs.Remove(job);
            job.StopTo = null;
        }
        job.Cts.Cancel();
        if (waiting) Finished?.Invoke(id, "cancelled");
        return true;
    }

    /// <summary>Downloads › Pause: stops the download, keeping what it got (legendary and gogdl carry on from there).</summary>
    public bool Pause(string id)
    {
        InstallJob? job;
        lock (_lock)
        {
            job = _jobs.FirstOrDefault(j => j.GameId == id);
            if (job is null) return false;
            if (job.State == "paused") return true;
            if (job.State == "queued") { job.State = "paused"; return true; }
            job.StopTo = "paused";
        }
        job.Cts.Cancel();
        return true;
    }

    /// <summary>Downloads › Resume: back in the queue (it starts when it's its turn).</summary>
    public bool Resume(string id)
    {
        lock (_lock)
        {
            var job = _jobs.FirstOrDefault(j => j.GameId == id);
            if (job is null) return false;
            if (job.State == "paused") job.State = "queued";
            if (_running) return true;
            _running = true;
        }
        _ = Task.Run(PumpAsync);
        return true;
    }

    /// <summary>Downloads › Start now: first in the queue; the download that was running waits its turn again.</summary>
    public bool First(string id)
    {
        InstallJob? running = null;
        lock (_lock)
        {
            var job = _jobs.FirstOrDefault(j => j.GameId == id);
            if (job is null) return false;
            _jobs.Remove(job);
            _jobs.Insert(0, job);
            if (job.State == "paused") job.State = "queued";
            running = _jobs.FirstOrDefault(j => j != job && j.State is "downloading" or "installing");
            if (running is not null) running.StopTo = "queued";
        }
        running?.Cts.Cancel();
        return true;
    }

    private async Task PumpAsync()
    {
        while (true)
        {
            InstallJob? job;
            lock (_lock)
            {
                job = _jobs.FirstOrDefault(j => j.State != "paused");
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
            lock (_lock)
            {
                // Paused, or waiting its turn again: stays in the queue with a fresh token.
                if (error == "cancelled" && job.StopTo is { } back && _jobs.Contains(job))
                {
                    job.State = back; job.StopTo = null; job.Speed = 0;
                    job.Cts.Dispose(); job.Cts = new CancellationTokenSource();
                    SystemBridge.Log($"install {job.GameId}: {back}");
                    continue;
                }
                _jobs.Remove(job);
            }
            SystemBridge.Log($"install {job.GameId} finished: {error ?? "ok"}");
            try { Finished?.Invoke(job.GameId, error); } catch { }
        }
    }
}
