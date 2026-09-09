#!/usr/bin/env bash
# R9 包B 总控：四组串行（flock 防并发，.131 禁同机并发 LLM 任务）
set -u
for G in G1 G2 G3 G4; do
  echo "[run_all] $G 开始 $(date -u +%H:%M:%S)"
  flock /tmp/hr-r9.lock bash /home/wutao/r9-pkgB/r9pkgb.sh "$G" > "/home/wutao/r9-pkgB/logs-$G.out" 2>&1
  echo "[run_all] $G 结束 $(date -u +%H:%M:%S) rc=$?"
done
echo "[run_all] 全部完成 $(date -u +%H:%M:%S)"
