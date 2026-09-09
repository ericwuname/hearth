#!/bin/bash
for t in b1c b2c b3c b1m b2m b3m a1c a2c a3c a1m a2m a3m; do
  f=/home/wutao/fa/p0_$t.log
  if [ -f "$f" ]; then
    done_flag=$(grep -c "_EXIT=" "$f" 2>/dev/null)
    err=$(grep -c "ERROR agent_core" "$f" 2>/dev/null)
    fails=$(grep -c "Task failed" "$f" 2>/dev/null)
    comps=$(grep -c "Task completed" "$f" 2>/dev/null)
    echo "$t: done=$done_flag ERROR=$err failed=$fails completed=$comps size=$(stat -c%s $f)"
  else
    echo "$t: pending"
  fi
done
[ -f /home/wutao/fa/p0_all_done.flag ] && echo "MATRIX=ALL_DONE" || echo "MATRIX=RUNNING"
