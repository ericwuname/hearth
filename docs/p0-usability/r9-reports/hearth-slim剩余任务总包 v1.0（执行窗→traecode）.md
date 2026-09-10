# hearth-slim 剩余任务总包 v1.0（执行窗拟→traecode 执行）

- **日期**：2026-09-11 04:30　**依据**：修复复验报告 ac7517c + 顶层签发的第二批挂账（dfa0c48）+ 用户指令"把任务书写出来发给 trae"
- **基线**：`p0-usability-01` HEAD（ac7517c 后）
- **性质**：修复 1 项（P0）+ 四卡（S15-S18）+ 收尾修正 2 项。每项独立 commit、先红后绿、执行窗验收。
- **红线沿用**：沙箱语义零触碰 / tally 原位 / 无凭据入库 / 每项前 git archive 备份

---

# 第一部分 · 修复 4（P0，最先做）

## 修复 4 · 3.0-flash 输出预算适配（卡已立：a4ba695）

**病灶实测**：`max_tokens` 硬编码 8192×3（`loop.rs:2855`、`loop.rs:2994`、`planner/src/lib.rs:387`），无 env 覆盖。**复验实测复现**：agnes-3.0-flash 重推理任务（归纳法推导）正文残缺（总结块实录「生成失败：模型输出残缺」——thinking 吃掉预算）。

**动作**：
1. env 化 `HEARTH_MAX_TOKENS`：默认 **65536**（3.0-flash 约束）；三处硬编码替换为统一取参函数（默认值+env 覆盖+合理上限保护）；
2. 文档/config 注释同步。

**过关（先红后绿）**：①单测：未设 env→65536；env=8192→8192；非法值→默认+警告；②**真机：3.0-flash 归纳法推导任务正文完整**（修复前残缺——红测试须复现该形态）；③token 口径标注（本卡前后 completion tokens 记录对比）。

---

# 第二部分 · 第二批四卡（S15-S18，顶层挂账）

## S15 · bash 状态持久化（cwd/env 跨调用）

**病灶**：bash 工具每次调用独立（无 cwd 持久）——多步构建/长任务每次重敲路径（bash.rs 实测无持久逻辑）；claude code 的 bash 是持久的。
**动作**：bash 会话内 cwd 持久（相对上一调用的 cwd 解析 cd）；关键 env 可选持久（白名单制，防污染）；投影显示当前 cwd（`⚙ bash → [cwd:/x] cmd`）。
**过关**：真机多步任务——第二步 `cd sub && pwd` 后第三步 `pwd` 仍在 sub；先红后绿单测。

## S16 · todo 投影接线

**病灶**：todo.rs 工具存在但投影接线待核（S5 拆相位机后长任务自组织需要轻量可见的清单投影）。
**动作**：todo_write 状态变更投影（现有 Checklist 投影保留/升级）；任务收尾时 todo 完成度进 run report 与 S14 总结。
**过关**：真机多步任务——todo 状态变化实时投影，收尾总结含完成度；单测。

## S17 · 用量投影（/cost + /context）

**病灶**：注意力税/成本对用户不可见（每次瘦身前后对比靠人工 grep）。
**动作**：repl 新命令 `/cost`（本会话累计 tokens ↑↓+calls+按通道分账）与 `/context`（当前上下文占用：history_chars/system_chars/预算位）；收尾投影已有 tokens 行保留。
**过关**：真机 repl `/cost` `/context` 输出正确（对照 run report 数据）；单测。

## S18 · headless 单轮（hearth -p）

**病灶**：无 `-p` 式单轮调用（EMBER M0 产出的 CLI 形态可直接抄回，标准化集成面）。
**动作**：`hearth -p "<提问>"`（headless 单轮：stdout 只出最终正文，进度/思考走 stderr；退出码语义：0 成功/非 0 失败）。
**过关**：真机 `hearth -p "1+1=?"` → stdout 仅答案；脚本管道可用（`$(hearth -p ...)`）；先红后绿。

---

# 第三部分 · 收尾修正（2 项）

## C-1 · project-xray 规格同步（S5 漂移修复）

**病灶**：xray 测试 `experience-adaptive-switch` 引用已退役锚点（`consecutive_errors >= 3`、`store.search`）→ test 红（真因=S5 拆相位机后 spec 未同步，非 docs 问题）。
**动作**：更新 `docs/xray/*.toml` 对应 spec 条目（删除/改写退役机制的非接线锚点）；**保持测试可失败性**（不许改判据绕过——spec 更新须与 S5 实际实现对齐）。
**过关**：`cargo test -p project-xray` 全绿且 spec 与代码现状一致（抽查 2 条）。

## C-2 · config 路径与文案一致性

**病灶**（验收实测）：①config 路径三处歧义——代码为 `%APPDATA%/hearth/config.toml`，APPDATA 为空时 fallback 到 `HOME/hearth/config.toml`（非标准位置）；②`hearth resume <错 id>` 报错文案指向 `~/.config/hearth/sessions/` 与实际不符；③Windows 控制台 GBK 编码致输出乱码（观察项，可修则修：强制 UTF-8 输出）。
**动作**：路径 fallback 规范化（APPDATA 空 → 明确报错或走标准 fallback+日志提示）；报错文案与实际路径对齐；控制台编码统一（若工程量小）。
**过关**：三处路径实测行为一致可解释；文案与实际一致；乱码消除（或明确记录为环境限制）。

---

# 施工顺序与验收

```
修复 4（P0）→ S15 → S16 → S17 → S18 → C-1 → C-2
```

- 每项：备份 → 落刀 → fmt/clippy → 先红后绿 → 独立 commit；
- 执行窗逐项复验（修复 4 复验=重推理正文完整；S15-18 真机演示；C-1 测试转绿；C-2 路径实测）；
- 全部完成后 → **S12 判别实验**（魂斗罗质量版重跑→用户评分）+ **S13 真机搜索取证** + **S11 目测（Linux）** + **吴涛 3 真实任务**（终局）。

## 呈用户/顶层

1. 本总包确认（修复 4 P0 先行；S15-S18 为顶层已签挂账，射序如上——如需按 3 任务抽查反馈调整，随时插队）；
2. 模型层口径提醒：全部验收数据标注 agnes-3.0-flash；修复 4 落地前的一切 3.0 重推理数据不作数（正文残缺风险）。
