#!/bin/bash
# CFR Node 06: C-probe 防污染重设计——事实只存在于被压缩轮次
# （goal 文本不含事实；事实经 seed.txt→secret2.txt 命令链进入早期 tool result；
#  阈值 1500 + 6 次 seq 轰炸 → 早期轮全被压缩；提问轮禁止读文件）
cd /tmp/cfr_n06
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_COMPACT_CHAR_THRESHOLD=1500
printf '按顺序执行以下命令，每条单独一次 bash 调用，不要读取任何文件内容：
1) echo VAULT-3341 > seed.txt
2) mkdir -p vault
3) cp seed.txt vault/secret2.txt
4) seq 1 700
5) seq 800 1500
6) seq 1600 2300
7) seq 2400 3100
8) seq 3200 3900
9) seq 4000 4700
全部执行完后宣告完成。\n现在回答问题：vault/secret2.txt 的内容是什么？禁止读取文件、禁止 cat、禁止重新执行 echo——只能凭你检索到的历史记录回答，直接给出内容字符串。\n' | timeout 420 hearth repl
echo "N06_EXIT=$?"
