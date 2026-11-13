# Hearth's terminal speaks C:\ (1.11): the common Windows commands, and C:\, D:\ … paths, in any interactive bash
# (Hearth Terminal included). Linux commands and paths keep working as always; nothing here changes a file by itself
# (del, rd /s and move ask first, like cp -i / rm -i). Turn it off for yourself with HEARTH_WINDOWS_COMMANDS=0 in
# ~/.bashrc (before this is read) or with `unset -f dir copy del` and so on. Sourced from /etc/bash.bashrc.
[ -n "${PS1:-}" ] || return 0 2>/dev/null
[ "${HEARTH_WINDOWS_COMMANDS:-1}" = 0 ] && return 0 2>/dev/null
# Settings › System › Drives & folders › Terminal speaks C:\ (files.json "terminalWindows": false turns it off).
grep -qs '"terminalWindows": *false' "${XDG_CONFIG_HOME:-$HOME/.config}/nimbus/files.json" && return 0 2>/dev/null

__hearth_letters=/usr/share/nimbus/desktop/shell/hearth_drive_letters.py
# A Windows path (C:\Users\me, d:/Games, C:) as the real one; anything else as it is.
__hearth_win() {
  local p=$1
  case "$p" in
    [A-Za-z]:|[A-Za-z]:[\\/]*) ;;
    *) printf '%s' "$p"; return 0 ;;
  esac
  local L=${p:0:1}; L=${L^^}
  if [ "$L" = C ]; then
    python3 "$__hearth_letters" c-path "$p" 2>/dev/null | python3 -c 'import json,sys
r=json.load(sys.stdin); p=r.get("path") or ""
print(p if p.startswith("/") else "", end="")' || true
    return 0
  fi
  local root rest
  root=$(python3 "$__hearth_letters" show 2>/dev/null | python3 -c 'import json,sys
d=json.load(sys.stdin).get("letters") or {}
L=sys.argv[1]
items = d.values() if isinstance(d, dict) else d
for v in items:
    if isinstance(v, dict) and str(v.get("letter","")).upper().rstrip(":") == L:
        print(v.get("mount") or v.get("path") or "", end=""); break' "$L")
  [ -n "$root" ] || { printf '%s' "$p"; return 0; }
  rest=${p:2}; rest=${rest//\\//}
  printf '%s%s' "$root" "$rest"
}
__hearth_args() {  # every argument that looks like a Windows path, made real; one per line (NUL-free names only)
  local a; for a in "$@"; do __hearth_win "$a"; printf '\n'; done
}
__hearth_run() {  # __hearth_run CMD ARGS…: the Windows paths among ARGS made real
  local cmd=$1; shift
  local -a out=(); local a r
  for a in "$@"; do
    r=$(__hearth_win "$a")
    if [ -z "$r" ]; then echo "The system cannot find the path specified: $a" >&2; return 1; fi
    out+=("$r")
  done
  "$cmd" "${out[@]}"
}

cd() {
  if [ $# -eq 1 ]; then
    case "$1" in
      [A-Za-z]:|[A-Za-z]:[\\/]*) local r; r=$(__hearth_win "$1"); [ -n "$r" ] || { echo "The system cannot find the path specified." >&2; return 1; }; builtin cd -- "$r"; return ;;
      \\) builtin cd /; return ;;
      *\\*) builtin cd -- "${1//\\//}"; return ;;
    esac
  fi
  builtin cd "$@"
}
eval 'cd..() { cd ..; }' 2>/dev/null    # Windows lets you leave out the space
dir() {
  local -a flags=(-l --group-directories-first -h) paths=(); local a
  for a in "$@"; do
    case "${a,,}" in
      /a|/ah) flags+=(-A) ;;
      /s) flags+=(-R) ;;
      /w) flags=(-C --group-directories-first) ;;
      /b) flags=(-1) ;;
      /o*|/p|/q) ;;
      *) paths+=("$a") ;;
    esac
  done
  __hearth_run ls "${flags[@]}" "${paths[@]}"
}
copy()   { __hearth_run cp -i "$@"; }
xcopy()  { local -a a=(); for x in "$@"; do case "${x,,}" in /s|/e|/i|/y|/h|/q) ;; *) a+=("$x");; esac; done; __hearth_run cp -ri "${a[@]}"; }
robocopy() { __hearth_run rsync -a --info=progress2 "${1%\\}/" "$2"; }
move()   { __hearth_run mv -i "$@"; }
ren()    { __hearth_run mv -i "$@"; }
rename() { __hearth_run mv -i "$@"; }
del()    { local -a a=(); for x in "$@"; do case "${x,,}" in /f|/q|/s|/p) ;; *) a+=("$x");; esac; done; __hearth_run rm -i "${a[@]}"; }
erase()  { del "$@"; }
md()     { __hearth_run mkdir -p "$@"; }
rd()     { local s=0; local -a a=(); for x in "$@"; do case "${x,,}" in /s) s=1;; /q) ;; *) a+=("$x");; esac; done
           if [ $s = 1 ]; then __hearth_run rm -rI "${a[@]}"; else __hearth_run rmdir "${a[@]}"; fi; }
type()   { if [ $# -ge 1 ] && [ -e "$(__hearth_win "$1")" ]; then __hearth_run cat "$@"; else builtin type "$@"; fi; }
cls()    { clear; }
where()  { command -v -a "$@" 2>/dev/null || which -a "$@"; }
tasklist() { ps -eo pid,comm,%cpu,rss --sort=-rss | head -n 40; }
taskkill() { local -a p=(); local f=0; while [ $# -gt 0 ]; do case "${1,,}" in /pid) shift; p+=("$1");; /im) shift; pkill -x "$1";; /f) f=1;; esac; shift; done
             [ ${#p[@]} -gt 0 ] && kill $([ $f = 1 ] && echo -9) "${p[@]}"; }
ipconfig() { ip -brief address; }
ver()    { . /etc/os-release 2>/dev/null; echo "${PRETTY_NAME:-Hearth OS}"; }
start()  { [ $# -gt 0 ] && __hearth_run xdg-open "$@" >/dev/null 2>&1 & }
explorer() { __hearth_run xdg-open "${1:-.}" >/dev/null 2>&1 & }
notepad()  { command -v hearth-text >/dev/null && __hearth_run hearth-text "$@" >/dev/null 2>&1 & }
