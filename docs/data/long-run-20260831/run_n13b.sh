#!/bin/bash
# P2-LR Node 13: phase 2 — resume + second failure + repair + compact#2 + complete
cd /tmp/lr_n13
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_COMPACT_CHAR_THRESHOLD=6000
export HEARTH_TASK_TIMEOUT_SECS=480
SESSION=$(ls /tmp/lr_n13/.hearth/reports/ | head -1)
echo "RESUMING_SESSION=$SESSION"
hearth resume "$SESSION" '继续完成 mathnotes：
1) 用 bash 执行 seq 2000 2900 和 seq 3000 3900（继续制造历史体积，触发第二次压缩）
2) 用 write_file 创建 STRESS_RESULT.txt，内容写 STRESS_ALL_OK
3) 宣告完成'
echo "N13B_EXIT=$?"
