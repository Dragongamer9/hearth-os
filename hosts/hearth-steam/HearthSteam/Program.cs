// hearth-steam: a headless Steam engine for Hearth OS. No window. It loads Valve's steamclient.so (through
// OpenSteamworks, from OpenSteamClient: see LICENSE.OpenSteamClient) and answers Hearth on a local socket.
// Started by Hearth (systemd user unit hearth-steam-engine.service) only when Settings › Library › Steam engine is
// Lightweight. Anything it can't do, Hearth does with the normal Steam client instead.
// With --engine open (or HEARTH_STEAM_ENGINE=open, which Hearth sets for the user unit) it runs the Open engine
// instead: Hearth's own Steam client on SteamKit2 (LGPL-2.1, see LICENSE.SteamKit2), with no steamclient at all.
using HearthSteam;

var home = Environment.GetEnvironmentVariable("HOME") ?? Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
var socket = args.SkipWhile(a => a != "--socket").Skip(1).FirstOrDefault() ?? Server.DefaultPath();

var kind = args.SkipWhile(a => a != "--engine").Skip(1).FirstOrDefault() ?? Environment.GetEnvironmentVariable("HEARTH_STEAM_ENGINE") ?? "lightweight";

// --check-interop: builds every P/Invoke stub of this program without calling anything (CI runs it on the published
// build, so a signature .NET can't marshal fails there instead of on a device).
if (args.Contains("--check-interop"))
{
    int bad = 0;
    foreach (var asm in new[] { typeof(SteamEngine).Assembly, typeof(Server).Assembly })
    {
        var (n, problems, missing) = NativeExec.CheckInterop(asm);
        Console.WriteLine($"{asm.GetName().Name}: {n} native calls checked, {problems.Count} problems, {missing.Count} libraries not here ({string.Join(", ", missing)})");
        foreach (var p in problems) Console.WriteLine("problem: " + p);
        bad += problems.Count;
    }
    return bad == 0 ? 0 : 1;
}

// --check-reexec: starts itself again the way the Lightweight engine does (execve with .NET's environment), and the
// second run says so: proves the restart and the environment it carries on the published build.
if (args.Contains("--check-reexec"))
{
    if (Environment.GetEnvironmentVariable("HEARTH_STEAM_REEXEC_CHECK") == "1") { Console.WriteLine("reexec ok: " + string.Join(' ', args)); return 0; }
    Environment.SetEnvironmentVariable("HEARTH_STEAM_REEXEC_CHECK", "1");
    var self = Directory.ResolveLinkTarget("/proc/self/exe", false)?.FullName ?? Environment.ProcessPath!;
    var errno = NativeExec.Exec(self, args.Prepend(self).ToList(), NativeExec.Environ());
    Console.WriteLine($"reexec failed: errno {errno}");
    return 1;
}

// Never disposed: AppDomain.ProcessExit can still fire after Main returns, and stopping twice is fine.
var stop = new ShutdownSignal();
IEngine engine;
Action start;
if (kind == "open")
{
    var open = new OpenEngine(home);
    open.Released += stop.Stop;
    engine = open; start = open.Start;
}
else
{
    var light = new SteamEngine(home);
    light.Released += stop.Stop;
    engine = light; start = light.Start;
}
var server = new Server(engine, s => Console.Error.WriteLine("hearth-steam: " + s));
Console.CancelKeyPress += (_, e) => { e.Cancel = true; stop.Stop(); };
AppDomain.CurrentDomain.ProcessExit += (_, _) => stop.Stop();
using var term = System.Runtime.InteropServices.PosixSignalRegistration.Create(System.Runtime.InteropServices.PosixSignal.SIGTERM, c => { c.Cancel = true; stop.Stop(); });

start();
await server.Run(socket, stop.Token);
// Released (or stopped): give the last answer a moment to go out, then let go of Steam.
await Task.Delay(300);
try { (engine as IDisposable)?.Dispose(); } catch (Exception e) { Console.Error.WriteLine("hearth-steam: while letting go: " + e.GetType().Name); }
return 0;
