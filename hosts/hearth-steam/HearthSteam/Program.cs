// hearth-steam: a headless Steam engine for Hearth OS. No window. It loads Valve's steamclient.so (through
// OpenSteamworks, from OpenSteamClient: see LICENSE.OpenSteamClient) and answers Hearth on a local socket.
// Started by Hearth (systemd user unit hearth-steam-engine.service) only when Settings › Library › Steam engine is
// Lightweight. Anything it can't do, Hearth does with the normal Steam client instead.
using HearthSteam;

var home = Environment.GetEnvironmentVariable("HOME") ?? Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
var socket = args.SkipWhile(a => a != "--socket").Skip(1).FirstOrDefault() ?? Server.DefaultPath();

using var cts = new CancellationTokenSource();
using var engine = new SteamEngine(home);
var server = new Server(engine, s => Console.Error.WriteLine("hearth-steam: " + s));
engine.Released += () => cts.Cancel();
Console.CancelKeyPress += (_, e) => { e.Cancel = true; cts.Cancel(); };
AppDomain.CurrentDomain.ProcessExit += (_, _) => cts.Cancel();
using var term = System.Runtime.InteropServices.PosixSignalRegistration.Create(System.Runtime.InteropServices.PosixSignal.SIGTERM, c => { c.Cancel = true; cts.Cancel(); });

engine.Start();
await server.Run(socket, cts.Token);
// Released (or stopped): give the last answer a moment to go out, then let go of Steam.
await Task.Delay(300);
return 0;
