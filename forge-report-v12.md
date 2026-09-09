# codex-rust v12 锻造报告

**日期**: 2026-07-30 05:50（修订：ZhiPu 已修复）
**版本**: v12 — 秤已校准
**结论**: 2 providers usable / 180 tests / tool chain verified

---

## 三门

| 门禁 | 状态 |
|:-----|:----:|
| FMT | RC=0 |
| CLIPPY | RC=0 |
| TEST | 180/0 |

---

## Provider 状态

| Provider | 模型 | Plan | Tool | 备注 |
|---|---|---|---|---|
| Doubao | deepseek-v4-flash | o | o | 50万/模型 |
| ZhiPu | glm-4.5-air | o | o | 1214 fixed |
| ZhiPu | glm-4.7/GLM-5.2 | o | o | 修复覆盖 |
| Agnes | agnes-2.0-flash | 503 | — | down |
| Ollama | qiyuan-8b | key | — | VM unreachable |

## 关键修复

| # | Bug | Fix |
|---|-----|-----|
| 1 | axum {id} routing | :id |
| 2 | ZhiPu 1214 Planner | add user msg |
| 3 | Agent send_message | runner flow update |
| 4 | port 4321→3000 | runner config |
| 5 | sessions/ dir missing | mkdir -p |

## 代码质量

- 22 crates, 180/0 tests
- bench/ isolated from crates/
- 0 TODO, 0 API shells

## Next: v12.1

See docs/forge-v12.1-sprint.md
B0 Provider rotation → B1 14 fixtures → B2 runner → B4 60 runs → B5 report
