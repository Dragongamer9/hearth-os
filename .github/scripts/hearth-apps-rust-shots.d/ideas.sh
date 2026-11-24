# Hearth Ideas (hosts/hearth-ui/apps/ideas; checklist docs/mockups/checklists/hearth-ideas.md): the inbox
# (desktop-layouts board 10), an idea's page (ideas-1), one being built (ideas-2) and the Ship box (ideas-3), on pretend
# GitHub answers ($HEARTH_IDEAS_FIXTURE, keyed by the gh api path; nothing is written). Sourced by
# hearth-apps-rust-test.sh inside its headless screen; run() pictures each one.
IFX=/tmp/ideas-fx
rm -rf "$IFX"; mkdir -p "$IFX"
python3 - "$IFX" "$ROOT/docs/mockups/hearth-ideas/ideas-2-being-built.png" <<'PY' || echo "::error title=ideas fixture::it couldn't be made"
import base64, json, sys, os
out, pic = sys.argv[1], sys.argv[2]
L, R = "repos/Dragongamer9/hearth-logs", "repos/Dragongamer9/hearth-os"
def body(part, asked, needs, closest=None, sketch=False, kept=False):
    j = {"format": 1, "part": part, "asked": asked, "needs": needs, "sketch": sketch, "hearth": "1.17", "app": "", "kept": kept}
    if closest: j["closest"] = {"name": closest}
    key = part + ":" + ("|".join(sorted(n.lower() for n in needs)) or "other")
    return "**Asked in:** %s\n\n```json\n%s\n```\n\nkey: `%s`" % (part, json.dumps(j, indent=2), key)
def issue(n, part, asked, needs, count, labels=(), closest=None, sketch=False, kept=False, created="2026-11-22T08:00:00Z", updated="2026-11-22T12:00:00Z"):
    return {"number": n, "title": "Idea · %s · %s · %d asked" % (part, "; ".join(needs) or asked, count), "state": "open",
            "labels": [{"name": x} for x in ("idea", "idea:" + part, *labels)], "created_at": created, "updated_at": updated,
            "comments": count - 1, "body": body(part, asked, needs, closest, sketch, kept)}
def world(replay_labels):
    ideas = [
        issue(812, "quickaccess", "Add a button that records the last 30 seconds of my game, like the Xbox does.", ["record the last 30 seconds of a game"], 23, replay_labels, "A Record button in Quick Access", kept=True),
        issue(815, "bar", "Show my Nvidia PC's GPU and temperature in the bar", [], 11, (), "Nvidia PC at a glance"),
        issue(816, "files", "Open two folders side by side", [], 7, (), "Two folders side by side"),
        issue(817, "calendar", "The calendar shows my game sessions", ["game sessions in the calendar"], 4, (), None, True),
    ]
    for k, (st, n) in enumerate([("state:later", 6), ("state:shipped", 9), ("state:no", 4)]):
        for m in range(n):
            ideas.append(issue(900 + k * 20 + m, "settings" if m % 2 else "files", "An older idea", ["something else %d %d" % (k, m)], 1, (st,)))
    for m in range(10):
        ideas.append(issue(960 + m, "taskmanager" if m % 3 == 0 else "dock", "Another new idea %d" % m, ["new thing %d" % m], 1))
    if "state:building" in replay_labels:
        ideas.append(issue(818, "store", "Show what changed before I update", ["a changes list"], 3, ("state:building",)))
    words = ["Instant replay like Shadowplay, in Game Mode too", "save the last minute when I press the share button on my controller"]
    comments = [{"body": 'Asked again (%d times now): "%s" · Hearth OS 1.17%s%s' % (i + 2, words[i] if i < 2 else "record the last 30 seconds please",
                 " · with a sketch" if i in (3, 9) else "", " · kept what Describe it made" if i % 3 else ""), "created_at": "2026-11-22T%02d:00:00Z" % (9 + i // 6)} for i in range(22)]
    w = {L + "/issues?labels=idea&state=all&per_page=100": ideas, R + "/releases/latest": {"tag_name": "v1.17"}}
    for i in ideas:
        w[L + "/issues/%d/comments?per_page=100" % i["number"]] = []
    w[L + "/issues/812/comments?per_page=100"] = comments
    return w
def built(w, ready):
    br = "idea/instant-replay"
    w[L + "/issues/812/comments?per_page=100"] = w[L + "/issues/812/comments?per_page=100"] + [{"body": "Sent to your AI team: Claude leads, ChatGPT checks · branch `%s`" % br, "created_at": "2026-11-22T14:31:00Z"}]
    sha = "4f2a9c1d"
    w[R + "/branches/" + br] = {"commit": {"sha": sha, "commit": {"committer": {"date": "2026-11-22T14:38:00Z"}}}}
    w[R + "/commits/%s/check-runs?per_page=100" % sha] = {"check_runs": [{"name": "Mantel in Rust", "status": "completed", "conclusion": "success"}, {"name": "Capture", "status": "completed", "conclusion": "success"}] + ([] if ready else [{"name": "Pictures", "status": "in_progress", "conclusion": None}])}
    w[R + "/pulls?head=Dragongamer9:%s&base=bughunt&state=all" % br] = [{"number": 812, "state": "open", "merged_at": None, "body": "Keeps 30 s in memory only while a game runs (Game Mode or a full-screen window), about 60 MB; nothing written until you press Save.\n\nAt rest: 0.1% CPU"}] if ready else []
    w[R + "/pulls/812/files?per_page=100"] = [{"filename": "hosts/mantel-rs/src/quick.rs"}, {"filename": "hosts/hearth-ui/apps/capture/src/replay.rs"}, {"filename": "docs/ROADMAP.md"}]
    w[R + "/pulls/812/reviews"] = [{"state": "COMMENTED"}] if ready else []
    shots = "ci/shots-idea-instant-replay"
    w[R + "/contents/?ref=" + shots] = [{"name": "rs-quick-access-replay.jpg"}]
    w[R + "/commits/" + shots] = {"commit": {"committer": {"date": "2026-11-22T14:44:00Z"}}}
    w[R + "/contents/rs-quick-access-replay.jpg?ref=" + shots] = {"content": base64.encodebytes(open(pic, "rb").read()).decode()}
    return w
json.dump(world(()), open(os.path.join(out, "new.json"), "w"))
json.dump(built(world(("state:building",)), False), open(os.path.join(out, "built.json"), "w"))
json.dump(built(world(("state:building",)), True), open(os.path.join(out, "ready.json"), "w"))
PY
run ideas-0-inbox hearth-ideas 20 env HEARTH_IDEAS_FIXTURE="$IFX/new.json" "$BIN/hearth-ideas-app"
run ideas-1-idea-page hearth-ideas 0 env HEARTH_IDEAS_FIXTURE="$IFX/new.json" "$BIN/hearth-ideas-app" --open 812
run ideas-2-being-built hearth-ideas 0 env HEARTH_IDEAS_FIXTURE="$IFX/built.json" "$BIN/hearth-ideas-app" --open 812 --page built
SHOT_WAIT=3.5 run ideas-3-ship-it hearth-ideas 0 env HEARTH_IDEAS_FIXTURE="$IFX/ready.json" "$BIN/hearth-ideas-app" --open 812 --page ship
