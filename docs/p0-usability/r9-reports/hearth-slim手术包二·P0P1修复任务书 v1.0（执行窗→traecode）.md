# hearth-slim 手术包二·P0/P1 修复任务书 v1.0（执行窗→traecode）

- **日期**：2026-09-11 03:30　**依据**：八卡真机验收报告 9033ac5（PC-1..4）+ 用户拍板（S9 降级链 de-scope、模型切 agnes-3.0-flash）
- **性质**：修复单（非新卡）——三处修复 + 一处 de-scope 登记，trae 开箱即干

---

## 修复 1（P0）· S10 流式 tool_calls 参数解析回归

**现象（实测锚点）**：流式模式下 `⚙ web_search →`（arguments 空）→ `missing 'query' argument` → `tool not found:`（工具名空）→ 消息历史出现 `messages[N]: missing field tool_call_id` 上游 400。**非流式路径正常**（对照：9-10 非流式时代魂斗罗/L1-01 工具全通）。

**定位线索**：
1. SSE delta 拼装：`tool_calls[]` 的 `index` 分片、`function.name`（首片）与 `function.arguments`（增量 concat）——大概率 arguments 片段未拼接或覆盖式赋值导致丢失；
2. 工具结果消息回填时 `tool_call_id` 配对（assistant tool_calls 与 tool 结果一一对应）——参数丢失会级联为配对缺失 → 400；
3. 建议：拿非流式解析路径做对照实现（同函数双路径），差异一目了然。

**验收（先红后绿）**：①单测：mock SSE 流（tool_calls 分片 delta）→ 拼装出的 arguments 与完整 JSON 一致；②真机：流式跑含工具任务（写文件+web_search）工具调用成功、无 400；③非流式对照不回退。

## 修复 2（P0）· S8 resume 不续跑

**现象（实测锚点）**：`hearth resume <id>` 读到断点（paused 状态+原目标），但 steps=0、"系统未启动"、输出总结即退出——**不恢复上下文、不继续执行**。`.hearth/runs/` 目录不存在（S8 卡要求 runs/<run-id>.json 落盘）。

**要求**：
1. 断点文件完整落盘（对话历史+scratch+产物清单+steps/预算位）至 `runs/<run-id>.json`；
2. resume = 重建完整上下文 → **重新进入消息循环继续执行**（不是报状态）；
3. `hearth resume <id> [--budget N]` 支持预算追加（当前 `--budget` 报 unexpected argument）。

**验收（先红后绿）**：真机——任务跑到一半 `kill -9`（Windows: taskkill /F）→ `hearth resume <id>` → **从断点续跑到交付**（产物完整、历史连续、steps 接着数）；日志取证。

## 修复 3（P1）· S7 错误分类不一致

**现象（实测锚点）**：agnes 不可达（连接拒绝 os error 10061 / 上游 502）→ `provider unrecoverable error` 直接暂停零重试；gemini 连接失败 → transient 长退避（正确路径，30s/1m 实测）。

**要求**：**连接拒绝/读超时/502/5xx 统一 transient**（走长退避）；仅 401/403/400/模型不存在 = fatal。验收：不可达 endpoint 三类（拒绝/超时/502）均出 `[retry]` 且退避 30s 起（先红后绿）。

## De-scope 登记 · S9 provider 降级链（用户拍板暂不做）

- zhipu 未注册问题随卡关闭；**已工作的部分保留**（`[fallback]` 切换+通道标注投影，实测工作——代码不删）；
- 配置面：`providers` 数组维持现状（默认单通道，不影响主路径）；
- 重新启用条件：另行拍板。

## 口径断点登记（模型层切换）

- **agnes-2.5-flash → agnes-3.0-flash（用户拍板 2026-09-11）**：换模型=换模型层，**此前全部基准数据（0.2.25-base/0.2.26 术前/slim 复测）与新数据不可比**——凡 3.0-flash 跑出的数据必须标注"3.0"，禁与 2.5 时代数据混标对照；
- 本机 config 已切换并实测（1 步 1.2s 直答 ✓）；.131/.133 config 待 VM 可达时同步。

## 施工顺序与纪律

```
修复1（S10，P0）→ 修复2（S8，P0）→ 修复3（S7，P1）→ 执行窗复验（原验收日志对照）
```

- 每修复独立 commit；先红后绿；红线沿用（沙箱语义零触碰/tally 原位）；
- 复验通过后：S12 判别实验（魂斗罗质量版重跑→用户评分）+ S13 真机搜索取证 + S11 目测（Linux）+ 吴涛 3 真实任务。
