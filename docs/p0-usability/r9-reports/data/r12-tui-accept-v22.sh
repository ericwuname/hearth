#!/bin/bash
# tui-accept-v22.sh -- round-12: task-specified acceptance flow (build + tmux send-keys + capture)
#   plus zero-LLM incremental probes (N6/N7 delimiter rendering, O4, D17 version surface).
# Append-only log. One real LLM call. v1..v21 untouched.
LOG=/tmp/tui-accept-v22.log
exec >>"$LOG" 2>&1
echo "===== v22 START $(date '+%F %T') ====="
export PATH=$HOME/.cargo/bin:$PATH
D=/home/wutao/hearth-tui-new; BIN=$D/target/release/hearth-tui; S=rv22; CAP=/tmp/v22-cap
R=$D/.hearth/reports
mkdir -p $CAP
snap(){ tmux capture-pane -p -t $S > "$CAP/$1.txt" 2>/dev/null; }
st(){ grep -oE '\[fold:(on|off) scroll:[0-9]+/[0-9]+\][^|]*' "$CAP/$1.txt" | tail -1; }
ans(){ python3 - "$1" <<'PYEOF'
import sys, re
p = sys.argv[1]
lines = open(p, encoding='utf-8', errors='replace').read().splitlines()
box = [l for l in lines if l.startswith('\u2502')]
hits = [l for l in box if re.search(r'\u25c2 span', l) or re.search(r'\u2713 Done', l)]
print("ASSERT_ANSWER_VISIBLE: %s  (box_rows=%d, answer_rows=%d)" % ("PASS" if hits else "FAIL", len(box), len(hits)))
for l in hits[:4]:
    print("   hit: %s" % l.strip()[:110])
PYEOF
}

echo "--- V0 state (baseline实测) ---"
date '+%F %T %z'
md5sum $D/src/main.rs; stat -c 'main.rs %s bytes mtime=%y' $D/src/main.rs
md5sum $BIN 2>/dev/null; stat -c 'binary %s bytes mtime=%y' $BIN 2>/dev/null
echo "  ember_proc=$(pgrep -f '[e]mber\.py' | wc -l)  hearthchat_proc=$(pgrep -f '[h]earth chat' | wc -l)"
echo "  sessions_before=$(ls -1 $R 2>/dev/null | wc -l)"
tmux ls 2>&1

echo "--- V1 build --release (task-specified) ---"
cd $D && cargo build --release 2>&1 | tail -6
echo "  build_rc=${PIPESTATUS[0]}"
md5sum $BIN 2>/dev/null; stat -c 'binary_after %s bytes' $BIN 2>/dev/null

echo "--- V2 fresh run 100x24: send-keys '只回答一个数字：3+4=?' + Enter, then sleep 40 (task-specified) ---"
tmux kill-session -t $S 2>/dev/null
tmux new-session -d -s $S -x 100 -y 24 -c $D "$BIN"
sleep 4
snap 00-init
echo "  init_status: $(st 00-init)"
echo "  init_viewport_nonblank_rows=$(grep -c '[^ ]' $CAP/00-init.txt)"
tmux send-keys -t $S -l '只回答一个数字：3+4=?'; sleep 0.6
T0=$(date '+%s'); tmux send-keys -t $S Enter
echo "  ENTER_SENT $(date '+%T')"
sleep 40
snap 10-t40
echo "  t_plus_40s status: $(st 10-t40)"
ans $CAP/10-t40.txt
echo "  ----- PANE 10-t40 (100x24, numbered) -----"; cat -n $CAP/10-t40.txt; echo "  ----- END PANE -----"
for i in $(seq 1 30); do
  if tmux capture-pane -p -t $S 2>/dev/null | grep -qE 'done \('; then
    echo "  'done (' observed at t+$(( $(date '+%s') - T0 ))s"; break
  fi
  sleep 2
done
sleep 2; snap 11-final
echo "  final status: $(st 11-final)"
echo "  sessions_after=$(ls -1 $R 2>/dev/null | wc -l)  newest=$(ls -1t $R 2>/dev/null | head -1)"

echo "--- V3 fold ON / OFF (zero LLM) ---"
tmux send-keys -t $S C-t; sleep 1.5; snap 20-foldon; echo "  foldon_status: $(st 20-foldon)"
ans $CAP/20-foldon.txt
tmux send-keys -t $S C-t; sleep 1.5; snap 21-foldoff; echo "  foldoff_status: $(st 21-foldoff)"

echo "--- V4 N6/N7 delimiter rendering evidence ---"
echo "  == all U+2550 rows in 21-foldoff =="
grep -n $'\u2550' $CAP/21-foldoff.txt | head -20
echo "  dot_delim_rows  (N6: '| . ===') = $(grep -c '│ · ═' $CAP/21-foldoff.txt)"
echo "  plain_delim_rows             = $(grep -c '│  ═' $CAP/21-foldoff.txt)"
echo "  == raw engine stdout shape (zero LLM): newest run-001.md tail =="
NEW=$(ls -1t $R 2>/dev/null | head -1)
[ -n "$NEW" ] && grep -n $'\u2550' $R/$NEW/run-*.md 2>/dev/null | head -10
echo "  == answer-line / space check (N2a residue) =="
grep -oE '[0-9]✓ Done|✓ Done' $CAP/11-final.txt | head -3
echo "  tail_block_dup_count(收尾) = $(grep -c '收尾' $CAP/11-final.txt)"

echo "--- V5 Ctrl+Q (session must be destroyed) ---"
tmux send-keys -t $S C-q; sleep 3
if tmux has-session -t $S 2>/dev/null; then echo "  Ctrl+Q: session ALIVE (FAIL)"; else echo "  Ctrl+Q: session destroyed OK"; fi

echo "--- V6 D17 version surface (zero LLM) ---"
echo "  PATH_hearth=$(command -v hearth 2>&1)"
bash -lc 'type hearth' 2>&1 | head -3 | sed 's/^/  type: /'
for b in /usr/local/bin/hearth $HOME/hearth-slim/target/release/hearth $HOME/codex-r6/target/release/hearth; do
  if [ -x "$b" ]; then
    echo "  BIN $b md5=$(md5sum $b | cut -c1-8) mtime=$(stat -c %y $b | cut -d. -f1) ver=$(timeout 10 $b --version 2>&1 | head -1)"
  else
    echo "  BIN $b ABSENT"
  fi
done
echo "  TUI hardcoded hearth path:"; grep -n 'usr/local/bin/hearth\|hearth-slim' $D/src/main.rs | head -5 | sed 's/^/    /'

echo "--- V7 leftovers ---"
pgrep -af '[h]earth chat|hearth-slim/target/release/[h]earth'; echo "  hearth_end_marker"
tmux ls 2>&1
echo "===== v22 END $(date '+%F %T') ====="
