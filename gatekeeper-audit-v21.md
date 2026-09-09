# 守门员审计 v21（刀入鞘轮）

- 审计时间：2026-08-01
- 审计对象：v21 刀入鞘轮交付物（消债 + 设计决策 + 维护规程）
- 方法：源码 grep 核对接线 + VM 门禁实测

---

## 一、接线核对（源码为准）

| # | 声称 | 核对方式 | 结果 |
|---|---|---|---|
| 1 | embedding 死代码已删 | `grep service/main.rs` → `zhipu_embed_fn` 0 命中 | ✅ |
| 2 | reqwest 依赖已删 | `grep service/Cargo.toml` → 0 命中 | ✅ |
| 3 | 硬编码 key 清零 | `grep -r "KEY_PATTERNS" crates/` → 0 命中 | ✅ |
| 4 | key 走 .env | `grep main.rs` → `unwrap_or_default()` ×4 + dotenvy | ✅ |
| 5 | .gitignore 含 .env | `grep .gitignore` → `.env` | ✅ |
| 6 | wiring 14/14 | VM 实测 | ✅ |
| 7 | smoke 真实 | T00-smoke PASS 42.3s（deepseek 经 .env） | ✅ |
| 8 | 三份新文档存在 | maintenance-protocol / design-decision-experience / design-decision-planner-v20 | ✅ |

## 二、数据判定

- **硬编码 key**：仓库 0 处（v21 前 5 处：ZhiPu×3 + deepseek×2）✅
- **死代码**：embedding 整块（44 行函数 + set_embed_fn 接线 + EMBEDDING_ENABLED + reqwest 依赖）✅
- **门禁**：cargo check PASS / wiring 14/14 / 6 provider 注册 / smoke PASS ✅

## 三、缺陷登记

### D1：计划遗漏的 4 处硬编码 key（审查补充）
- v21 计划只列了 embedding 内 1 处 ZhiPu key
- 实际 grep 发现 provider 注册还有 4 处 fallback（ZhiPu×2 + deepseek×2）
- 按"密钥决不入仓"原则全部清理 → .env（比计划更彻底）

### D2：VM .env 依赖（新引入的运维约束）
- service 启动依赖 `/home/wutao/codex_work/.env` 存在且含 DEEPSEEK_API_KEY 等
- 若 .env 丢失 → provider 用空 key 注册 → 调用 401
- 缓解：维护规程 §6 已记录"新增 key 同步写 VM .env"

### D3：dotenvy 只读 CWD 的 .env
- `dotenvy::dotenv()` 从当前工作目录读取（VM 上 service 从 ~/codex_work 启动 → 正确）
- 若未来换启动目录需同步 .env 位置

## 四、验收结论

**✅ v21 刀入鞘轮通过守门员验收。**

- 4 条 🔴 红线全部通过（key 清零 / check PASS / wiring 14/14 / 规程存在）✅
- 1 条 🟡 通过（设计决策均含完整证据链）✅
- 附加：比计划多清 4 处硬编码 key，运维约束已文档化
