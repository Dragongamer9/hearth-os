# Hearth Notes (hosts/hearth-ui/apps/notes; checklist docs/mockups/checklists/hearth-notes.md): every page of the
# approved mockups (docs/mockups/hearth-notes/notes-1…9) on pretend worlds (hearth-notes-fixture.py: a copy of the
# workspace's layout with the mockups' content, a local repository standing in for GitHub, the night of Nov 20 at
# 3:04 AM US Central). Sourced by hearth-apps-rust-test.sh inside its headless screen; run() pictures each one.
NF=/tmp/notes-fixture
rm -rf "$NF"
NOTES_NOW=$(python3 "$ROOT/.github/scripts/hearth-notes-fixture.py" "$NF" "$ROOT" | tail -1) || echo "::error title=notes fixture::it couldn't be made"
NOTES_LOOK="$NF/desktop.json"
printf '{"look": "steam", "mode": "dark", "accent": "#ff7a2f"}\n' > "$NOTES_LOOK"
notes_home() { # WORLD
  export HOME_BEFORE_NOTES="${HOME_BEFORE_NOTES:-$HOME}" PATH_BEFORE_NOTES="${PATH_BEFORE_NOTES:-$PATH}"
  export HOME="$NF/$1/home" XDG_DATA_HOME="$NF/$1/home/.local/share" XDG_CONFIG_HOME="$NF/$1/home/.config" XDG_CACHE_HOME="$NF/$1/home/.cache"
  export HEARTH_DESKTOP_JSON="$NOTES_LOOK" HEARTH_NOTES_NOW="$NOTES_NOW" HEARTH_NOTES_THIS_PC="Hearth OS 1.16 · Fedora edition" HEARTH_NOTES_PC_NAME="Nvidia PC"
  export PATH="$NF/bin:$PATH_BEFORE_NOTES"
}
notes_done() {
  export HOME="$HOME_BEFORE_NOTES" PATH="$PATH_BEFORE_NOTES"
  unset XDG_DATA_HOME XDG_CONFIG_HOME XDG_CACHE_HOME HEARTH_DESKTOP_JSON HEARTH_NOTES_NOW HEARTH_NOTES_THIS_PC HEARTH_NOTES_PC_NAME HOME_BEFORE_NOTES PATH_BEFORE_NOTES HEARTH_NOTES_AWAKE_SECS
}
N="$BIN/hearth-notes-app"
STORE_NOTE=hearth/mockups/hearth-store/README.md
COMPACT="--open-folders hearth,other"

notes_home main
before=$(find "$HOME" -path "$HOME/hearth-workspace/.git" -prune -o -type f -newer "$NOTES_LOOK" -print 2>/dev/null | sort | md5sum)
# At rest: the main window, then the graph after its animation (3 s awake here, 30 s on a PC) has stopped.
run notes-rest-files hearth-notes 20 "$N" --note "$STORE_NOTE" --no-auto-sync
HEARTH_NOTES_AWAKE_SECS=2 run notes-rest-graph hearth-notes 20 "$N" --page graph --graph-mode galaxy --no-auto-sync
run notes-1-main hearth-notes 0 "$N" --note "$STORE_NOTE" --line 13 --open-folders atlas,hearth,hearth/mockups,hearth/mockups/hearth-store --no-auto-sync
run notes-2b-board hearth-notes 0 "$N" --page board --focus card $COMPACT --no-auto-sync
run notes-3-graph-classic hearth-notes 0 "$N" --page graph --graph-mode classic --select "$STORE_NOTE" --still $COMPACT --no-auto-sync
run notes-3b-graph-modes hearth-notes 0 "$N" --page modes --graph-mode classic --still $COMPACT --no-auto-sync
run notes-3c-galaxy hearth-notes 0 "$N" --page graph --graph-mode galaxy --fly ts2010 --select other/research/ts2010-freeware/README.md --flew --zoom 1.6 --still $COMPACT --no-auto-sync
run notes-3d-solar-system hearth-notes 0 "$N" --page graph --graph-mode solar --select "$STORE_NOTE" --still $COMPACT --no-auto-sync
run notes-3e-black-hole hearth-notes 0 "$N" --page graph --graph-mode blackhole --note "$STORE_NOTE" --select "$STORE_NOTE" --still $COMPACT --no-auto-sync
run notes-3g-ember hearth-notes 0 "$N" --page graph --graph-mode ember --select hearth/mockups/hearth-notes/README.md --still $COMPACT --no-auto-sync
run notes-3h-railway-map hearth-notes 0 "$N" --page graph --graph-mode railway --fly ts2010 --fly-closed --select other/research/ts2010-freeware/README.md --still $COMPACT --no-auto-sync
for m in nightsky neural city river; do
  run notes-3i-$m hearth-notes 0 "$N" --page graph --graph-mode $m --select "$STORE_NOTE" --still $COMPACT --no-auto-sync
done
run notes-4-search hearth-notes 0 "$N" --page search --note hearth/notes/1.16-fedora-release-channel/README.md --search "Fedora USB" --search-folder hearth/notes,log --search-days 7 --search-open 1 --quick-switcher g15 --no-auto-sync
run notes-8-settings hearth-notes 0 "$N" --page settings --focus s-sync $COMPACT --no-auto-sync
run notes-9-controller hearth-notes 0 "$N" --controller --osk-text "Tried Hearth Notes on the Ally" --no-auto-sync
run notes-10-source hearth-notes 0 "$N" --note "$STORE_NOTE" --mode source --no-auto-sync
run notes-11-local-graph hearth-notes 0 "$N" --note "$STORE_NOTE" --page files --local --no-auto-sync
HEARTH_NOTES_REDUCE_MOTION=1 run notes-12-reduce-motion hearth-notes 0 "$N" --page modes --no-auto-sync
# Opening it wrote none of your settings (red flag 9) and nothing else of the app's: no hearth-notes.json, no
# trackers or counts changed, the workspace untouched (git status clean).
[ -e "$XDG_CONFIG_HOME/nimbus/hearth-notes.json" ] && echo "::error title=notes::opening Hearth Notes wrote hearth-notes.json: $(cat "$XDG_CONFIG_HOME/nimbus/hearth-notes.json" | head -c 300)"
after=$(find "$HOME" -path "$HOME/hearth-workspace/.git" -prune -o -type f -newer "$NOTES_LOOK" -print 2>/dev/null | sort | md5sum)
[ "$before" = "$after" ] || echo "::error title=notes::opening Hearth Notes wrote a file: $(find "$HOME" -path "$HOME/hearth-workspace/.git" -prune -o -type f -newer "$NOTES_LOOK" -print | head -5 | tr '\n' ' ')"
[ -z "$(git -C "$HOME/hearth-workspace" status --porcelain)" ] || echo "::error title=notes::opening Hearth Notes changed the workspace: $(git -C "$HOME/hearth-workspace" status --porcelain | head -5 | tr '\n' ' ')"
notes_done

notes_home today
run notes-2-today hearth-notes 0 "$N" --page today --quick "Hearth Notes mockups: the pages plus the graph modes|hearth/mockups/hearth-notes|the owner's look" --open-folders atlas,log --no-auto-sync
notes_done

notes_home custom
cp "$XDG_CONFIG_HOME/nimbus/hearth-notes.json" /tmp/notes-custom-before.json
run notes-3f-customise-graph hearth-notes 0 "$N" --page graph --graph-mode classic --show customise --select "$STORE_NOTE" --note "$STORE_NOTE" --still $COMPACT --no-auto-sync
cmp -s /tmp/notes-custom-before.json "$XDG_CONFIG_HOME/nimbus/hearth-notes.json" || echo "::error title=notes::opening Customise changed hearth-notes.json"
notes_done

notes_home sync
run notes-5-sync hearth-notes 0 "$N" --page sync --show sync --commit-message "Hearth Notes mockups: first draft; log" $COMPACT --no-auto-sync
notes_done

notes_home conflict
run notes-6-conflict hearth-notes 0 "$N" --page conflict --show conflict $COMPACT --no-auto-sync
notes_done

notes_home newitem
run notes-7-new-item hearth-notes 0 "$N" --note "$STORE_NOTE" --show new-item --name "Hearth Notes" $COMPACT --no-auto-sync
notes_done

# The first run: no clone yet (and no Git sign-in here), so it says how to sign in.
mkdir -p "$NF/empty/home/.config" "$NF/empty/home/.local/share"
notes_home empty
run notes-13-first-run hearth-notes 0 "$N" --no-auto-sync
notes_done
