# 维护期自主检测与优化报告（v1.0.2）

> 时间：2026-08-01 14:18 ~ 15:00（用户睡觉，全自主）
> 范围：window-framework + 项目记忆 + 主项目体检
> 原则：修 bug ≠ 加机制；只做既有机制的正确化/同步/保护

---

## 一、检测发现的问题（全部已处理）

| # | 发现 | 严重度 | 处置 |
|---|---|---|---|
| 1 | MEMORY.md 16.9KB 严重超限（注入被截断） | 🔴 | 压缩至 6.6KB（保留全部核心硬事实） |
| 2 | `VERSION = "0.1.0"` 从未更新——`codex --help` 显示错误版本 | 🟡 | 修正为 1.0.2 |
| 3 | **provider 字段是摆设**：base_url/key/model 全 hardcode deepseek，LLM 产出 `provider: openai/anthropic` 时语义错误（MEMORY 已知缺口） | 🟡 | 落地 provider 路由（见下） |
| 4 | dogfooding 修复的 12 个 bug **无回归测试保护** | 🔴 | 新增 test_v10.py（12 项） |
| 5 | README 停在 139/139（实际 172）| 🟡 | 同步 172/172 + 条款表补 v0.9/v1.0.1/v1.0.2 |
| 6 | CHANGELOG 缺 v1.0.1/v1.0.2 节 | 🟡 | 补节 |
| 7 | **version-iteration-manual.md 滞后**：§二 250 行误写"窗口群框架未工程化"（MEMORY 记录） | 🟡 | 修复 + 追加版本详节 + §四基线刷新 |
| 8 | workflow 引擎注释 U+FFFD 乱码残留 | 🔵 | 修复 |

## 二、优化交付

### 1. provider 路由（v1.0.2 核心）
```python
PROVIDERS = {
    "deepseek": {base_url, env_key: DEEPSEEK_API_KEY, model: deepseek-v4-flash},
    "zhipu":    {base_url: open.bigmodel.cn, env_key: ZHIPU_API_KEY, model: glm-4.5},
    "agnes":    {base_url: agnes-ai.cn, env_key: AGNES_API_KEY, model: agnes-2.5-flash},
    "openai":   {base_url: openai.com, env_key: OPENAI_API_KEY, model: gpt-4o-mini},
}
```
- `Agent(provider="auto")` → 从 window.toml `budget.provider` 路由
- 未知 provider（如 anthropic）回退 deepseek
- **意义**：MEMORY 预判的"距可用≈1 小 PR"已落——智谱免费 token 路径打通，下次 dogfooding 可零成本用 zhipu

### 2. 回归保护（tests/test_v10.py，12 项）
- provider 路由（zhipu/未知回退）
- 虚假 done 防护（_has_outputs 判定）
- budget 超限检测 + blocked
- read 大文件截断
- stage auto gate 脚本生成
- working 残留识别

### 3. 零成本验证
replay 模式全链路（analyze→deploy→workflow→check）本机跑通——改动后不必等真 LLM 即可发现集成断裂。

## 三、验证结果

| 项 | 结果 |
|---|---|
| 全量回归 | **172/172 全绿**（v10 12 + v09 21 + v07 19 + v06 14 + v05 28 + v04 17 + v03 19 + v02 21 + framework 21） |
| replay 全链路 | ✅ analyze→deploy→workflow→check 全通 |
| VM 主项目 | ✅ 门禁 rc=0、service 在跑、磁盘 72% |

## 四、commits（5 个）

```
cbe3b4d docs: README 补 v1.0.2 provider 路由说明（zhipu 免费 token 路径）
783db2b fix: 修复 workflow 引擎注释乱码（U+FFFD 残留）
b5624b9 docs: version-iteration-manual 滞后修复——窗口群框架工程化落地状态同步
c34f341 feat: v1.0.2 维护优化——provider 路由 + VERSION 修正 + dogfooding 回归保护
86e2557 chore: 忽略 __pycache__（上一轮）
```

## 五、未动项（诚实边界）

- 主项目 Rust 债务（/readyz 假实现、drain_nervous_alerts、Simplify 空挡）——属 G0/G1 域，
  按分工铁律顶层不写代码，记入报告待任务书
- 其他窗口的未提交文件（bench 结果、maintenance-protocol 补丁、swagger bundle）——不动
- 主项目大量未跟踪文档（gemini-test 相关，MEMORY 已标作废）——不动，建议用户后续归档

---

**结论**：窗口群框架从 v1.0.1（能跑）升级到 v1.0.2（能换脑 + 修护有保护 + 文档对齐）。
172/172 全绿，改动零回归。主项目维护状态健康，无红线。
