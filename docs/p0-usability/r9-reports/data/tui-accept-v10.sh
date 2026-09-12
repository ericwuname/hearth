#!/usr/bin/env bash
# Review-window INDEPENDENT acceptance for TUI step5 (multi-turn session).
# v10 — new file; tui-accept.sh / v2..v8 left untouched.
# Append-only log. No hardcoded keys. No LLM beyond the TUI's own hearth calls.
set -u
D=/home/wutao/hearth-tui-new
BIN=$D/target/release/hearth-tui
REP=$D/.hearth/reports
export PATH=$HOME/.cargo/bin:$PATH
OUT=/home/wutao/tui-accept-v10.log

say() { echo "$@" | tee -a "$OUT"; }

say "=================================================="
say "v10 acceptance start $(date '+%F %T')"
say "=================================================="

say "--- (1) build (fresh, forced) ---"
cd "$D" || exit 1
touch src/main.rs
cargo build --release 2>&1 | tail -4 | tee -a "$OUT"
say "build_pipe_rc=${PIPESTATUS[0]}"
say "main.rs md5: $(md5sum src/main.rs | cut -d' ' -f1)  lines: $(wc -l < src/main.rs)"
say "binary: $(ls -la "$BIN")"

say "--- (2) reports dir baseline ---"
N0=$(ls -t "$REP" | head -1); C0=$(ls -1 "$REP" | wc -l)
say "before turn1: newest=$N0 count=$C0"

say "--- (3) start TUI in tmux (binary as session cmd) ---"
tmux kill-session -t T6 2>/dev/null
sleep 1
tmux new-session -d -s T6 -x 100 -y 24 -c "$D" "$BIN"
sleep 6
say "session alive: $(tmux has-session -t T6 2>&1 && echo YES || echo NO)"
tmux capture-pane -p -t T6 > /tmp/v10-init.txt
say "--- initial capture ---"
cat /tmp/v10-init.txt | tee -a "$OUT"

say "--- (4) TURN1: 只回答一个数字：3+4=? ---"
tmux send-keys -t T6 "只回答一个数字：3+4=?"
tmux send-keys -t T6 Enter
for i in $(seq 1 24); do
  sleep 5
  P=$(tmux capture-pane -p -t T6 2>/dev/null)
  echo "$P" | grep -qE "done \([0-9]+s\)" && { say "turn1 settled after ~$((i*5))s"; break; }
done
tmux capture-pane -p -t T6 > /tmp/v10-t1.txt
say "--- turn1 capture (tail 14) ---"
tail -14 /tmp/v10-t1.txt | tee -a "$OUT"
N1=$(ls -t "$REP" | head -1); C1=$(ls -1 "$REP" | wc -l)
say "after turn1: newest=$N1 count=$C1"

say "--- (5) TURN2: 我叫小明，请记住 ---"
tmux send-keys -t T6 "我叫小明，请记住"
tmux send-keys -t T6 Enter
for i in $(seq 1 30); do
  sleep 5
  tmux capture-pane -p -t T6 2>/dev/null | grep -qE "steps: 2" && { say "turn2 settled after ~$((i*5))s"; break; }
done
tmux capture-pane -p -t T6 > /tmp/v10-t2.txt
say "--- turn2 capture (tail 16) ---"
tail -16 /tmp/v10-t2.txt | tee -a "$OUT"
N2=$(ls -t "$REP" | head -1); C2=$(ls -1 "$REP" | wc -l)
say "after turn2: newest=$N2 count=$C2"

say "--- (6) TURN3: 我叫什么名字？ ---"
tmux send-keys -t T6 "我叫什么名字？"
tmux send-keys -t T6 Enter
for i in $(seq 1 30); do
  sleep 5
  tmux capture-pane -p -t T6 2>/dev/null | grep -qE "steps: 3" && { say "turn3 settled after ~$((i*5))s"; break; }
done
sleep 3
tmux capture-pane -p -t T6 -S -400 > /tmp/v10-t3.txt
N3=$(ls -t "$REP" | head -1); C3=$(ls -1 "$REP" | wc -l)
say "after turn3: newest=$N3 count=$C3"

say "=================== FULL SCROLLBACK ==================="
cat /tmp/v10-t3.txt | tee -a "$OUT"

say "=================== ASSERTIONS ==================="
FULL=$(cat /tmp/v10-t3.txt)
A1=FAIL; echo "$FULL" | grep -qE "hearth: .*7" && A1=PASS
say "A1 turn1 answered 7 .................... $A1"
A2=FAIL; echo "$FULL" | grep -q "小明" && A2=PASS
say "A2 turn3 recalled 小明 (same session) .. $A2"
A3=FAIL; [ "$N1" != "$N0" ] && A3=PASS
say "A3 turn1 created a NEW session dir ..... $A3  ($N0 -> $N1)"
A4=FAIL; [ "$N2" = "$N1" ] && [ "$C2" = "$C1" ] && A4=PASS
say "A4 turn2 REUSED it (no new dir) ........ $A4  ($C1 -> $C2 dirs)"
A5=FAIL; echo "$FULL" | grep -qE "session: [0-9a-f]{8}" && A5=PASS
say "A5 status bar shows session:<8hex> ..... $A5"
S1=$(echo "$FULL" | grep -oE "session: [0-9a-f]{8}" | head -1)
say "session string seen: $S1"
A6=FAIL; echo "$FULL" | grep -q "running" && A6="STILL-RUNNING"
echo "$FULL" | grep -qE "done \(" && A6=DONE
say "A6 final state ......................... $A6"

say "--- (7) cleanup ---"
tmux kill-session -t T6 2>/dev/null && say "T6 killed" || say "T6 already gone"
tmux ls 2>&1 | tee -a "$OUT"
say "v10 acceptance end $(date '+%F %T')"
say "=================================================="
