# CHANGELOG v15.0 — 铸基轮

## Added
- **确定性回放引擎** `crates/llm-replay/`：实现 `LlmProvider` trait，按 fixture 录制内容逐轮回放，LLM 打桩/工具真跑。支持 fixture 加载、turn_index 索引、portable_paths 路径可移植化、会话回放期间同步状态。
- **天花板对照 provider**（S1b）：deepseek 通道 `v4-flash` → `v4-pro`、zhipu 通道 `glm-4.5-air` → `glm-4.7`，同网关换模型规模使唯一变量为模型能力。
- **wiring 第 9 条红线断言** `replay-provider-wired`（severity=red）：ReplayProvider 生产路径接线被回退即 CI 断裂（`docs/xray/wiring-v13.toml`）。
- **价值雏形脚本** `bench/value_pilot_v15.py`：输出 agent 侧耗时按难度分档 + 人机对照套件（3 题待填）。
- **汇总证据表** `bench/summarize_v15.py`：自动聚合 S1/S1b/S2/S2b 全部 JSONL 结果输出 `bench/results/matrix-v15.md`。

## Fixed
- **turn_index 跳轮**（v15 引入 bug）：ReplayProvider 的 `turn_index()` 原统计全部 assistant 消息，被 Reflect（纯文本音频消息）干扰 → 每真实轮 +2 → 跳过隔轮。只统计带 `ToolCalls` 非空的 assistant 消息（`crates/llm-replay/src/lib.rs`）。
- **回放绝对路径问题**：fixture 工具参数含原会话绝对路径 `/sessions/<uuid>/` → 回放写出到旧目录。新增 `portable_paths()` 正则剥离前缀。
- **VM 磁盘满致 nervous Abandon**：`/` 分区 100%（502MB 可用），nervous 系统因 `disk critical` 主动中止 agent。清理旧版本目录释放至 24GB 可用。

## Changed
- **ST7 判据拆分**（v14 债务#3）：`stress_v15.py` 将 `isolation_ok`（安全属性）与 `task_ok`（模型行为）分离，不再因模型未创建自身标记而误报隔离失败。
- **service 部署**：`bench/_vm_restart_service.py` 修复 pkill 自匹配问题、用 `setsid nohup ... & disown` 实现可靠后台启动。

## Metrics
- deepseek v15 20×2 主表 **36/40 = 90.0%**（达标红线≥90%）。
- S1b 天花板：deepseek-pro 4/6 = 66.7%（T14/T09 全部通过，T19 无差别全挂）。
- S2 确定性回放：**31/31 = 100%**，合计 68s（录制 2,354s），省 97% + 零 token。
- S3 价值雏形：40 轮合计 36.2 分钟机器时间。

## Known Debt（v16）
1. T19-merge-duplicate 所有模型全挂 → 夹具/工具链侧分析。
2. S2b 应力场 `stress_v15.py` 结果。
3. 人机对照套件人侧数据待补填。
4. service 部署依赖性：`DEEPSEEK_API_KEY` 硬编码含 fallback、智谱 API 连接偶发超时。
