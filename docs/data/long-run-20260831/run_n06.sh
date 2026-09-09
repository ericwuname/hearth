#!/bin/bash
# P2-LR Node 06: Archive C-level recoverability probe (deterministic判定，批-2 口径1)
cd /tmp/lr_n06
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_COMPACT_CHAR_THRESHOLD=2000
export HEARTH_TASK_TIMEOUT_SECS=300
hearth chat '严格按顺序执行：
1) 用 write_file 创建 secret.txt，内容恰好为 CODE-7788
2) 用 bash 执行以下 6 条命令（每条单独一次调用）：seq 1 700；seq 800 1500；seq 1600 2300；seq 2400 3100；seq 3200 3900；seq 4000 4700
3) 全部执行完后，回答这个问题：secret.txt 的内容是什么？
回答时直接给出内容字符串，不要重新读取文件（凭你检索到的记录作答）。' \
  --budget 60 \
  --acceptance 'file: secret.txt contains CODE-7788'
echo "N06_EXIT=$?"
