# CHANGELOG v21.0 — 刀入鞘轮

锻造九轮收官 + 维护期启动。不装子弹，只擦枪。

## Removed（消债）
- **embedding 死代码**：`zhipu_embed_fn()`（44 行）+ `set_embed_fn` 接线 + `EMBEDDING_ENABLED` 开关（v18 证 embedding ≤ keyword，零增量）
- **reqwest 依赖**：`service/Cargo.toml`（仅 embedding 使用）
- **全部硬编码 API key（5 处）**：ZhiPu×3 + deepseek×2 → 改 `.env` 读取（`unwrap_or_default()` + dotenvy）。**仓库内密钥归零**。

## Added（沉淀 + 规程）
- `docs/design-decision-experience.md`——经验=降级通道（v17 +20pt / v18 -5pt / v19 自适应开关完整证据链）
- `docs/design-decision-planner-v20.md`——all_done 必须要求真实 Write（v19 解剖 + v20 修复 + wiring 锚定）
- `docs/maintenance-protocol.md`——季度体检 / 按需增量 / 红线 / 响应流程 / 新功能 checklist / 年度回顾
- `docs/maintenance-v21-report.md`——本轮报告
- `.gitignore` + `.env`（防止密钥入仓）

## Changed
- service 启动改为从 `.env` 读 API key（VM：`/home/wutao/codex_work/.env`）
- provider 注册 fallback key 移除（无 env 时注册空 key，调用 401——需运维保证 .env 存在）

## Verified
- `cargo check -p service` PASS
- **wiring 14/14 全绿**（11 基础 + 3 新增不受影响）
- 6 provider 注册成功，smoke T00 PASS
- 仓库 grep：0 处硬编码 key

## 维护期启动声明（v21.0 起）
- 季度全量体检：deepseek 20×2（90%±5%）+ 应力 0 panic + 回放 100% + wiring 14
- 按需增量：新功能加基准题、修 bug 跑受影响题
- 明确不跑：经验实验 / embedding 对比 / LLM 精炼 / 正交矩阵

## Known Debt
1. T19 工具链盲区（NO_GENERIC_FN）——需工具链升级，已记录在能力边界白皮书
2. VM `.env` 是运维依赖（丢失则 provider 401）——维护规程已记录
3. 剩余 API 余额：deepseek ~¥7（备用）
