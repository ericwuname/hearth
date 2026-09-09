#!/bin/bash
# P2 Node 08: REPL multi-turn + compaction + "继续" continuity test
cd /tmp/p2_n08
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
export HEARTH_COMPACT_CHAR_THRESHOLD=800
printf '依次创建三个文件：用 write_file 创建 f1.txt 内容 A1；再用 write_file 创建 f2.txt 内容 B2；最后用 write_file 创建 f3.txt 内容 C3。三个都创建完后宣告完成。\n继续\n' | timeout 420 hearth repl
echo "N08_EXIT=$?"
