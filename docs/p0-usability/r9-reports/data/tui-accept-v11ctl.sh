#!/usr/bin/env bash
# v11 — discriminating control: does a BRAND-NEW session (chat) already know 小明?
# If yes -> the name comes from workspace notes/, not session memory
#          -> the step5 card's "(b) only possible via resume" claim is INVALID.
# Append-only log. No keys.
set -u
D=/home/wutao/hearth-tui-new
BIN=$D/target/release/hearth-tui
REP=$D/.hearth/reports
OUT=/home/wutao/tui-accept-v11ctl.log
say() { echo "$@" | tee -a "$OUT"; }

say "=================================================="
say "v11 CONTROL start $(date '+%F %T')"
say "=================================================="
say "--- notes/ inventory (before) ---"
ls -la "$D/notes/" | tee -a "$OUT"
say "--- notes/user_profile.md bytes (python repr) ---"
python3 -c "import pathlib;print(repr(pathlib.Path('$D/notes/user_profile.md').read_text(encoding='utf-8')))" | tee -a "$OUT"
say "--- contains 小明 ? ---"
grep -c '小明' "$D/notes/user_profile.md" | tee -a "$OUT"

C0=$(ls -1 "$REP" | wc -l); N0=$(ls -t "$REP" | head -1)
say "reports before: count=$C0 newest=$N0"

say "--- fresh TUI session (session_uuid starts None => first msg goes to 'chat') ---"
tmux kill-session -t T7 2>/dev/null
sleep 1
tmux new-session -d -s T7 -x 100 -y 24 -c "$D" "$BIN"
sleep 6
tmux capture-pane -p -t T7 | grep -E "session:|steps:" | tee -a "$OUT"

say "--- FIRST message of this brand-new session: 我叫什么名字？ ---"
tmux send-keys -t T7 "我叫什么名字？"
tmux send-keys -t T7 Enter
for i in $(seq 1 18); do
  sleep 5
  tmux capture-pane -p -t T7 2>/dev/null | grep -qE "done \([0-9]+s\)" && { say "settled after ~$((i*5))s"; break; }
done
tmux capture-pane -p -t T7 > /tmp/v11-ctl.txt
say "--- capture (tail 20) ---"
tail -20 /tmp/v11-ctl.txt | tee -a "$OUT"

C1=$(ls -1 "$REP" | wc -l); N1=$(ls -t "$REP" | head -1)
say "reports after: count=$C1 newest=$N1 (new dir => chat path taken)"

say "--- newest run-001.md ---"
head -30 "$REP/$N1/run-001.md" 2>/dev/null | tee -a "$OUT"

say "=================== CONTROL VERDICT ==================="
F=$(cat /tmp/v11-ctl.txt)
if echo "$F" | grep -q '小明'; then
  say "C1 fresh session ANSWERED 小明 => notes/ IS the source (confound REAL)"
  say "    => card claim '(b) only possible via resume' = INVALID"
else
  say "C1 fresh session did NOT say 小明 => notes/ not auto-injected into fresh session"
  say "    => resume's 小明 more plausibly from session state (claim weaker but stands)"
fi
say "--- notes/ inventory (after) ---"
ls -la "$D/notes/" | tee -a "$OUT"

tmux kill-session -t T7 2>/dev/null && say "T7 killed" || say "T7 already gone"
say "v11 CONTROL end $(date '+%F %T')"
