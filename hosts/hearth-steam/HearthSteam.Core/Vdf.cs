using System.Text;

namespace HearthSteam;

/// <summary>Steam's text KeyValues (libraryfolders.vdf, config.vdf, the client manifest), read only. Keys are
/// case-insensitive like Steam's. A node is either a string or a nested dictionary.</summary>
public sealed class VdfNode
{
    public string? Value { get; init; }
    public Dictionary<string, VdfNode> Children { get; } = new(StringComparer.OrdinalIgnoreCase);

    public VdfNode? Get(params string[] path)
    {
        VdfNode? n = this;
        foreach (var p in path) { if (n == null || !n.Children.TryGetValue(p, out n)) return null; }
        return n;
    }
    public string? Str(params string[] path) => Get(path)?.Value;
}

public static class Vdf
{
    public static VdfNode Parse(string text)
    {
        var root = new VdfNode();
        var stack = new Stack<VdfNode>();
        stack.Push(root);
        string? key = null;
        int i = 0;
        while (i < text.Length)
        {
            char c = text[i];
            if (char.IsWhiteSpace(c)) { i++; continue; }
            if (c == '/' && i + 1 < text.Length && text[i + 1] == '/') { while (i < text.Length && text[i] != '\n') i++; continue; }
            if (c == '{')
            {
                var child = new VdfNode();
                if (key != null) stack.Peek().Children[key] = child;
                stack.Push(child); key = null; i++; continue;
            }
            if (c == '}') { if (stack.Count > 1) stack.Pop(); key = null; i++; continue; }
            if (c == '[') { while (i < text.Length && text[i] != ']') i++; i++; continue; } // [$WIN32] conditions: ignored
            string tok = c == '"' ? Quoted(text, ref i) : Bare(text, ref i);
            if (key == null) key = tok;
            else { stack.Peek().Children[key] = new VdfNode { Value = tok }; key = null; }
        }
        return root;
    }

    static string Quoted(string t, ref int i)
    {
        var sb = new StringBuilder();
        i++;
        while (i < t.Length && t[i] != '"')
        {
            if (t[i] == '\\' && i + 1 < t.Length)
            {
                i++;
                sb.Append(t[i] switch { 'n' => '\n', 't' => '\t', _ => t[i] });
            }
            else sb.Append(t[i]);
            i++;
        }
        i++;
        return sb.ToString();
    }

    static string Bare(string t, ref int i)
    {
        int s = i;
        while (i < t.Length && !char.IsWhiteSpace(t[i]) && t[i] != '{' && t[i] != '}' && t[i] != '"') i++;
        return t[s..i];
    }
}
