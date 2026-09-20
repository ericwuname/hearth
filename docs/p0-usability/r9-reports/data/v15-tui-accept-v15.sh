#!/bin/bash
# v15 -- independent acceptance probe for TUI step6 (D9 / D10 / D11)
# Written by review window. Does NOT reuse any ember-authored script.
# Anchoring rule (own constraint): locate widgets by BORDER, never by a line prefix.
BIN=/home/wutao/hearth-tui-new/target/release/hearth-tui
DIR=/home/wutao/hearth-tui-new
OUT=/tmp/v15
rm -rf $OUT; mkdir -p $OUT

log() { echo "[$(date +%H:%M:%S)] $*"; }

# ---------- concurrency guard ----------
log "=== concurrency guard ==="
EMBER=$(pgrep -f ember.py | grep -v $$ | wc -l)
CHAT=$(pgrep -f "hearth-slim/target/release/hearth" | wc -l)
log "ember.py procs=$EMBER   hearth child procs=$CHAT"
cat > $OUT/00-concurrency.txt <<EOF
ember.py procs = $EMBER
hearth procs   = $CHAT
EOF

# ---------- P1: zero-LLM startup screen ----------
log "=== P1 startup (zero LLM) ==="
for s in t15a t15d1 t15d2 t15d3; do tmux kill-session -t $s 2>/dev/null; done
tmux new-session -d -s t15a -x 100 -y 24 -c $DIR "$BIN"
sleep 2
tmux capture-pane -t t15a -p > $OUT/01-startup.txt
log "startup captured: $(wc -l < $OUT/01-startup.txt) lines"

# ---------- B1: D11 positive + turns=1 ----------
log "=== B1 3+4 (LLM run 1) ==="
tmux send-keys -t t15a '只回答一个数字：3+4=?' Enter
> $OUT/02-poll.txt
DONE_AT=""; CHILD_ALIVE_AT_DONE=""
for i in $(seq 1 120); do
  sleep 0.5
  p=$(tmux capture-pane -t t15a -p)
  st=$(echo "$p" | grep "model: agnes" | head -1)
  alive=$(pgrep -fc "hearth-slim/target/release/hearth" 2>/dev/null || echo 0)
  echo "$(date +%H:%M:%S.%2N) alive=$alive | $st" >> $OUT/02-poll.txt
  if [ -z "$DONE_AT" ] && echo "$st" | grep -q "done ("; then
    DONE_AT=$(date +%H:%M:%S)
    CHILD_ALIVE_AT_DONE=$alive
    tmux capture-pane -t t15a -p > $OUT/03-b1-capture.txt
    break
  fi
done
log "DONE_AT=$DONE_AT child_alive_at_done=$CHILD_ALIVE_AT_DONE"

# sanity: was 'running' ever visible (proves we caught the transition)?
grep -c "running" $OUT/02-poll.txt > $OUT/04-running_count.txt
grep -m1 "running" $OUT/02-poll.txt >> $OUT/04-running_count.txt || echo "no running line" >> $OUT/04-running_count.txt

# ---------- B2: D9 reproducibility x3 (fresh session each) ----------
log "=== B2 D9 leak reproducibility (LLM runs 2-4) ==="
n=0
for s in t15d1 t15d2 t15d3; do
  n=$((n+1))
  tmux kill-session -t $s 2>/dev/null
  tmux new-session -d -s $s -x 100 -y 24 -c $DIR "$BIN"
  sleep 2
  tmux send-keys -t $s '我叫什么名字？' Enter
  D=""
  for i in $(seq 1 120); do
    sleep 0.5
    st=$(tmux capture-pane -t $s -p | grep "model: agnes" | head -1)
    if echo "$st" | grep -q "done ("; then D=1; break; fi
  done
  tmux capture-pane -t $s -p > $OUT/05-d9-$n.txt
  tmux kill-session -t $s 2>/dev/null
  hits=$(grep -icE "wutao|hostname|\buid\b|/home/wutao" $OUT/05-d9-$n.txt)
  evasive=$(grep -cE "无法回答|不知道|未提供|反问" $OUT/05-d9-$n.txt)
  echo "run$n done=$D leak_hits=$hits evasive_lines=$evasive" >> $OUT/06-d9-summary.txt
  log "run$n done=$D leak=$hits evasive=$evasive"
done

# ---------- teardown / exit check ----------
log "=== Ctrl+Q clean exit ==="
tmux send-keys -t t15a C-q
sleep 2
tmux ls 2>&1 | sed 's/^/  tmux: /' > $OUT/07-tmux-after.txt
log "remaining ember=$(pgrep -f ember.py | wc -l) hearth=$(pgrep -f hearth-slim/target/release/hearth | wc -l)"
echo "V15_DONE $(date +%H:%M:%S)"
