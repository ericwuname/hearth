# 真机回归集（R-12 / R6 贯穿项）

> 定位：与 mock 门禁**分列**的重复真机回归。mock 门禁证明内部逻辑自洽；
> 本回归集产出**真实成功率**——G-W1（≥70%）的标尺。

## 组成

| 文件 | 内容 | 状态 |
|---|---|---|
| `corpus/blindpack-15.jsonl` | 盲测包 A 组 10 条 + B 组 5 条（`release/盲测包-正式轮-v0.2.23-10+5.md` 逐字） | 完整 |
| `corpus/corpus-c.jsonl` | C 语料（用户原话 14 句）——3 条原始入册 + 11 条自手工测试日志逐字抽取（候选清单 v0.1，语料所有者 2026-09-06 确认"全过"） | **完整 14/14**（2026-09-06 起，partial 标记移除） |
| `run-regression.sh` | 执行器：引擎身份留档（--version+md5）→ 逐条 `hearth chat`（fresh session，BP-2 达标）→ 日志束落盘 | — |

## 用法（真机 .131/.133）

```bash
# B 臂（六相基线，默认）
./run-regression.sh

# A 臂（R6-9 单循环——判别实验 A 组跑分）
./run-regression.sh --arm A

# 只跑盲测包 / 只跑 C 语料
./run-regression.sh --corpus blind
./run-regression.sh --corpus c
```

产物：`results/<时间戳>-arm<X>/`——version.txt、md5.txt、逐条 log/
elapsed/terminal 锚点、summary.csv。五维打分按语料 `judge` 字段人工/评审判读后填 `score.csv`。

## 判别实验协议（阶段 2，预注册）

R6-1 落地后重跑 G-A 语料：**G-A ≥7/10 → 框架层占比大**（砺"模型层"裁定修正）；
**仍 <7/10 → 模型层占比大**（外部主张修正）。双方已预注册认输条件——只如实出数据，不做倾向性解读。

R6-9 A/B 门控同用本回归集：A 臂（`--arm A`）盲测包 A 组跑分**显著优于** B 臂才翻默认合入；
LLM 调用次数（log 内 chat 请求数）须同步降 ≥50%。
