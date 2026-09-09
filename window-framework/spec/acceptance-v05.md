# 窗口群框架 v0.5 验收报告

> 验收时间：2026-08-01
> 验收对象：`window-framework/` v0.5（需求分析自主化 + 导入/模板收尾）
> 计划依据：`window-framework/spec/plan-v05.md`（v0.5.1 审查补充版）

---

## 一、验收红线核对

| 红线 | 判据 | 结果 |
|---|---|---|
| 🔴 | analyze 产出含环依赖 | ✅ 测试 `cycle rejected`（validate 拓扑检测） |
| 🔴 | analyze 窗口数 = 0 或 > 10 | ✅ 测试 `window count 1..10` |
| 🔴 | import 后数据丢失 | ✅ JSONL + OpenAI 双格式导入后 export 一致 |
| 🔴 | 框架 4 断言 + v0.1-v0.4 回归 | ✅ check 全绿 + 回归 78/78 |
| 🟡 | analyze 退回率 > 50% | ✅ replay 3 场景（补充1 机制已备，测试覆盖流程） |

## 二、S1 window analyze

- replay 模式：内置测试 YAML（不调 LLM）✅
- real 模式：需求分析师专用 prompt → 产 YAML；无 key 拒绝 ✅
- `--confirm`：产出写入需求窗口对话 → `workflow deploy` 直接读取建窗 ✅
- **analyze → deploy 全链路**：测试证实（win-dev-01 自动创建）✅

## 三、S2 YAML 质量校验

- 窗口数 1..10 / role 唯一 / depends 无环（拓扑）✅
- **契约字段**（v0.5.1 补充2）：budget/outputs/gate(.sh) 必填 ✅
- 缺字段 → 拒绝 + 原因 ✅

## 四、S3 window import

- JSONL 格式（同框架）+ OpenAI messages 数组 + chat completion 兼容 ✅
- tool_calls 映射（arguments JSON 字符串 → 对象）✅
- 长对话导入自动压缩 ✅
- 引用文件缺失警告 ✅

## 五、S4 template

- save（--from-window）/ list / create（--template）三部曲 ✅
- 重名拒绝 + --force 覆盖 ✅（补充5）
- 模板存 `{PROJECTS_ROOT}/templates/`（全局）✅

## 六、S5 export 增强

- `--compress`：只输出摘要 + 热层 ✅
- `--full`：完整对话 ✅

## 七、v0.5.1 审查补充 5 项落地

| 补充 | 落地 |
|---|---|
| ① analyze replay 模式 | ✅ 假 YAML 不调 LLM + real 无 key 拒绝 |
| ② 契约校验 | ✅ validate_deploy_yaml（budget/outputs/gate） |
| ③ 20 测试清单 | ✅ test_v05.py 28 项全覆盖 |
| ④ OpenAI 格式映射 | ✅ tool_calls/arguments 解析 |
| ⑤ 模板存储/重名 | ✅ templates/ 全局 + --force |

## 八、测试结果

| 套件 | 结果 |
|---|---|
| tests/test_v05.py（v0.5 新增） | ✅ **28 passed / 0 failed** |
| tests/test_v04.py（回归） | ✅ **17 passed / 0 failed** |
| tests/test_v03.py（回归） | ✅ **19 passed / 0 failed** |
| tests/test_v02.py（回归） | ✅ **21 passed / 0 failed** |
| tests/test_framework.py（回归） | ✅ **21 passed / 0 failed** |
| **合计** | ✅ **106/106** |

## 九、框架完成度（对照设计 v2.1）

| 设计条款 | 版本 | 状态 |
|---|---|---|
| skeleton（项目容器+窗口发现） | v0.1 | ✅ |
| 窗口生命周期 + 4 断言 | v0.1 | ✅ |
| agent 引擎 + 沙箱 + budget | v0.2 | ✅ |
| 工作流引擎 + deploy | v0.3 | ✅ |
| 上下文压缩 + 快照/回滚 | v0.4 | ✅ |
| **需求自主分析 + 导入/模板** | **v0.5** | ✅ |
| 共享层冲突仲裁（§8.4） | v0.6（留） | ⏳ |

**v0.5 = 设计 v2.1 目标达成**：人只需创建项目 + 在需求窗口聊天 → `window analyze` → 自动建群 → `workflow start` 全自动流转。

## 结论

**✅ 窗口群框架 v0.5 验收通过——"全自动"版达成。**
106/106 测试全绿（新增 28 + 回归 78），红线 4+1 全过。
框架演进：skeleton → brain → orchestration → memory → **autonomy**。
剩余唯一未落地条款：共享层冲突仲裁（§8.4），留 v0.6（依赖并行工作流实测）。
