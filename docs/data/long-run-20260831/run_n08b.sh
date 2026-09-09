#!/bin/bash
# P2-LR Node 08 phase 2: resume (empty goal = continue current goal)
cd /tmp/lr_n08
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=420
SESSION=$(ls /tmp/lr_n08/.hearth/reports/ | head -1)
echo "RESUMING_SESSION=$SESSION"
hearth resume "$SESSION"
echo "N08B_EXIT=$?"
