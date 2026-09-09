# Hearth CLI 修复 + 渲染美化验收报告（v0.1.1）

> 日期：2026-08-22 | 任务书：`docs/hearth-cli-fix-and-pretty-taskbook.md`（基线 a6559d9）
> 来源：用户真机测试贪吃蛇任务暴露三类问题（write_file 截断 / init 混淆 / 渲染简陋）

---

## 1. 门禁结果总览

| 门禁 | 内容 | 结果 |
|---|---|---|
| **R1** write_file 超长参数修复 | 双层容错 + 防死循环 + 根治截断源头 | ✅ 三层修复 + max_tokens 4096 + 单测 +6 |
| **R2** hearth init 交互 | rpassword 不回显 + 分隔标签 + fallback | ✅ 实测 key 写入 + 组装 OK |
| **R3** 通用门禁 | fmt/clippy/test | ✅ fmt 0 / clippy 0 / **257 passed** / build 0 |
| **R4** chat 渲染美化 | 气泡/折叠/卡片/日志分级 | ✅ 实测输出见 §3 |
| **R5** 安装链路不回退 | --version/零配置/安全徽章 | ✅ 无相关改动（纯 render + 修复） |

---

## 2. R1 修复详情（三层 + 根治）

### 根因链（用户日志 column 2350 EOF）
```
do_act max_tokens=1024 → write_file 整段 HTML 输出被截断
→ LLM 层 from_str 失败 → 静默降级 Value::String
→ edit.rs args.get("path") 失败（String 无字段）→ missing 'path'
→ 4 连撞 → budget exhausted（贪吃蛇只写出 1383 字节 index.html）
```

### 修复（对应任务书 A/B/C）
| 层 | 改动 | 位置 |
|---|---|---|
| **A 治标** | `parse_tool_args`：JSON 截断（`{` 开头缺尾 `}`）→ 补 `}` 重试；仍失败保留 raw + **醒目 WARN（含截断长度）** | llm-openai lib.rs |
| **B 治本** | `extract_str_arg`：raw String 完整 parse + **前缀提取 path/pattern**（关键字段在对象最前通常完整）；edit/read/grep/glob 四工具接入 | tools-builtin |
| **C 防死循环** | `force_param_gap`：missing/argument 类错误连续 ≥2 → 下轮 build_messages 强制注入 gap 澄清 | agent-core loop.rs |
| **根治源头** | `max_tokens 1024→4096`（write_file 输出空间） | agent-core loop.rs |

### 实测证据（VM，用户同款任务）
```
⚙ write_file → path: index.html            ← 工具折叠（Object 分支）
  ✓ wrote 11834 bytes to /tmp/snake_test5/index.html   ← 完整落盘！
📄 产物 file: index.html（+474 行）          ← 产物登记高亮
```
**对比**：修复前 1383 字节截断 + 4 连撞 missing path；修复后 **11834 字节完整（474 行）零错误**。

### 新增单测（+6）
- `parse_tool_args`：截断补 `}` 修复 / 不可修复保留 raw / 正常 ×3
- `coerce_args`：String→Object / Object 透传 / 不可 parse 保留 ×3
- ⚠️ 踩坑记录：`r#"..."#` 的 `"#` 是 raw string 结束标记——测试 content 闭合引号须单独写（`""#` = 内容 `"` + 结束 `"#`），否则截断场景构造失真（调试 30 分钟定位）。

---

## 3. R2 init 实测（三次）

```
── ① provider（模型服务商，回车=deepseek 默认）──      ← 分隔标签
provider [deepseek]:
── ② API key（不回显——粘贴后按回车）──               ← 不回显
API key（回车跳过）:
── ③ base URL（可选，回车=provider 默认）──
base URL（回车=provider 默认）: 已写入 ~/.config/hearth/config.toml
provider deepseek 组装 OK
```
- 管道实测：key 写入 config（grep -c = 1）+ 组装 OK
- 非 tty fallback：rpassword 在管道返回空 → fallback 普通读取（CI 可测；真实终端仍不回显）

---

## 4. R4 渲染美化实测（贪吃蛇输出对比）

**修复前**：纯文本行式 dump（`⚙ write_file {"path": "index.html", "content": "...` 整段 JSON 刷屏）

**修复后**：
```
> 写一个贪吃蛇的html小游戏                        ← 用户气泡（蓝）
┌─ 🗺 规划草案（steps=4 gaps=1 待问=0 自动假设=1）  ← 规划卡片
   🤖 ⚡ 假设[missing_goal_source]: ...            ← 假设醒目
⚙ glob → pattern: **/*                            ← 工具折叠（关键参数）
⚙ write_file → path: index.html                   ← content 不刷屏
  ✓ wrote 11834 bytes ...                          ← 工具结果着色
📄 产物 file: index.html（+474 行）                 ← 高亮
```
- 红线合规：render 层只读不写 BE 语义（字段含义未变，仅颜色/布局/折叠）

---

## 5. 交付清单

| 交付 | 说明 |
|---|---|
| commit `6dc8704` | 12 files，+318/-60（llm-openai/tools-builtin/agent-core/codex-cli） |
| 单测 | +6（parse_tool_args ×3 + coerce_args ×3） |
| 门禁 | fmt 0 / clippy 0 / 257 passed / build 0（VM --test-threads=2） |
| 真机复测 | 贪吃蛇同款任务 write_file 11834 字节完整落盘（474 行） |

---

## 5.5 顺手交付：REPL 直跑模式（用户问询 → 基础版评估中等偏易 → 完成）

任务书范围外排除 REPL，但用户明确问"难度如何，不难顺手做"。评估后基础版可做，已交付：

- `hearth repl` 无 `--url` → **直跑模式**：reedline 行编辑（方向键/历史/编辑）+ 多轮循环
  （读目标 → run_local 直跑内核 → 渲染 → 再读）；有 `--url` → 原远程模式保留
- `/quit` / `/exit` / `/q` 退出；空行跳过；Ctrl-C/D 退出
- **兼容兜底**：reedline 光标查询失败（受限终端/测试 pty 不响应 ESC[6n）→ fallback
  普通读取（真实终端仍行编辑）
- 实测：`hearth> ` 提示 + 🔒 徽章 + 🗺 规划渲染 + `/quit → bye 👋` 全验证
- commit `83fbf9d`；门禁 fmt/clippy 0 / 257 passed / build 0
- 明确不做：Claude Code 底部固定输入框 + 流式 repaint（终端重绘竞态易花屏，中高难度——留 v0.2）

---

## 6. 挂账

| 项 | 状态 |
|---|---|
| `hearth repl` 交互式 TUI（reedline） | 范围外（任务书明确排除；基础版可作下轮候选——用户已问询，评估：基础版中等偏易可做，Claude Code 完美重绘版中高难度） |
| service integration 并发偶发（budget 测试） | 已知挂账（--test-threads=2 全绿） |
| bash 工具在 KILL 化 + 特定命令下偶发 exit=-1 | 本轮复刻验证 sandbox 内 bash 正常（pwd&&ls exit=0）；贪吃蛇场景可跑通 write_file；若复现需 strace -f 深挖（挂账观察） |
