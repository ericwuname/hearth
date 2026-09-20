#!/usr/bin/env bash
# Review-window acceptance v24 (new file; v1..v23 untouched).
# Purpose: POST-fix GREEN check for N6, same instrument as v23 (RED).
# Append-only logging. No existing file overwritten.
export PATH=$HOME/.cargo/bin:$PATH
TUI=/home/wutao/hearth-tui-new
BIN=$TUI/target/release/hearth-tui
LOG=/tmp/r24-accept.log
S=/tmp/r24-screen.txt
SB=/tmp/r24-scroll.txt

: >> "$LOG"
echo "=== $(date '+%F %T') v24 start ===" >> "$LOG"

echo "--- identity ---" >> "$LOG"
md5sum "$TUI/src/main.rs" >> "$LOG"
wc -l < "$TUI/src/main.rs" >> "$LOG"

echo "--- rebuild (warnings visible) ---" >> "$LOG"
cd "$TUI" || exit 1
cargo build --release 2>&1 | tee /tmp/r24-build.txt >> "$LOG"
echo "build_rc=${PIPESTATUS[0]}" >> "$LOG"
printf 'warn_count=%s\n' "$(grep -c 'warning' /tmp/r24-build.txt)" >> "$LOG"
printf 'error_count=%s\n' "$(grep -c '^error' /tmp/r24-build.txt)" >> "$LOG"
md5sum "$BIN" >> "$LOG"
ls -l --time-style=+%F_%T "$BIN" >> "$LOG"

tmux kill-session -t rv24 2>/dev/null

echo "--- start session 100x24 $(date '+%T') ---" >> "$LOG"
tmux new-session -d -s rv24 -x 100 -y 24 -c "$TUI" "$BIN"
sleep 2
tmux send-keys -t rv24 '只回答一个数字：3+4=?' Enter
sleep 45
tmux capture-pane -p -t rv24 > "$S"
echo "viewport_lines=$(wc -l < "$S")" >> "$LOG"
echo "=== VIEWPORT START ===" >> "$LOG"
cat "$S" >> "$LOG"
echo "=== VIEWPORT END ===" >> "$LOG"

echo "--- post-fix metrics (SAME instrument as v23) ---" >> "$LOG"
printf 'dot_eq=%s\n'   "$(grep -cF $'\u2502\u00b7 \u2550' "$S")" >> "$LOG"
printf 'plain_eq=%s\n' "$(grep -cF $'\u2502  \u2550' "$S")" >> "$LOG"
printf 'done_sig=%s\n' "$(grep -cF 'done (' "$S")" >> "$LOG"
printf 'running_sig=%s\n' "$(grep -cF 'running' "$S")" >> "$LOG"
printf 'turns1=%s\n'   "$(grep -cF 'turns: 1' "$S")" >> "$LOG"
grep -F '[fold:' "$S" >> "$LOG"

echo "--- scrollback (-S -400) ---" >> "$LOG"
tmux capture-pane -p -S -400 -t rv24 > "$SB"
printf 'SB_eq_total=%s\n' "$(grep -cF $'\u2550' "$SB")" >> "$LOG"
printf 'SB_dot_eq=%s\n'   "$(grep -cF $'\u2502\u00b7 \u2550' "$SB")" >> "$LOG"
printf 'SB_plain_eq=%s\n' "$(grep -cF $'\u2502  \u2550' "$SB")" >> "$LOG"
echo "--- ═ lines (numbered, lossless) ---" >> "$LOG"
grep -nF $'\u2550' "$SB" >> "$LOG"
echo "--- python cross-check (independent instrument) ---" >> "$LOG"
python3 - "$SB" >> "$LOG" 2>&1 <<'PY'
import sys
p = sys.argv[1]
dotted = plain = tot = 0
for ln in open(p, encoding="utf-8", errors="replace").read().splitlines():
    i = ln.find("\u2550")
    if i < 0:
        continue
    tot += 1
    if "\u00b7 \u2550" in ln[i-3:i+1] or "\u00b7 \u2550" in ln:
        dotted += 1
    if "\u2502  \u2550" in ln:
        plain += 1
print("PY eq_lines=%d dotted=%d plain=%d" % (tot, dotted, plain))
PY

echo "--- Ctrl+Q $(date '+%T') ---" >> "$LOG"
tmux send-keys -t rv24 C-q
sleep 3
tmux has-session -t rv24 2>/dev/null
echo "has_session_rc=$?" >> "$LOG"
echo "--- sessions after ---" >> "$LOG"
tmux ls >> "$LOG" 2>&1
echo "=== $(date '+%F %T') v24 end ===" >> "$LOG"
