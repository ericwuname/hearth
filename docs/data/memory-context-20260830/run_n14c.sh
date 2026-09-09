#!/bin/bash
# P2 Node 14 Task B: QA 10+ turns via REPL — must stay QA, never enter TaskGraph/recovery
cd /tmp/p2_n14b
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
printf '什么是所有权？\n什么是借用检查器？\nRust 和 Go 的主要区别是什么？\n什么是生命周期标注？\n解释一下 trait object。\n什么是 Send 和 Sync？\nBox、Rc、Arc 的区别？\n什么是 unsafe Rust？\nCargo.toml 和 Cargo.lock 的区别？\n什么是零成本抽象？\n总结一下上面聊过的所有主题。\n' | timeout 600 hearth repl
echo "N14B_EXIT=$?"
