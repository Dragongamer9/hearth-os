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
# Shared from your other devices: the PC ready with an Nvidia card (Describe it uses it), the Ally playing, the office PC
# Hearth Link couldn't confirm (pair it again). Stand-in answers (ai-shared-fixtures.py); the PC's chat lists models only.
python3 "$SA" 11436 qwen3:30b-a3b huihui_ai/qwen3-abliterated:14b >/tmp/standin-pc.log 2>&1 & SA3=$!
sleep 1
python3 "$ROOT/.github/scripts/ai-shared-fixtures.py" /tmp/ai-shared http://127.0.0.1:11436/v1
SHX="HEARTH_AI_SHARED_FIXTURE=/tmp/ai-shared/devices.json HEARTH_AI_LINK_FIXTURE=/tmp/ai-shared/link.json"
env $SHX python3 "$ROOT/hosts/linux/desktop/shell/hearth-ai-check" use-shared desktop > /tmp/ai-use-shared.json 2>&1
echo "::notice title=ai-describe: use-shared desktop::$(head -c 300 /tmp/ai-use-shared.json)"
env $SHX python3 "$ROOT/hosts/linux/desktop/shell/hearth-ai-check" shared > /tmp/ai-shared.json 2>&1
echo "::notice title=ai-describe: shared::$(head -c 900 /tmp/ai-shared.json)"
grep -q '"unconfirmed"' /tmp/ai-shared.json || echo "::error title=ai-describe::the office PC isn't shown as unconfirmed: $(head -c 400 /tmp/ai-shared.json)"
grep -q '"inUse": true' /tmp/ai-shared.json || echo "::error title=ai-describe::no device is shown as the one Describe it uses: $(head -c 400 /tmp/ai-shared.json)"
SHOT_WAIT=8 run ai-describe-4-shared-devices hearth-settings 0 env "$AIE" $SHX "$BIN/hearth-settings-app" --page sec-ai --focus "Refresh"
kill $SA3 2>/dev/null; wait $SA3 2>/dev/null
# the canned asks still sleeping after their windows closed (an app that's closed doesn't wait for them)
pkill -f 'hearth-layout describe-app' 2>/dev/null || true
kill $SA1 $SA2 2>/dev/null; wait $SA1 $SA2 2>/dev/null
