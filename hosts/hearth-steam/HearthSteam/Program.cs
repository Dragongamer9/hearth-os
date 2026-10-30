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

using var cts = new CancellationTokenSource();
IEngine engine;
Action start;
if (kind == "open")
{
    var open = new OpenEngine(home);
    open.Released += () => cts.Cancel();
    engine = open; start = open.Start;
}
else
{
    var light = new SteamEngine(home);
    light.Released += () => cts.Cancel();
    engine = light; start = light.Start;
}
var server = new Server(engine, s => Console.Error.WriteLine("hearth-steam: " + s));
Console.CancelKeyPress += (_, e) => { e.Cancel = true; cts.Cancel(); };
AppDomain.CurrentDomain.ProcessExit += (_, _) => cts.Cancel();
using var term = System.Runtime.InteropServices.PosixSignalRegistration.Create(System.Runtime.InteropServices.PosixSignal.SIGTERM, c => { c.Cancel = true; cts.Cancel(); });

start();
await server.Run(socket, cts.Token);
// Released (or stopped): give the last answer a moment to go out, then let go of Steam.
await Task.Delay(300);
(engine as IDisposable)?.Dispose();
return 0;
