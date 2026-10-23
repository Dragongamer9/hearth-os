# Programs you install for yourself (Claude Code, Claude Desktop's launcher, pip --user...) live in ~/.local/bin:
# keep it on PATH in every login shell, on every Hearth OS edition, even if it was made after you signed in.
case ":${PATH}:" in
  *":${HOME}/.local/bin:"*) ;;
  *) [ -n "$HOME" ] && PATH="${HOME}/.local/bin:${PATH}"; export PATH ;;
esac
