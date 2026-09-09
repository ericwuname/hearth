# Node 01 — Actual chars/token Calibration（P2-MEMORY-CONTEXT-01）

日期：2026-08-30　方法：真实 workload 语料 × Agnes API authoritative usage（`response.usage.prompt_tokens`）× estimate_chars 口径复刻。测量脚本：`measure_tokens.py`（本目录，可复跑）。

## 先验复核（批-1/批-3 要求先行项）

**estimate_chars 仪器虚增实测**（`context.rs:174-181`，锚点确认）：`format!("{:?}", m.content).len()` ——两个叠加偏差：
1. **Debug 包装**：String 被 `{:?}` 包裹为 `"..."`（+2 字符 + 转义）；
2. **`.len()` = 字节非字符**：UTF-8 中文 3 字节/字——中文语料虚增远超先验的 "~2×"。

## 校准表（Agnes agnes-2.5-flash，API usage 权威）

| 语料 | naive chars | est 口径¹ | tokens(API) | chars/token | est/tokens | est 虚增² |
|---|---:|---:|---:|---:|---:|---:|
| system（constitution.md） | 1,430 | 3,562 | 1,063 | 1.35 | 3.35 | **2.49×** |
| code（loop.rs 500 行） | 14,028 | 18,508 | 5,356 | 2.62 | 3.46 | 1.32× |
| 中文对话（FA01 真机 goal+追问） | 673 | 1,036 | 560 | 1.20 | 1.85 | 1.54× |
| 英文 tool result（cargo test） | 809 | 826 | 556 | 1.46 | 1.49 | 1.02× |
| mixed（规划草案+日志+JSON） | 631 | 766 | 552 | 1.14 | 1.39 | 1.21× |
| TaskGraph/Continuity 形态 | 792 | 921 | 552 | 1.43 | 1.67 | 1.16× |
| **COMBINED（全语料拼接，最代表）** | **18,373** | **25,629** | **7,215** | **2.55** | **3.55** | **1.39×** |

¹ est 口径 = Rust `format!("{:?}", content).len()` 复刻（Debug 包装 + UTF-8 字节计数），与生产 `estimate_chars` 同口径。
² est 虚增 = est口径 / naive chars。
注：单条小语料的 tokens 含 ~500 chat template 底数（对比时以 COMBINED 为准）。

## 核心答案（Q1 前半）

> **32,000（est 口径）在 Hearth 真实 workload 下 ≈ 9,009 tokens**；若按 naive chars 理解 32,000 chars ≈ 12,566 tokens。

- est 口径 32,000 实际只代表 **22,940 个 naive 字符**的原始内容（Debug+bytes 虚增 1.39×）。
- 对照 Agnes 512K：**压缩触发点 ≈ 真实容量的 1.8%**（比砺批-1 先验的 ~3% 更激进——`.len()` 字节口径是被先验低估的新增偏差源）。
- 分语料 chars/token：中文 1.14-1.35、英文/代码 1.46-2.62——**中文 workload 的压缩点更提前**（字节虚增 ×3 + chars/token 更低，双重放大）。
- provider usage（API prompt_tokens）为唯一权威口径；本地估算（无论 estimate 口径还是 naive）只做交叉验证（批-3）。

## 对后续 Node 的输入

- **Node 02**：阈值单位 = 字节化 Debug 字符，provider-unaware 确认；触发点 ≈ 9k tokens。
- **Node 05/11**：candidate 阈值以 provider-aware 表达（窗口×比例）；char↔token 换算系数取 **2.55 naive chars/token（combined 实测）**作为本地估算修正基准（est→naive 除以 1.39）。
- **Node 10**：estimate_chars 虚增 = 第 6 处"变相截断"正式定量（combined 1.39×、中文最高 2.49×）。
