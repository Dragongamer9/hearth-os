using System.Text;

namespace Nimbus.Host.Library;

/// <summary>Minimal parser for Valve's text KeyValues (.vdf / .acf) format.</summary>
public static class Vdf
{
    public static Dictionary<string, object> Parse(string text)
    {
        int i = 0;
        return ParseObject(text, ref i);
    }

    public static Dictionary<string, object>? Load(string path)
    {
        try { return File.Exists(path) ? Parse(File.ReadAllText(path)) : null; }
        catch { return null; }
    }

    /// <summary>Follows a key path (case-insensitive); returns null if any step is missing.</summary>
    public static object? Get(Dictionary<string, object>? d, params string[] path)
    {
        object? cur = d;
        foreach (var k in path)
        {
            if (cur is not Dictionary<string, object> obj || !obj.TryGetValue(k, out cur)) return null;
        }
        return cur;
    }

    public static Dictionary<string, object>? Obj(Dictionary<string, object>? d, params string[] path) =>
        Get(d, path) as Dictionary<string, object>;

    public static string? Str(Dictionary<string, object>? d, params string[] path) => Get(d, path) as string;

    private static Dictionary<string, object> ParseObject(string s, ref int i)
    {
        var d = new Dictionary<string, object>(StringComparer.OrdinalIgnoreCase);
        while (true)
        {
            var key = Next(s, ref i);
            if (key is null || key.Value.Brace == '}') return d;
            if (key.Value.Brace == '{') continue; // stray brace, ignore
            var val = Next(s, ref i);
            if (val is null) return d;
            if (val.Value.Brace == '{') d[key.Value.Text] = ParseObject(s, ref i);
            else if (val.Value.Brace == '}') return d;
            else d[key.Value.Text] = val.Value.Text;
        }
    }

    private readonly record struct Token(string Text, char Brace);

    private static Token? Next(string s, ref int i)
    {
        while (i < s.Length)
        {
            char c = s[i];
            if (char.IsWhiteSpace(c)) { i++; continue; }
            if (c == '/' && i + 1 < s.Length && s[i + 1] == '/')
            {
                while (i < s.Length && s[i] != '\n') i++;
                continue;
            }
            if (c is '{' or '}') { i++; return new Token("", c); }
            if (c == '"')
            {
                var sb = new StringBuilder();
                i++;
                while (i < s.Length && s[i] != '"')
                {
                    if (s[i] == '\\' && i + 1 < s.Length)
                    {
                        char n = s[i + 1];
                        sb.Append(n switch { 'n' => '\n', 't' => '\t', _ => n });
                        i += 2;
                    }
                    else sb.Append(s[i++]);
                }
                i++; // closing quote
                return new Token(sb.ToString(), '\0');
            }
            int start = i;
            while (i < s.Length && !char.IsWhiteSpace(s[i]) && s[i] is not ('{' or '}' or '"')) i++;
            return new Token(s[start..i], '\0');
        }
        return null;
    }
}
