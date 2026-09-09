#!/bin/bash
# CFR Node 06 v2: C-probe 防污染（chat 任务 + resume 提问两阶段；goal 不含事实字符串）
cd /tmp/cfr_n06
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_COMPACT_CHAR_THRESHOLD=1500
export HEARTH_TASK_TIMEOUT_SECS=240
hearth chat '按顺序执行以下命令，每条单独一次 bash 调用，不要读取或显示任何文件内容：
1) echo VAULT-3341 > seed.txt
2) mkdir -p vault
3) cp seed.txt vault/secret2.txt
4) seq 1 700
5) seq 800 1500
6) seq 1600 2300
7) seq 2400 3100
8) seq 3200 3900
9) seq 4000 4700
全部执行完后宣告完成。' \
  --budget 40
echo "N06A_EXIT=$?"
SESSION=$(ls /tmp/cfr_n06/.hearth/reports/ | head -1)
echo "RESUMING_SESSION=$SESSION"
export HEARTH_TASK_TIMEOUT_SECS=180
hearth resume "$SESSION" '回答问题：vault/secret2.txt 的内容是什么？禁止读取文件、禁止 cat、禁止重新执行 echo——只能凭你检索到的历史记录回答，直接给出内容字符串。'
echo "N06B_EXIT=$?"
