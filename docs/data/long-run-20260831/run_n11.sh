#!/bin/bash
# P2-LR Node 11: QA 15+ turns (REPL, 逐轮记录 give_up/GoalMutation/duplicate)
cd /tmp/lr_n11
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
printf '什么是 Rust 的所有权？\n那借用检查器是做什么的？\nGo 和 Rust 的主要区别是什么？\n生命周期标注是什么？\n你刚才说的生命周期，能举个例子吗？\ntrait object 是什么？\n什么是 Send 和 Sync？\nBox、Rc、Arc 有什么区别？\n为什么 Rc 不能跨线程？\n什么是 unsafe Rust？\n继续\n查看状态\nCargo.toml 和 Cargo.lock 有什么区别？\n为什么需要 Cargo.lock？\n什么是零成本抽象？\n总结一下我们聊过的所有主题。\n' | timeout 900 hearth repl
echo "N11_EXIT=$?"
