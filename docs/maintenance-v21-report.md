# v21 刀入鞘轮报告 —— 消债固化 + 设计决策沉淀 + 维护期启动

> 日期：2026-08-01
> 定位：锻造九轮（v12-v20）之后的第一轮"擦枪"——不装子弹，只清脏血、写下来、定规程

---

## 一、S1：消债——删死代码，清硬编码

### 1.1 删除 embedding 死代码

| 项目 | 位置 | 动作 |
|---|---|---|
| `zhipu_embed_fn()`（~44 行） | `service/main.rs:25-69` | ✅ 删除（v18 证 embedding ≤ keyword，零增量） |
| `set_embed_fn` + EMBEDDING_ENABLED 逻辑 | `main.rs:489-501` | ✅ 删除，恢复纯 `ExperienceStore::new()` |
| `reqwest.workspace = true` | `service/Cargo.toml:44` | ✅ 删除（仅 embedding 用它） |

### 1.2 清除全部硬编码 API key（比计划多发现 3 处）

**审查补充**：v21 计划只列了 embedding 里的 ZhiPu key（1 处），实际 grep 发现 **5 处硬编码 key**：

| 行号 | 变量 | 处理 |
|---|---|---|
| main.rs:225 | `ZHIPU_API_KEY` fallback | ✅ 改 `.env` 读取 |
| main.rs:259 | `DEEPSEEK_API_KEY` fallback | ✅ 改 `.env` 读取 |
| main.rs:283 | `DEEPSEEK_API_KEY` fallback | ✅ 改 `.env` 读取 |
| main.rs:298 | `ZHIPU_API_KEY` fallback | ✅ 改 `.env` 读取 |
| （embedding 内 1 处） | — | ✅ 随死代码删除 |

- 方案：`unwrap_or_default()` + `dotenvy::dotenv()`（main.rs:148 已有）→ key 全部走 `.env`
- `.gitignore` 加 `.env`
- VM 建 `/home/wutao/codex_work/.env`（真实 key，不进 git）
- **验证：grep 仓库 → 0 处硬编码 key 残留**

## 二、S2：门禁验证

| 项 | 结果 |
|---|---|
| `cargo check -p service` | ✅ PASS |
| wiring | ✅ **14/14 全绿**（11 基础 + 3 新增未受影响） |
| service 启动 | ✅ 6 provider 全部注册成功（deepseek/zhipu/gemini/agnes/deepseek-pro/zhipu-max） |
| smoke 测试 | ✅ T00-smoke PASS 42.3s（deepseek 经 .env key 正常工作） |

## 三、S3+S4：设计决策沉淀（两篇）

1. **`docs/design-decision-experience.md`**——"经验是降级通道，不常驻"
   - 证据链：v17 zhipu +20pt / v18 deepseek -5pt / v19 自适应开关 + wiring 锚定
   - 维护须知：不改回无条件注入、不再投 embedding/LLM 精炼

2. **`docs/design-decision-planner-v20.md`**——"all_done 必须要求真实 Write"
   - 证据链：v19 解剖（grep+read→Done 事件流）+ v20 修复（0/8→2/3）+ wiring 锚定
   - 维护须知：不改回无写 Done、T13/T19 回归必跑

## 四、S5：维护期操作规程

**`docs/maintenance-protocol.md`**：
- 季度全量体检（基准 90%±5% + 应力 0 panic + 回放 100% + wiring 14）
- 按需增量（新功能加基准题 / 修 bug 跑受影响题）
- 红线表 + 响应流程 + 新功能 checklist + 年度回顾
- 明确不跑：经验实验/embedding/LLM 精炼/正交矩阵

## 五、验收红线核对

| 判据 | 结果 |
|---|---|
| 🔴 硬编码 ZhiPu key 仍在任何文件 | ✅ 0 处（含 deepseek key 一并清除） |
| 🔴 `cargo check -p service` 失败 | ✅ PASS |
| 🔴 wiring 14 条断裂 | ✅ 14/14 全绿 |
| 🔴 `maintenance-protocol.md` 不存在 | ✅ 已创建 |
| 🟡 设计决策缺证据链 | ✅ 两篇均有完整数据引用 |

## 六、结论

**v21 刀入鞘轮完成——锻造九轮正式收官，维护期启动。**

- 脏血擦净：embedding 死代码删除、5 处硬编码 key 清出仓库（改 .env）
- 教训固化：两篇设计决策文档把"为什么"写死了，wiring 断言强制执行
- 规程发布：季度体检/按需增量/红线/响应流程一条龙
- 系统状态：wiring 14/14、smoke PASS、provider 全注册——**干净可入鞘**
