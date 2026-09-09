#!/bin/bash
# P2 Node 05/06: A/B paired long-conversation experiment (interleaved runner)
# Usage: bash run_ab.sh <A|B> <pair#>
# A = baseline threshold 32,000 (default)   B = candidate 65,536 (delayed compaction proxy)
COND=$1
PAIR=$2
RUNDIR=/tmp/p2_ab_${COND}${PAIR}
rm -rf "$RUNDIR"
mkdir -p "$RUNDIR"
cd "$RUNDIR"
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=600
if [ "$COND" = "B" ]; then
  export HEARTH_COMPACT_CHAR_THRESHOLD=65536
fi
# 时间戳（层归 model 噪声输入，S-10）
date -Is
hearth chat '依次用 bash 执行以下 12 条命令（每条单独一次 bash 调用，不要合并，不要重复），分别记录输出的最后一个数字：
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
全部执行完后，凭你已经获得的信息（可以查看你自己写过的文件，但不得重新执行任何 seq 命令）用 write_file 创建 RESULT.txt，内容为 12 行，每行一个数字，顺序对应上面 12 条命令的输出最后一个数字。写完 RESULT.txt 后立即宣告任务完成（DONE）。' \
  --budget 80 \
  --acceptance 'file: RESULT.txt nonempty'
echo "EXIT_CODE=$?"
date -Is
