#!/bin/bash
source ~/.cargo/env
echo "=== independent artifact verification ==="
for spec in "lr_n08:chainlib" "lr_n09:textkit" "lr_n10:geoutil" "lr_n12_r1:sortlib" "lr_n12_r2:sortlib" "lr_n13:mathnotes"; do
  d="${spec%%:*}"; p="${spec##*:}"
  m="/tmp/$d/$p/Cargo.toml"
  if [ -f "$m" ]; then
    r=$(cargo test --manifest-path "$m" 2>&1 | grep "test result" | head -1)
    echo "$p ($d): $r"
  else
    echo "$p ($d): MISSING"
  fi
done
echo "=== marker files ==="
cat /tmp/lr_n08/CHAIN_RESULT.txt 2>/dev/null
cat /tmp/lr_n13/STRESS_RESULT.txt 2>/dev/null
cat /tmp/lr_n06/secret.txt 2>/dev/null
echo "=== failure attribution: n09 / n12r1 / n12r2 ==="
grep -aE "Task failed|budget_exhausted|deadline exceeded|stalled|GIVE_UP" /home/wutao/fa/lr_n09.log | tail -3
echo "---"
grep -aE "Task failed|budget_exhausted|deadline exceeded|stalled|GIVE_UP" /home/wutao/fa/lr_n12r1.log | tail -3
echo "---"
grep -aE "Task failed|budget_exhausted|deadline exceeded|stalled|GIVE_UP" /home/wutao/fa/lr_n12r2.log | tail -3
echo "=== n06 C-level answer ==="
grep -aB1 -aA2 "CODE-7788" /home/wutao/fa/lr_n06.log | tail -8
