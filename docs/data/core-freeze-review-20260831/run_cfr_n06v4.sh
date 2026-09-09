#!/bin/bash
# CFR Node 06 v4: 终极防污染 C-probe
# 事实链：/tmp/vault_fact.txt（预置）→ cat 的 tool result（早期轮 → 压缩 → archive）
# → 第 8 步删除事实文件（re-read 不可行）→ resume 提问（用户消息不含事实串）
# 判定：回答含 VAULT-3341 + 零 cat/read 文件 + 事实文件已删 → C 证明（唯一载体=archive）
cd /tmp/cfr_n06v4
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_COMPACT_CHAR_THRESHOLD=1500
export HEARTH_TASK_TIMEOUT_SECS=240
printf 'VAULT-3341' > /tmp/cfr_vault_fact.txt
hearth chat '按顺序执行以下命令，每条单独一次 bash 调用：
1) cat /tmp/vault_fact.txt
2) mkdir -p vault
3) seq 1 700
4) seq 800 1500
5) seq 1600 2300
6) seq 2400 3100
7) seq 3200 3900
8) rm /tmp/vault_fact.txt
9) seq 4000 4700
全部执行完后宣告完成。不要读取 vault/ 下任何文件。' \
  --budget 40
echo "N06A_EXIT=$?"
SESSION=$(ls /tmp/cfr_n06v4/.hearth/reports/ | head -1)
echo "RESUMING_SESSION=$SESSION"
hearth resume "$SESSION" '回答问题：我刚才让你在第 1 步记住的字符串是什么？禁止读取任何文件、禁止 cat——只凭你检索到的历史记录回答，直接给出该字符串。'
echo "N06B_EXIT=$?"
echo "FACT_FILE_EXISTS=$(test -f /tmp/vault_fact.txt && echo yes || echo no)"
