#!/bin/bash
# P0-ATTRIBUTION Node 02 矩阵 v2：resume 链驱动（REPL stdin 驱动不可行——
# 管道 EOF 忙等 + PTY 下输入消费竞争，本身登记为新缺陷：REPL 不可程序化驱动）
# 会话连续性 = chat 建 session + resume 逐条注入（污染经持久 state 累积 ✓）
# 偏差声明：REPL 连续内存会话 → resume 恢复态（载体同为持久 state，已记录）
set -u
export HEARTH_DEBUG_PLANNER_INPUT=1
export HEARTH_ALLOW_NO_CGROUP=1
unset HEARTH_URL
cd "$HOME"

PREFIX=(
"现在怎么样啊，身体通不通"
"木有特定的检查项，你自己先自检一下，看看"
"不用检查代码库，就是看你能不能做哪些事，住户体验，你住在hearth这个harness上"
"2026-08-31T05:00:49.591654Z ERROR agent_core::r#loop: do_plan_inner failed error=stalled: 2 consecutive replans produced identical TaskGraph (4 nodes) — semantic progress absent, giving up early (Tier3 T4)"
"你的完整自测计划是什么，怎么设计的"
"那你继续跑吧"
"继续跑吧"
"现在是否偏离了，我说继续跑吧，但是你目前输出的内容是：**T12 观察第 4 轮（新数据）**：✓ Done (14 steps)"
"那你继续吧"
"上一轮放弃了，✗ Done (5 steps) 这个原因是什么"
"你的本轮回复是，✓ Done (21 steps)。就这个✓ Done (21 steps)。但是我不知道你发生了什么，我需要猜你的结果，这个是不是一个问题。"
"你登记到台账吧。"
"现在还有什么可以测试的吗"
)
FILLER=(
"什么是 Rust 的所有权机制？"
"用一句话解释什么是幂等性。"
"HTTP 409 状态码代表什么？"
"什么是事件循环？"
"简述 TCP 三次握手。"
"什么是线性一致性和顺序一致性？"
"JSON 和 YAML 各自的优缺点？"
"什么是死锁？举一个例子。"
"数据库索引的原理是什么？"
"什么是垃圾回收中的分代假设？"
"解释一下什么是乐观锁。"
"Redis 和 Memcached 的区别是什么？"
)

run_session() {  # $1=tag $2=prefix-file $3=request
  # v3 修正：①write_prefix 首行曾写入路径字符串（goal=路径→沙箱拒绝）；
  #   ②resume 不接受 --budget（CLI 直接报错，12 轮全部瞬失败）→ 移除。
  local tag="$1" pfile="$2" req="$3"
  local log="/home/wutao/fa/p0_$tag.log"
  cd "$HOME"
  local first
  first=$(head -1 "$pfile")
  timeout 600 hearth chat "$first" > "$log" 2>&1
  local uuid
  uuid=$(grep -aoE "reports/[0-9a-f-]{36}/" "$log" | head -1 | cut -d/ -f2)
  echo "SESSION=$uuid" >> "$log"
  local rest
  rest=$(tail -n +2 "$pfile")
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    timeout 600 hearth resume "$uuid" "$line" >> "$log" 2>&1
  done <<< "$rest"
  timeout 600 hearth resume "$uuid" "$req" >> "$log" 2>&1
  echo "P0_${tag}_EXIT=$?" >> "$log"
}

write_prefix() { local dest="$1"; shift; printf '%s\n' "$@" > "$dest"; }

B=/tmp/p0_prefix_b.txt
A=/tmp/p0_prefix_a.txt
write_prefix $B "${PREFIX[@]}"
write_prefix $A "${FILLER[@]}"
CONT="继续你的提议吧"
META="当前轮出现了✗ Done (18 steps)，这个是出现了什么情况呢"

for i in 1 2 3; do run_session "b${i}c" "$B" "$CONT"; done
for i in 1 2 3; do run_session "a${i}c" "$A" "$CONT"; done
for i in 1 2 3; do run_session "b${i}m" "$B" "$META"; done
for i in 1 2 3; do run_session "a${i}m" "$A" "$META"; done
echo ALL_DONE > /home/wutao/fa/p0_all_done.flag
