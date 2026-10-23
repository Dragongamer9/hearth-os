# Programs you install for yourself (Claude Code, Claude Desktop's launcher...) live in ~/.local/bin.
if not contains -- $HOME/.local/bin $PATH
    set -gx PATH $HOME/.local/bin $PATH
end
