#!/bin/bash
# P2 Node 14 Task A phase 2: resume the interrupted session (empty goal)
cd /tmp/p2_n14
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=420
SESSION=$(ls /tmp/p2_n14/.hearth/reports/ | head -1)
echo "RESUMING_SESSION=$SESSION"
hearth resume "$SESSION"
echo "N14B_EXIT=$?"
