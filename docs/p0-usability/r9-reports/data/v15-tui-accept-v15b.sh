#!/bin/bash
# v15b -- corrected probe. Fix vs v15: status CONTENT line is "│[fold:...] <state>",
# not the block's border title. Anchor on the content line.
BIN=/home/wutao/hearth-tui-new/target/release/hearth-tui
DIR=/home/wutao/hearth-tui-new
OUT=/tmp/v15b
rm -rf $OUT; mkdir -p $OUT
log() { echo "[$(date +%H:%M:%S)] $*"; }

log "guard: ember=$(pgrep -f ember.py | wc -l) hearth=$(pgrep -f 'hearth-slim/target/release/hearth' | wc -l)"

tmux kill-session -t t15b 2>/dev/null
tmux new-session -d -s t15b -x 100 -y 24 -c $DIR "$BIN"
sleep 2
tmux capture-pane -t t15b -p > $OUT/01-startup.txt

log "=== B1: 3+4 (1 LLM call) ==="
tmux send-keys -t t15b '只回答一个数字：3+4=?' Enter
> $OUT/02-poll.txt
DONE_AT=""; LAST_RUNNING_AT=""; ALIVE_MARKS=""
for i in $(seq 1 90); do
  sleep 0.4
  p=$(tmux capture-pane -t t15b -p)
  st=$(echo "$p" | grep -m1 -E '^[│|]\[fold:' | head -1)
  alive=$(pgrep -f "hearth-slim/target/release/hearth" | wc -l)
  now=$(date +%H:%M:%S)
  echo "$now alive=$alive stat=$st" >> $OUT/02-poll.txt
  if [ -n "$st" ] && echo "$st" | grep -q "running"; then LAST_RUNNING_AT=$now; fi
  if [ -n "$st" ] && echo "$st" | grep -q "done ("; then
    DONE_AT=$now
    echo "$p" > $OUT/03-b1-capture.txt
    echo "DONE_AT=$now alive=$alive last_running=$LAST_RUNNING_AT" > $OUT/04-d11.txt
    break
  fi
done
log "DONE_AT=$DONE_AT last_running=$LAST_RUNNING_AT"

# post-done sanity: was the child still alive right after 'done' appeared?
sleep 0.2
echo "alive_200ms_after_done=$(pgrep -f 'hearth-slim/target/release/hearth' | wc -l)" >> $OUT/04-d11.txt
echo "turns_line=$(grep -m1 'model: agnes' $OUT/03-b1-capture.txt)" >> $OUT/04-d11.txt

tmux send-keys -t t15b C-q
sleep 1.5
tmux ls 2>&1 > $OUT/05-tmux-after.txt
echo "V15B_DONE $(date +%H:%M:%S) sessions_v15b=$(tmux ls 2>/dev/null | grep -c t15b)"
