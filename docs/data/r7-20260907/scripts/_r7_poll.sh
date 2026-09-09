ls /home/wutao/regress-run/r7-pipeline.done 2>/dev/null && echo DONE || echo RUNNING
for f in r7-armB-blind r7-armA-blind r7-armB-corpusc r7-armA-corpusc; do
  echo "-- $f: $(tail -1 /home/wutao/regress-run/$f.log 2>/dev/null | head -c 70)"
done
echo "=== 各 run 的 summary 行数 ==="
for d in /home/wutao/regress-run/results/2026090[67]*arm*; do
  echo "$(basename $d): $(wc -l < $d/summary.csv 2>/dev/null)"
done
