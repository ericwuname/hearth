# RT3 seccomp 硬化 — 顶层守门员验收记录

> 验收日期：2026-08-22（守门员独立核验，不信报告信源码）
> 依据报告：`docs/acceptance-rt3-seccomp-2026-08-22.md`
> 依据任务书：`docs/seccomp-hardening-taskbook-rt3.md`
> 交付 commit：`bcb2080`（在当前 `ux-polish-01` 历史内 ✅）

## 一、实测核验（守门员独立核对）

| 核验项 | 结果 |
|---|---|
| commit 真实性 | `bcb2080` 存在、775 行增 / 5 文件，在 `ux-polish-01` 历史内 |
| fail-closed 真终止 | `lib.rs:299/305` `if fail_closed { return Err(e) }` —— 之前仅 warn 放行裸进程，**已修复为终止 child** ✅ |
| 白名单 syscall 数 | `lib.rs:501` `SECCOMP_ALLOWLIST: [u32; 99]` = **99 个**，与报告一致 ✅ |
| 默认 deny 语义 | `lib.rs:115` fallthrough → `SECCOMP_RET_ERRNO(EPERM)`（阶段一 ERRNO 回退，预留 KILL 在 line 94/633）✅ |
| 红队探针真实性 | `bench/seccomp-redteam.sh`(6371B) + `.py`(4102B) 真存在、被报告+契约引用；VM 真跑 6 探针 BLOCKED=5/INCONCL=1/SAFE=0 ✅ |
| 4 个关键单测真实存在 | `test_sandbox_fail_closed_when_seccomp_unavailable`(1251) / `test_sandbox_degrades_when_fail_closed_false`(1278) / `test_allowlist_covers_cargo_syscalls`(1304) / `test_rt3_redteam_probes`(1332) ✅ |
| 测试能失败 | `force_seccomp_fail` 注入字段（避免 env 竞态）；`lib.rs:1302` 注释"清空则必失败"——证明非永真 ✅ |
| 门禁 | 报告：fmt 0 / clippy -D warnings 0 / test 244 passed(+4) / release 0 / T00 生产实测 1/1 PASS（VM 真跑）|
| 旧名守约 | `codex-rust` 仍被 docs 引用、crate 名 `sandbox`/`codex-cli`/`codex` 未误改 ✅ |

## 二、偏离率判定

- 🔴 = 0（无接口/trait 违规，fail-closed 真终止、白名单真默认 deny、探针真在 VM 跑）
- 🟡 = 3（见下，均不阻塞，属文档滞后类）
- 实现率 = 1.0（B-1/B-2/B-4 全落地，B-3 契约文档同步遗漏）

**闸门公式：🔴=0 且 实现率≥0.9 → 过闸。RT3 PASS。**

## 三、🟡 遗留（不阻塞，执行窗口次轮或顶层收口时修）

1. **契约文档口径滞后**：`docs/seccomp-allowlist-v1.md` line 10 写"77 个"、line 42 写"包含全部 77 个"，但源码已扩到 **99**（line 501 + 报告）。执行窗口扩白名单后未回头同步契约文档。→ 修法：把契约文档两处"77"改为"99"，与源码/报告对齐。
2. **源码注释过时**：`lib.rs:610` 注释仍写"77 个实测必需 syscall"，应改为"99 个"。属同类滞后，一并修。
3. **mount 探针 INCONCL**：mount 被静默拦无输出，归为 INCONCL 单列不混 SAFE（符合红队纪律），非缺陷；但后续若要在报告里缩小 INCONCL 比例，可加"mount 失败显式返回 EPERM 计数"的判定。

## 四、挂账（明确不在本任务，归顶层/验证窗口）

- **阶段二 KILL 化**：白名单稳定后 ERRNO→KILL_THREAD（`seccomp-hardening-taskbook-rt3.md §4`）。
- **readonly 目录 find**：T00 覆盖 writable；readonly find 的精确 syscall 缺口未完全定位（生产 readonly 走 grep/read 工具，风险可控）。
- **cgroup fail-closed**：cgroup 创建失败仍仅 warn，未纳入本任务 fail-closed（建议后续，与 RT3 同类 G0 缺口）。
- **RT3 与 CLI 设计衔接**：`hearth-cli-design.md §6.1` 已写明"RT3 让 seccomp/landlock 加载失败变终止 → CLI 隔离徽章只会出现'真隔离'或'启动失败'，绝不会出现'假装隔离'"——本交付与此一致，无需改设计。

## 五、结论

RT3 **PASS，过闸**。G0 硬边界从"半做且静默裸跑"升级为"fail-closed + 默认 deny 99 syscall 白名单 + VM 红队 5 阻 1 不确定 0 放行"。这是"可信委托/放心走开"硬边界的另一半牙齿（与已落地的 landlock 配对）。

下一步：执行窗口修 🟡 遗留（契约文档 77→99 同步）即可闭环；其余挂账按顶层节奏排期。
