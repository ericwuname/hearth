#!/bin/bash
# FA01 Node 14 QA 负回归：直答不得进入 failure recovery / TaskGraph 循环
cd /tmp
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_TASK_TIMEOUT_SECS=240
echo "=== Q1 问答 ==="
hearth chat '什么是所有权？' --budget 8
echo "EXIT_Q1=$?"
echo "=== Q2 任务控制 ==="
hearth chat '查看状态' --budget 8
echo "EXIT_Q2=$?"
