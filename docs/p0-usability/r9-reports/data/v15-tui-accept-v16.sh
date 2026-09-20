#!/bin/bash
# v16 -- answer visibility / ordering probe (1 LLM call). Tall pane so nothing is hidden.
BIN=/home/wutao/hearth-tui-new/target/release/hearth-tui
DIR=/home/wutao/hearth-tui-new
OUT=/tmp/v16
rm -rf $OUT; mkdir -p $OUT
log() { echo "[$(date +%H:%M:%S)] $*"; }
log "guard: ember=$(pgrep -f ember.py | wc -l) hearth=$(pgrep -f 'hearth-slim/target/release/hearth' | wc -l)"

tmux kill-session -t t16 2>/dev/null
tmux new-session -d -s t16 -x 110 -y 45 -c $DIR "$BIN"
sleep 2
tmux send-keys -t t16 '只回答一个数字：3+4=?' Enter
for i in $(seq 1 90); do
  sleep 0.4
  st=$(tmux capture-pane -t t16 -p | grep -m1 -E '^[│|]\[fold:')
  if echo "$st" | grep -q "done ("; then echo "DONE at $(date +%H:%M:%S): $st"; break; fi
done
sleep 0.5
tmux capture-pane -t t16 -p > $OUT/01-tall-after-done.txt
# ordering: which line index holds the user echo / hearth answer / summary block
grep -n "> you\|hearth:\|═══════\|【一句话】\|fold:off" $OUT/01-tall-after-done.txt > $OUT/02-order.txt
# small-window replication (the real 100x24 case): does the answer stay visible?
tmux kill-session -t t16 2>/dev/null
tmux new-session -d -s t16 -x 100 -y 24 -c $DIR "$BIN"
sleep 2
tmux send-keys -t t16 '只回答一个数字：3+4=?' Enter
for i in $(seq 1 90); do
  sleep 0.4
  st=$(tmux capture-pane -t t16 -p | grep -m1 -E '^[│|]\[fold:')
  if echo "$st" | grep -q "done ("; then break; fi
done
sleep 0.5
tmux capture-pane -t t16 -p > $OUT/03-small-after-done.txt
echo "--- visible-answer check (small window) ---" > $OUT/04-visibility.txt
if grep -q "hearth:" $OUT/03-small-after-done.txt; then echo "hearth answer line VISIBLE" >> $OUT/04-visibility.txt; else echo "hearth answer line NOT VISIBLE (scrolled off)" >> $OUT/04-visibility.txt; fi
if grep -q "> you" $OUT/03-small-after-done.txt; then echo "user echo VISIBLE" >> $OUT/04-visibility.txt; else echo "user echo NOT VISIBLE" >> $OUT/04-visibility.txt; fi
echo "status: $(grep -m1 -E '^[│|]\[fold:' $OUT/03-small-after-done.txt)" >> $OUT/04-visibility.txt
tmux send-keys -t t16 PageUp
sleep 0.6
tmux capture-pane -t t16 -p > $OUT/05-small-pageup.txt
grep -n "> you\|hearth:" $OUT/05-small-pageup.txt >> $OUT/04-visibility.txt
cat $OUT/04-visibility.txt
tmux send-keys -t t16 C-q
sleep 1
tmux kill-session -t t16 2>/dev/null
log "guard after: ember=$(pgrep -f ember.py | wc -l) hearth=$(pgrep -f 'hearth-slim/target/release/hearth' | wc -l)"
echo "V16_DONE"
