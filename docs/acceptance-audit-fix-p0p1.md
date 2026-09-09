# v22 红队审计修复验收报告（P0/P1 全闭环）

> 日期：2026-08-02 | 基线：HEAD `1255498` → 验收后 `a3736e9`
> 依据：`docs/audit-fix-tracker-v22.md`（验收口径 V1-V7）+ `docs/audit-fix-taskbook-v22.md` + `docs/audit-findings-v22.md`
> 性质：施工角色完成 P0/P1 全 12 项修复 + 门禁全绿；P2 挂账待排期

---

## 一、P0（阻塞发布，6/6 全闭环）

| 任务 | 修复要点 | 验收证据 |
|---|---|---|
| **P0-1 审批零鉴权** | 默认 fail-closed（无 API_KEY 且无 ALLOW_NO_AUTH 拒绝启动）；ALLOW_NO_AUTH=1 只绑 `127.0.0.1`；401（缺凭据）/403（凭据错）区分（`ERR_FORBIDDEN` 新增） | routes.rs require_api_key 重写 + main.rs 启动逻辑反转；编译 + 测试过 |
| **P0-2 CLI approve 422** | decision 契约统一（`approved:bool` → `"approve"/"deny"`） | 上轮 WP-0a R3；CLI 契约测试 approve/deny 不再 422 |
| **P0-3 sessions 405** | `GET /api/v1/sessions` 路由 + `list_sessions_json`（内存活跃 + 持久化并集） | CLI 契约测试 `sessions` 实测 rc=0 |
| **P0-4 wiring 可绕过** | 匹配前**剥注释**（注释掉调用不再命中）；阈值 `>=7` → **精确 15**；spec **哈希锁**（真实指纹 `0x4868f86d2452ee27`，V3 先红后绿） | strip 单测 5 断言；xray_test 全绿 |
| **P0-5 bash 零沙箱** | 命令白名单（ls/cat/grep/find/python3/pytest/git…）+ `shlex.split`（去 shell=True）+ 路径参数沙箱校验 + 禁网 env + `WINDOW_ALLOW_RAW_BASH=1` 逃生开关 | 回归测试：越权写 `/tmp` 拒 / curl 拒 / `ls outputs` 通 |
| **P0-6 prompt 注入** | prompt 写入改 `json.dumps(ensure_ascii=False)` 完整转义 | 上轮 WT17；TOML 往返回归 |

## 二、P1（本轮内，6/6 全闭环）

| 任务 | 修复要点 | 验收证据 |
|---|---|---|
| **P1-1 会话 404** | `get_session_status` 内存 miss → 回落 `load_persisted_session`（"写了不读"修复） | 编译 + 测试过 |
| **P1-2 readyz 永绿** | memory `list_sessions` 错误上抛；None store → `Err`（503）；readyz 加**写探活**；EPIC-B 测试语义更新（无 store 不再当 healthy，测试固化弱点被纠正） | integration_test 更新 + 全绿 |
| **P1-3 启动 panic** | `startup_fatal` helper（可操作错误 + `exit(1)` 不 panic）替换 5 处启动 `.expect` | main.rs 全启动路径覆盖 |
| **P1-4 civ 三层静默** | `civ_note` 补 else（warn + fallback 落盘 `civ-fallback.jsonl`）；drain 移到**相位循环**（原仅 do_reflect）；append 失败计数 → readyz 503 | loop.rs + main.rs + routes.rs |
| **P1-5 路径旁路** | `is_relative_to` 取代 `startswith`（`outputs_evil/` 不再放行）；`_has_outputs` 要求**非空文件** | 回归测试 4 断言（含空文件不算产出） |
| **P1-6 CLI 契约测试** | 新增 `cli_contract_test.rs`：真 service（ALLOW_NO_AUTH 回环）+ CLI 12 命令，断言协议层不 5xx/422/405；CLI `get_status` 补状态码检查（404 不再静默成功） | 2 测试 12 命令全过 |

## 三、门禁汇总

| 门禁 | 结果 |
|---|---|
| cargo fmt --check | ✅ 0 |
| cargo clippy -D warnings | ✅ 0 |
| cargo test --workspace | ✅ **219 passed**（214 + R1/R2 2 + wiring strip 1 + cli_contract 2）零失败 |
| window-framework | ✅ **187/187**（v10 27 含 P0-5/P1-5 回归） |

## 四、修复过程顺带处理

1. **nervous-system 测试环境隔离**：`with_snapshot` 注入固定快照——原测试 `test_query_with_budget_warning` 依赖系统内存/磁盘状态（VM 资源紧张时 memory/disk 规则抢先，cost 规则失效 → 假红）
2. **VM 磁盘满**：docker 镜像 2GB 移除 + release 产物清理（5.6G 可用恢复）
3. **P0-4 剥离器边界**：初版连字符串一起剥导致 4 条 wiring 断言误红——修正为**只剥注释保留字符串**（wiring 断言含工具名/文件名等字符串匹配，必须保留）

## 五、挂账（P2 待排期，v23-execution-order §三）

P2-1 限流 Retry-After / P2-2 seccomp 扩展 / P2-3 gate reject / P2-5 provider 路由 compress/analyze / P2-6 快照回滚 / P2-7 边界硬化 / 红队 RT2 civ 缺失路径（P1-4 已覆盖主路径）/ RT3 / RT6 / RT9 / WT1 / N7 密钥串用复核 / CT2 测试自身验证

## 六、交付

- commit `a3736e9`（P0/P1 全 12 项 + 测试 + 门禁）
- tracker §5 已贴 R1 施工记录（状态列留守门员验收）
- 验收口径 V1-V7 全部满足（测试先红后绿：哈希锁、EPIC-B 语义更新均走 V3 流程）
