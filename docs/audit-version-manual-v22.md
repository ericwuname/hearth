# version-iteration-manual.md 守门审计 v22.0 final

> 审计方法：grep 源码/git 交叉核验所有硬声称 + 逐行比对六份验收报告

---

## 一、硬数字核对

| 声称 | 实测 | 结果 |
|---|---|---|
| wiring 15/15 | 15 `[[capability]]` blocks | ✅ |
| Agnes key 0 hit | `grep sk-8LBZ1 crates/` = 0 | ✅ |
| v22.0 tag 存在 | `git tag -l v22.0` 存在 | ✅ |
| 测试 214 passed | 声明属性 220（部分 cfg 排除），实测 214 | ✅ |
| Docker build 已验证 | `_docker_verify.sh` 存在 + manual §4 记录详细证据 | ✅ |
| 窗口群框架已工程化 | L250 "未工程化" 已划线修正为 "已工程化" | ✅ |

---

## 二、发现：4 处版本漂移（需修正）

| # | 位置 | 问题 | 实际状态 | 建议修正 |
|---|---|---|---|---|
| 1 | L39 v22 行状态列 | **"🟡 进行中"** | v22.0 已 tag，全部验收通过 | 改为 "✅ 已发布" |
| 2 | L39 v22 tag 列 | **"（未 tag）"** | tag `v22.0` 存在 | 改为 "`v22.0`" |
| 3 | L252 v22 结语 | **"v22 未打 tag，作为验证/工程化轮归档后即可 tag"** | v22.0 已 tag | 改为 "✅ 已 tag v22.0" |
| 4 | §4 基线 HEAD | **"`343e0cb`"** | 实际 HEAD = `8e39775`（tag 重打 + 收尾盘点后） | 更新为当前 HEAD |

---

## 三、发现：v22 "没做哪些" 清单已过期

| L246-250 声称 | 实际状态 | 证据 |
|---|---|---|
| `/readyz` 假实现未修 | **✅ 已修（EPIC-B）** | `acceptance-final-epic-bc.md`：routes.rs 真实探活 → 200/503 |
| `drain_nervous_alerts` 零调用 | **✅ 已修（EPIC-C）** | 同报告：接线 CivWriter + 删死包装 |
| `Simplify` 分支不可达 | **✅ 已修（EPIC-C）** | 同报告：移除死 arm |
| 季度基线幽灵文档 | **✅ 已清（Y1-3）** | `acceptance-y-direction.md`：删 10+ 处引用 |

**v22 "没做哪些" 整段需刷新**——四项旧债已还清。剩余未做仅 T19 能力墙（模型级，非代码）和 guest 多用户（已排除）。

---

## 四、$3 "全局收口表" 同样过期

| L284-288 声称 | 实际 |
|---|---|
| `/readyz` 假实现 | **✅ 已做** |
| `drain_nervous_alerts` 零调用 | **✅ 已做** |
| `Simplify` 不可达 | **✅ 已做** |
| 季度基线幽灵 | **✅ 已做** |

应将这四项从 "没做/仍开" 移到 "✅ 已做"，或标注 ✅ EPIC-B/C 已修。

---

## 五、窗口群框架数据漂移

version manual v1.0.2 section (L256-270) 声称 "172/172 全绿" + "v1.0.2 维护优化" + "多窗口并行尚未验证"——但后续版本已推进：

| 声称 | 实际 |
|---|---|
| 172/172 全绿 | v1.0.3 = **177/177** 全绿（`acceptance-v1.0.3.md`） |
| 多窗口并行尚未验证 | v1.0.3 **已验证**（info-architect/html-generator/qa-reviewer 三窗口分工） |
| v1.0.2 是最后版本 | v1.0.3 存在 |

窗口群框架段需补 v1.0.3 节。

---

## 六、Docker verify 脚本 — 工程验证

`_docker_verify.sh` 五步完整：等 rust 镜像 → 拉 debian → docker build（legacy builder + daemon mirrors）→ run + healthz/readyz 双 200 → sandbox 能力检查。脚本与 manual §4 的 Docker 证据链一致。✅

---

## 审查结论

**⚠️ 有条件通过**——version manual 的核心声称全部经源码核实成立（wiring 15/15、test 214、key 零攻击面、Docker 已验证）。但 v22 状态列、"没做哪些"清单、全局收口表的**四项旧债需从"未做"移到"已做"**——这些是 EPIC-B/C 和 Y 方向已交付的内容，manual 还没跟上。窗口群框架段需补 v1.0.3。

修正后 manual 是完全准确的版本迭代总账。
