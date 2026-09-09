#!/bin/bash
# CFR Node 11: QA 15+ turns（"继续"两形态 + 情绪+任务 + 状态）
# 场景设计：轮 1-5 真任务（进行中形态的"继续"）；轮 6 完成后"继续"（已完成形态 RC47 家族）；
# 轮 7-14 QA/情绪/自我指涉/修正；轮 15 总结
cd /tmp/cfr_n11
unset HEARTH_URL
export HEARTH_ALLOW_NO_CGROUP=1
printf '我有点烦，帮我创建一个 notes/ 目录并写入 notes/todo.txt 内容 "buy milk"，做完告诉我。——另外别管我的情绪，说正经的，目录名必须是 notes。\n继续\n看看状态\nnote/todo.txt 和 notes/todo.txt 有什么区别？\n创建 notes/done.txt 内容 "all set"\n继续\n查看状态\n什么是 markdown？\n我自己就是 hearth，你觉得我状态如何？\n不对，我刚才说错了，我是用户。继续。\n为什么 markdown 比 html 简单？\n你刚才说到 markdown 了吗？\n情绪不好也能干活吗？顺便确认 notes/todo.txt 还在。\n把 notes/done.txt 内容改成 "really all set"\n总结一下今天干了什么。\n' | timeout 900 hearth repl
echo "N11_EXIT=$?"
