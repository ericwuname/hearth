#!/usr/bin/env bash
# Review-window acceptance v23 (new file; v1..v22 untouched).
# Purpose: pre-fix RED baseline for N6 (Proc dot on U+2550 delimiter lines)
#          + four-stage verdict on the current binary.
# Append-only logging. No existing file overwritten.
export PATH=$HOME/.cargo/bin:$PATH
TUI=/home/wutao/hearth-tui-new
BIN=$TUI/target/release/hearth-tui
LOG=/tmp/r23-accept.log
S=/tmp/r23-screen.txt
SB=/tmp/r23-scroll.txt

: >> "$LOG"
echo "=== $(date '+%F %T') v23 start ===" >> "$LOG"

echo "--- git/source identity ---" >> "$LOG"
md5sum "$TUI/src/main.rs" >> "$LOG"
wc -l < "$TUI/src/main.rs" >> "$LOG"

echo "--- build ---" >> "$LOG"
cd "$TUI" || exit 1
cargo build --release >> "$LOG" 2>&1
echo "build_rc=$?" >> "$LOG"
md5sum "$BIN" >> "$LOG"
ls -l --time-style=+%F_%T "$BIN" >> "$LOG"

echo "--- sessions before ---" >> "$LOG"
tmux ls >> "$LOG" 2>&1
tmux kill-session -t rv23 2>/dev/null

echo "--- start session 100x24 $(date '+%T') ---" >> "$LOG"
tmux new-session -d -s rv23 -x 100 -y 24 -c "$TUI" "$BIN"
sleep 2
tmux capture-pane -p -t rv23 > "$S"
echo "startup_lines=$(wc -l < "$S")" >> "$LOG"
cat "$S" >> "$LOG"

echo "--- send question $(date '+%T') ---" >> "$LOG"
tmux send-keys -t rv23 '只回答一个数字：3+4=?' Enter
sleep 40
tmux capture-pane -p -t rv23 > "$S"
echo "viewport_lines=$(wc -l < "$S")" >> "$LOG"
echo "=== VIEWPORT START ===" >> "$LOG"
cat "$S" >> "$LOG"
echo "=== VIEWPORT END ===" >> "$LOG"

echo "--- viewport metrics ---" >> "$LOG"
printf 'dot_eq=%s\n'  "$(grep -cF $'\u2502\u00b7 \u2550' "$S")" >> "$LOG"
printf 'plain_eq=%s\n' "$(grep -cF $'\u2502  \u2550' "$S")" >> "$LOG"
printf 'done_sig=%s\n' "$(grep -cF 'done (' "$S")" >> "$LOG"
printf 'running_sig=%s\n' "$(grep -cF 'running' "$S")" >> "$LOG"
printf 'statusline=%s\n' "$(grep -oF '[fold:' "$S" | head -1)" >> "$LOG"
grep -F '[fold:' "$S" >> "$LOG"

echo "--- scrollback capture (-S -400) ---" >> "$LOG"
tmux capture-pane -p -S -400 -t rv23 > "$SB"
printf 'scrollback_lines=%s\n' "$(wc -l < "$SB")" >> "$LOG"
printf 'SB_eq_total=%s\n'   "$(grep -cF $'\u2550' "$SB")" >> "$LOG"
printf 'SB_dot_eq=%s\n'     "$(grep -cF $'\u2502\u00b7 \u2550' "$SB")" >> "$LOG"
printf 'SB_plain_eq=%s\n'   "$(grep -cF $'\u2502  \u2550' "$SB")" >> "$LOG"
echo "--- ═ lines in scrollback (numbered, lossless) ---" >> "$LOG"
grep -nF $'\u2550' "$SB" >> "$LOG"

echo "--- Ctrl+Q $(date '+%T') ---" >> "$LOG"
tmux send-keys -t rv23 C-q
sleep 3
tmux has-session -t rv23 2>/dev/null
echo "has_session_rc=$?" >> "$LOG"
echo "--- sessions after ---" >> "$LOG"
tmux ls >> "$LOG" 2>&1
echo "=== $(date '+%F %T') v23 end ===" >> "$LOG"
