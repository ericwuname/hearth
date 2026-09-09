#!/bin/bash
# P0-ATTRIBUTION-01 Node 02：fresh/polluted 对照矩阵编排（12 会话，每条件 ≥3 跑）
# 主机 .133（盲测保真）；dump 开关全开；cwd=$HOME（盲测保真）
set -u
export HEARTH_DEBUG_PLANNER_INPUT=1
export HEARTH_ALLOW_NO_CGROUP=1
unset HEARTH_URL
unset HEARTH_TASK_TIMEOUT_SECS 2>/dev/null || true
cd "$HOME"

PREFIX='现在怎么样啊，身体通不通
木有特定的检查项，你自己先自检一下，看看
不用检查代码库，就是看你能不能做哪些事，住户体验，你住在hearth这个harness上
2026-08-31T05:00:49.591654Z ERROR agent_core::r#loop: do_plan_inner failed error=stalled: 2 consecutive replans produced identical TaskGraph (4 nodes) — semantic progress absent, giving up early (Tier3 T4)

你的完整自测计划是什么，怎么设计的
那你继续跑吧
继续跑吧
现在是否偏离了，我说继续跑吧，但是你目前输出的内容是：**T12 观察第 4 轮（新数据）**：✓ Done (14 steps)
那你继续吧
上一轮放弃了，✗ Done (5 steps) 这个原因是什么
你的本轮回复是，✓ Done (21 steps)。就这个✓ Done (21 steps)。但是我不知道你发生了什么，我需要猜你的结果，这个是不是一个问题。
你登记到台账吧。
现在还有什么可以测试的吗'

FILLER='什么是 Rust 的所有权机制？
用一句话解释什么是幂等性。
HTTP 409 状态码代表什么？
什么是事件循环？
简述 TCP 三次握手。
什么是线性一致性和顺序一致性？
JSON 和 YAML 各自的优缺点？
什么是死锁？举一个例子。
数据库索引的原理是什么？
什么是垃圾回收中的分代假设？
解释一下什么是乐观锁。
Redis 和 Memcached 的区别是什么？'

CONT='继续你的提议吧'
META='当前轮出现了✗ Done (18 steps)，这个是出现了什么情况呢'

run_session() {  # $1=tag $2=prefix $3=request
  # 盲测保真三修正：①PTY 包装（script）——管道 stdin 非 TTY 会触发
  #   DenyAllNonInteractive（首跑 12/12 failed 的根因）；②cwd=$HOME（盲测原样）；
  #   ③/quit 终止（EOF 后 REPL 忙等刷 prompt——56MB 教训）。
  local tag="$1"
  cd "$HOME"
  { printf '%s\n' "$2"; printf '%s\n' "$3"; printf '%s\n' '/quit'; sleep 100000; } | \
    timeout 3600 script -qec "hearth repl" /dev/null > "/home/wutao/fa/p0_${tag}.log" 2>&1
  echo "P0_${tag}_EXIT=$?" >> "/home/wutao/fa/p0_${tag}.log"
}

# B 侧（polluted）×3 继续型 + ×3 元诊断型
for i in 1 2 3; do run_session "b${i}c" "$PREFIX" "$CONT"; done
for i in 1 2 3; do run_session "b${i}m" "$PREFIX" "$META"; done
# A 侧（fresh）×3 继续型 + ×3 元诊断型
for i in 1 2 3; do run_session "a${i}c" "$FILLER" "$CONT"; done
for i in 1 2 3; do run_session "a${i}m" "$FILLER" "$META"; done
echo ALL_DONE > /home/wutao/fa/p0_all_done.flag
