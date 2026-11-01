using System.Collections;
using System.Reflection;
using System.Runtime.InteropServices;

namespace HearthSteam;

/// <summary>Starting the process again with Steam's libraries on the path (the Lightweight engine). The argument and
/// environment lists are handed to execve as plain arrays of UTF-8 C strings that this code allocates itself: .NET's
/// built-in marshaller can't pair a string[] with UTF-8 (0.6.59's "Cannot marshal 'parameter #2'" that kept the engine
/// from ever starting), and Environment.SetEnvironmentVariable only changes .NET's own copy of the environment, so
/// the environment is passed explicitly instead of relying on execv's inherited one.</summary>
public static class NativeExec
{
    [DllImport("libc", EntryPoint = "execve", SetLastError = true)]
    static extern int execve([MarshalAs(UnmanagedType.LPUTF8Str)] string file, IntPtr[] argv, IntPtr[] envp);

    /// <summary>A NULL-terminated array of newly allocated UTF-8 C strings (free with <see cref="Free"/>).</summary>
    public static IntPtr[] Alloc(IEnumerable<string> items)
    {
        var list = new List<IntPtr>();
        foreach (var s in items) list.Add(Marshal.StringToCoTaskMemUTF8(s));
        list.Add(IntPtr.Zero);
        return list.ToArray();
    }

    public static void Free(IntPtr[] block)
    {
        foreach (var p in block) if (p != IntPtr.Zero) Marshal.FreeCoTaskMem(p);
    }

    /// <summary>"KEY=value" for every variable in .NET's view of the environment (which has the changes made with
    /// Environment.SetEnvironmentVariable), sorted for the tests.</summary>
    public static List<string> Environ()
    {
        var list = new List<string>();
        foreach (DictionaryEntry e in Environment.GetEnvironmentVariables())
        {
            var k = e.Key?.ToString();
            if (string.IsNullOrEmpty(k) || k.Contains('=') || k.Contains('\0')) continue;
            var v = e.Value?.ToString() ?? "";
            if (v.Contains('\0')) continue;
            list.Add(k + "=" + v);
        }
        list.Sort(StringComparer.Ordinal);
        return list;
    }

    /// <summary>Replaces this process with <paramref name="file"/> (argv[0] included in <paramref name="args"/>).
    /// Returns only when that failed, with errno.</summary>
    public static int Exec(string file, IReadOnlyList<string> args, IEnumerable<string> environment)
    {
        var argv = Alloc(args);
        var envp = Alloc(environment);
        try
        {
            execve(file, argv, envp);
            return Marshal.GetLastPInvokeError();
        }
        finally { Free(argv); Free(envp); }
    }

    /// <summary>Checks that every P/Invoke in the assembly can be marshalled, without calling any of them (each one's
    /// stub is built and its library function looked up). Libraries that aren't on this machine (steamclient) are
    /// reported as missing, not as errors. Returns (checked, problems).</summary>
    public static (int Checked, List<string> Problems, List<string> Missing) CheckInterop(Assembly asm)
    {
        var problems = new List<string>();
        var missing = new List<string>();
        int n = 0;
        Type[] types;
        try { types = asm.GetTypes(); }
        catch (ReflectionTypeLoadException e) { types = e.Types.Where(t => t != null).ToArray()!; }
        foreach (var t in types)
        foreach (var m in t.GetMethods(BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly))
        {
            if ((m.Attributes & MethodAttributes.PinvokeImpl) == 0) continue;
            n++;
            var name = $"{t.FullName}.{m.Name}";
            // The rule .NET applies when it builds the stub (checked here too, in case the stub is built lazily):
            // a string array needs an ANSI/UTF-16 element type; UTF-8 string arrays must be marshalled by hand.
            foreach (var p in m.GetParameters())
            {
                var ma = p.GetCustomAttribute<MarshalAsAttribute>();
                if (p.ParameterType == typeof(string[]) && ma != null && ma.ArraySubType != 0 && ma.ArraySubType is not (UnmanagedType.LPStr or UnmanagedType.LPWStr or UnmanagedType.LPTStr))
                    problems.Add($"{name}: string[] parameter '{p.Name}' with element type {ma.ArraySubType} can't be marshalled");
            }
            try { Marshal.Prelink(m); }
            catch (DllNotFoundException) { missing.Add(name); }
            catch (EntryPointNotFoundException) { missing.Add(name); }
            catch (Exception e) { problems.Add($"{name}: {e.GetType().Name}: {e.Message}"); }
        }
        return (n, problems, missing);
    }
}

/// <summary>The process's "stop now" signal, safe to trigger from anywhere at any time: from the socket's release, a
/// signal handler, or AppDomain.ProcessExit after Main has already finished (0.6.59 crashed there with
/// ObjectDisposedException, so systemd saw an abort instead of a clean exit).</summary>
public sealed class ShutdownSignal
{
    readonly CancellationTokenSource cts = new();
    int stopped;
    public CancellationToken Token => cts.Token;
    public bool Stopped => Volatile.Read(ref stopped) == 1;

    public void Stop()
    {
        if (Interlocked.Exchange(ref stopped, 1) == 1) return;
        try { cts.Cancel(); } catch (ObjectDisposedException) { } catch (AggregateException) { }
    }
}
