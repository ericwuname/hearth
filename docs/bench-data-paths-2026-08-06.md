# 基准测试数据路径与内容说明（zhipu-v24 / deepseek-v24）

> 生成：2026-08-06
> 所有数据都在 **VM（Linux）** 上，不在你 Windows 本机。
> 访问方式：`ssh wutao@192.168.220.131`（密码 123456），home = `/home/wutao`。

---

## 一、你能看的「测试数据」在哪

### A. 问题数据（测了哪些任务 / 问了什么）
路径前缀：`~/codex_work/bench/tasks/`

```
T00-smoke/            T01-read-api/         T02-change-timeout/
T03-fix-off-by-one/   T04-json-output/      T05-extract-common/
T06-fix-null-check/   T07-fix-logic-invert/ T08-rename-function/
T09-add-error-type/   T10-extract-config/   T11-read-struct/
T12-change-default/   T13-fix-index/        T14-add-serde/
T15-add-bench/        T16-split-module/     T17-fix-race/
T18-add-pagination/   T19-merge-duplicate/
```

每个任务目录结构（以 T19 为例）：
```
T19-merge-duplicate/
├── goal.txt       ← 给 agent 的问题原文（prompt）
├── meta.json      ← 难度等级(L1–L5)、超时、最大步数
├── verify.sh      ← 判分脚本（怎么算 PASS/FAIL）
└── fixture/       ← 待修改的 Rust crate 代码（agent 的作业本）
    ├── Cargo.toml
    └── src/
```

**T19 的 goal.txt（问题原文）**：
```
parse_age_u8 和 parse_count_u32 有大量重复的解析+验证逻辑。
写一个泛型函数替代两个函数中的重复代码。
硬性要求：泛型函数的名字必须是 parse_positive（验收脚本会精确检查这个函数名，用其他名字判失败）。
parse_age_u8 和 parse_count_u32 保留为对 parse_positive 的薄封装，公开签名不变。
所有原有测试必须继续通过，cargo test 全绿。
```

**T19 的 verify.sh（判分逻辑）**：
```bash
#!/bin/bash
cd "$(dirname "$0")"
grep -q "parse_positive" src/lib.rs || { echo "NO_GENERIC_FN"; exit 1; }
source ~/.cargo/env 2>/dev/null
cargo test 2>&1 | grep -q "test result: ok" && echo VERIFY_PASS || { echo TEST_FAIL; exit 1; }
```
→ 判定优先级：先查函数名 `parse_positive` 是否存在（没有就 `NO_GENERIC_FN`），再跑 `cargo test`（不过就 `TEST_FAIL`）。

### B. 回答/结果数据（答得怎么样）
路径：`~/codex_work/bench/results/raw/`

| 文件 | 大小 | 内容 |
|---|---|---|
| `zhipu-v24.jsonl` | 100 KB | 免费 zhipu，400 runs 全量 |
| `deepseek-v24.jsonl` | 40 KB | deepseek，160 runs 全量 |

每行是一条 run 的判定摘要（JSON），字段：
```json
{
  "task": "T17-fix-race",
  "run": 0,
  "success": true,
  "phase": "error",
  "steps": 15,
  "wall_s": 21.0,
  "provider": "deepseek",
  "session_id": "aba7e8ba-b7a2-4bb1-badb-2a57d983f8d5",
  "level": "L3",
  "verify_tail": "VERIFY_PASS"
}
```
- `verify_tail` = **判分脚本的输出**，也就是「这轮回答被判成啥」：
  - `VERIFY_PASS` 通过
  - `TEST_FAIL` cargo test 没过
  - `NO_GENERIC_FN` / `NO_ENUM` / `NO_STRUCT` … 验收脚本精确检查项没满足
  - 空 / `running N tests …` 片段 = 部分失败时的原始输出

**DeepSeek 的 verify_tail 分布（160 runs）**：
```
129  VERIFY_PASS
  8  (test result: ok. 0 passed ...) VERIFY_PASS   ← T00-smoke 类，仅编译过但无测试
  8  NO_GENERIC_FN                                ← T19 专属失败原因
  6  TEST_FAIL
  5  (cargo test 3 passed) VERIFY_PASS: multiply function found
  2  (cargo test 3 passed) VERIFY_PASS: multiply function found  ← T00 PASS
  1  NO_ENUM
  1  (cargo test 3 passed) VERIFY_PASS: multiply function found
```

### C. 真实对话 / 代码改动（⚠️ 重要限制）
- **没有被持久化保存**。基准的 session 存在 service 进程内存里，zhipu→deepseek 重启时全部丢失。
- `~/codex_work/sessions/` 仅 **6 个残留文件（28K）**，和 560 次 run 对不上。
- `~/codex_work/crates/service/sessions/` 有 240 个文件，但**不按 1:1 对应**这 560 次基准 run（跨多次重启累积），无法可靠定位「第 X 次 run 的对话」。
- **结论**：本基准只留得住「问了什么(goal.txt) + 怎么判(verify.sh) + 判成啥(jsonl/verify_tail)」，留不住「agent 具体怎么聊、改了哪几行」。要看真实对话需重跑并开启 session 落盘（见第四节）。

---

## 二、T19 为什么双方都 0%（用现有数据就能解释）

DeepSeek 在 T19 的 8 次 run 全部失败，原因**完全一致**：
```
run=0 success=False phase=done tail=NO_GENERIC_FN
run=1 success=False phase=done tail=NO_GENERIC_FN
... (8/8 全是 NO_GENERIC_FN)
```
→ agent **从没写出名为 `parse_positive` 的泛型函数**（verify.sh 第一步 `grep -q "parse_positive"` 直接挂）。这不是 API 问题（phase=done，跑完了），是 agent **不遵守「精确函数名」硬性约束**的真实能力/指令遵循缺口。这是一个可修的点（prompt 强化 or 验收约束纳入工具反馈）。

---

## 三、怎么自己查（VM 上常用命令）
```bash
# 看某个任务的问题原文
cat ~/codex_work/bench/tasks/T19-merge-duplicate/goal.txt

# 看某个任务怎么判分
cat ~/codex_work/bench/tasks/T19-merge-duplicate/verify.sh

# 统计 deepseek 通过率
python3 -c "import json;r=[json.loads(l) for l in open('~/codex_work/bench/results/raw/deepseek-v24.jsonl')];print(sum(x['success'] for x in r),'/','len(r))"

# 看某任务所有 run 的判分原因
grep 'T19-merge-duplicate' ~/codex_work/bench/results/raw/deepseek-v24.jsonl | python3 -c "import sys,json;[print(json.loads(l)['verify_tail']) for l in sys.stdin]"
```

---

## 四、想要真实对话记录怎么办
当前 harness 不落盘 session，建议二选一：
1. **改 runner/service 让 session 落盘**（推荐，长期可审计）——每 run 把 transcript 存到 `results/transcripts/<session_id>.jsonl`。
2. **针对性重跑单个任务并开持久化**（需 LLM 预算；你已说明没钱，此项暂缓）。

> 注：本会话所有分析脚本在本地 `.workbuddy/` 下（`vm_analyze_zhipu.py`、`vm_deepseek_progress.py`、`vm_show_answers.py` 等），可复用。
