# Describe it on a slow device (the owner's laptop, Nov 24, 2026: CPU Ollama at ~4 words a second, his PC's AI over
# Hearth Link): Settings › AI with "Describe it uses" (Fastest available picks the PC: this laptop has no graphics card),
# and Describe it's waiting line in Hearth Files at 5 s and at ~25 s (who's asked, the seconds, "slow on this device
# (about 4 words a second)", where to pick a faster one, Cancel). Settings is measured at rest too (the run needs one). Stand-in AIs answer only "which models" (ai-standin.py);
# the AI's answer is CI's canned one, made to take 2 minutes (HEARTH_DESCRIBE_FAKE_WAIT). Sourced by hearth-apps-rust-test.sh.
SA="$ROOT/.github/scripts/ai-standin.py"
python3 "$SA" 11434 dolphin3:8b >/tmp/standin-here.log 2>&1 & SA1=$!
python3 "$SA" 11435 qwen3:30b-a3b >/tmp/standin-link.log 2>&1 & SA2=$!
sleep 1
AIE="HEARTH_SHELL_DIR=$ROOT/hosts/linux/desktop/shell"
look steam dark
python3 "$ROOT/hosts/linux/desktop/shell/hearth-ai-check" for describe > /tmp/ai-describe-for.json 2>&1
echo "::notice title=ai-describe: for describe::$(head -c 600 /tmp/ai-describe-for.json)"
grep -q '"from": "link"' /tmp/ai-describe-for.json || echo "::error title=ai-describe::Fastest available didn't pick the other device over this slow one: $(head -c 400 /tmp/ai-describe-for.json)"
SHOT_WAIT=6 run ai-describe-1-settings hearth-settings 10 env "$AIE" "$BIN/hearth-settings-app" --page sec-ai
cat > /tmp/ai-describe-fake.json <<'JSON'
{"kind": "app", "layout": {"sidebar": "icons"}, "changes": ["The sidebar shows icons only"]}
JSON
mkdir -p "$HOME/Downloads"
FK="HEARTH_DESCRIBE_FAKE=/tmp/ai-describe-fake.json HEARTH_DESCRIBE_FAKE_WAIT=120 HEARTH_DESCRIBE_FAKE_WORDS=4.4 HEARTH_DESCRIBE_FAKE_FROM=here HEARTH_DESCRIBE_FAKE_LABEL=Ollama"
SHOT_WAIT=5 run ai-describe-2-waiting-5s hearth-files 0 env PATH="$BIN:$PATH" $FK "$BIN/hearth-files-app" "$HOME/Downloads" --describe "Sidebar icons only"
SHOT_WAIT=26 run ai-describe-3-waiting-25s hearth-files 0 env PATH="$BIN:$PATH" $FK "$BIN/hearth-files-app" "$HOME/Downloads" --describe "Sidebar icons only"
# the canned asks still sleeping after their windows closed (an app that's closed doesn't wait for them)
pkill -f 'hearth-layout describe-app' 2>/dev/null || true
kill $SA1 $SA2 2>/dev/null; wait $SA1 $SA2 2>/dev/null
