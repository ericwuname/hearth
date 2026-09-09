# 窗口群框架 v0.2 验收报告

> 验收时间：2026-08-01
> 验收对象：`window-framework/` v0.2（���口心智 + 快照 + 命令面补全）
> 计划依据：`window-framework/spec/plan-v02.md`（v0.2.1 审查补充版）

---

## 一、验收红线核对

| 红线 | 判据 | 结果 |
|---|---|---|
| 🔴 | `window start` 后 agent 不能完成"写 hello.txt"（端到端断裂） | ✅ **VM 实测通过**（见下） |
| 🔴 | 框架 4 断言退化 | ✅ v0.1 回归 21/21 |
| 🔴 | 对话数据丢失 | ✅ conversation.jsonl 5 行完整 |
| 🟡 | `window export` 不可读 | ✅ markdown 含 role/时间/内容 |

## 二、S1 agent 引擎（端到端实测，VM）

```
window start win-hello:
  state: pending → working → done
  agent 2 轮完成（LLM 调用 + write_file 工具）
  ✅ shared/outputs/hello.txt 内容 "hi"
  ✅ conversation.jsonl 5 行（system/user/assistant/tool_calls/tool）
  ✅ current_tokens 从 0 变为正数（budget 运行时追踪）
  ✅ 写路径沙箱：窗外路径 DENIED（单测覆盖）
```

## 三、S2 对话存储 + 导出

- `conversation.jsonl` JSONL 增量追加 ✅
- `window export --format markdown` → 可读（含窗口名/角色/时间/工具摘要）✅
- `window export --format json` → Message 数组 ✅
- `_read_conv` 脏行容错 ✅

## 四、S3 快照/回滚

- `window snapshot`：复制 window.toml + conversation ✅（同秒冲突自动 -2 后缀）
- `window rollback --to`：恢复快照 + 当前历史移入 `.snapshots/rolled-back/` ✅
- 快照触发规则（stop 自动/50 轮）— 命令面已备，自动触发留 v0.2.5

## 五、v0.2.1 审查补充 5 项落地

| 补充 | 落地 |
|---|---|
| ① LLM key 来源（防硬编码） | ✅ env DEEPSEEK_API_KEY，无 key 报错退出码 1 |
| ② 写路径沙箱（§8.7） | ✅ Agent._allowed_write 校验（outputs 内可写/窗外拒绝） |
| ③ window.toml prompt 字段 | ✅ create --prompt + 无 prompt 拒绝 start |
| ④ budget 双上限追踪 | ✅ current_tokens/current_cost 更新 + 超限停 |
| ⑤ S4 测试 10 项 | ✅ test_v02.py 21 项全覆盖 |

## 六、测试结果

| 套件 | 结果 |
|---|---|
| tests/test_v02.py（v0.2 新增） | ✅ **21 passed / 0 failed** |
| tests/test_framework.py（v0.1 回归，适配 v0.2 start 语义） | ✅ **21 passed / 0 failed** |
| **合计** | ✅ **42/42** |

## 七、与设计 v2.1 对齐

- §8.1 budget 运行时追踪 ✅（v0.2 新增）
- §8.3 gate 脚本化 ✅（v0.1 保留）
- §8.7 写路径沙箱 ✅（v0.2 新增）
- §5f 快照/回滚 ✅（v0.2 新增）
- §5e 对话导出 ✅（v0.2 新增，markdown+json）

## 八、遗留（v0.3 范围）

- 自动快照触发（stop/50 轮）
- 上下文压缩（§3）
- 工作流引擎（§5）
- 需求窗口 → 契约 YAML → 自动建窗（补丁2）
- 共享层冲突仲裁（§8.4）
- 模板系统

## 结论

**✅ 窗口群框架 v0.2 验收通过**——agent 引擎端到端可用（真实 LLM 完成写文件任务），
对话存储/导出/快照/回滚全部落地，42/42 测试全绿，无回归。
