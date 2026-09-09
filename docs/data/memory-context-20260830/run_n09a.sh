#!/bin/bash
# P2 Node 09: resume-after-compact (phase 1: interrupt mid-task after compaction)
cd /tmp/p2_n09
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=75
hearth chat '依次用 bash 执行以下 12 条命令（每条单独一次 bash 调用，不要合并），分别记录输出的最后一个数字：
1) seq 1 1500
2) seq 2001 3500
3) seq 4001 5500
4) seq 6001 7500
5) seq 8001 9500
6) seq 10001 11500
7) seq 12001 13500
8) seq 14001 15500
9) seq 16001 17500
10) seq 18001 19500
11) seq 20001 21500
12) seq 22001 23500
全部执行完后，凭已获得的信息用 write_file 创建 RESULT.txt（12 行数字，顺序对应）。写完立即宣告完成。' \
  --budget 80 \
  --acceptance 'file: RESULT.txt nonempty'
echo "N09A_EXIT=$?"
