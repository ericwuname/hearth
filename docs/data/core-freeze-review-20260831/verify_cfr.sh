#!/bin/bash
# CFR independent artifact verification (Node 09/10/08 products)
source ~/.cargo/env
echo "=== independent artifact verification ==="
for spec in "cfr_n08:chainfree" "cfr_n09:todoapi" "cfr_n09b:units" "cfr_n10a:configlib"; do
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
cat /tmp/cfr_n08/FREEZE_RESULT.txt 2>/dev/null; echo
cat /tmp/cfr_n10b/legacy_app/FINAL.txt 2>/dev/null; echo
cat /tmp/cfr_n09/todoapi/NOTES.md 2>/dev/null; echo
echo "=== n11 artifacts ==="
ls /tmp/cfr_n11/notes/ 2>/dev/null
cat /tmp/cfr_n11/notes/todo.txt 2>/dev/null; echo
cat /tmp/cfr_n11/notes/done.txt 2>/dev/null
