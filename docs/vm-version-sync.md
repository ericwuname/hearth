# VM 版本同步状态与规程（W1 交付 · 2026-08-29）

## 当前基线（W1 核实轮）

| 项 | 值 |
|---|---|
| 本地 HEAD | `08c2d5e`（v0.2.9 Final Acceptance 收官）→ 本轮后推进至 W3/W4 commit（见 git log） |
| 本地 Cargo version | 0.2.9 |
| **.131**（执行窗口 VM） | binary `/usr/local/bin/hearth` = 0.2.9（W3/W4 轮已更新）；source `~/codex_t` = 与本地 HEAD 代码同步（逐文件上传+marker 核实：`mock_tool_named`=4、`topology_block`=9） |
| **.133**（评审/测试窗口 VM） | binary `/usr/local/bin/hearth` = **0.2.8 → 0.2.9**（W1 重同步+重建部署）；source `~/codex_t` = 与本地 HEAD 同步（marker 4/9 核实） |

> ⚠ **binary 已更新 ≠ source tree 已同步**——两者独立核实（marker grep + Cargo version + binary --version 三查）。本轮前 .133 即"binary 0.2.8 + source 0.2.8"双双落后。

## source 路径约定

- 两 VM 统一使用 **`~/codex_t`**（执行/测试专用树）。
- **`~/codex` 是其他窗口维护的资源——任何窗口不得写入/覆盖**（门禁串台事故的根因，见 `docs/incidents/` 与Closure Window 记录）。
- `~/codex_t` 无 `.git`（tar 同步），provenance 靠 marker grep + Cargo version 三查。

## 同步规程（发版/同步标准动作）

```bash
# 本地打包（四排除——保 VM 35G 增量编译缓存，全量覆盖=缓存重来，历史血泪）
tar --force-local \
  --exclude='target' --exclude='.workbuddy' --exclude='.git' --exclude='release' \
  -czf codex_sync.tgz .

# 上传后目标机（两 VM 同动作）
find ~/codex_t -maxdepth 1 -mindepth 1 ! -name target -exec rm -rf {} +
tar -xzf codex_sync.tgz -C ~/codex_t
find ~/codex_t/crates -name '*.rs' -exec touch {} +   # 触发增量重编

# 构建部署（binary 更新）
cd ~/codex_t && cargo build --release -p codex-cli
echo <密码> | sudo -S cp ./target/release/hearth /usr/local/bin/hearth

# 三查验证
grep "^version" ~/codex_t/Cargo.toml | head -1     # source 版本
grep -c <当版 marker> <相应文件>                     # 源码特征
/usr/local/bin/hearth --version                     # binary 版本
```

## 同步纪律（W1 批示 + 历史教训）

1. **二进制版本 ≠ 源码版本**：用户/评审看到旧版本的报修，先做三查再动手术（PATH 分叉教训：`~/.cargo/bin` 旧副本、shell hash 缓存都会伪装版本）。
2. **tar 四排除必须带全**：`target/`（35G 缓存）、`.git/`、`.workbuddy/`、`release/`。
3. **touch 全量 .rs**：tar 保留 mtime，不 touch 则增量编译可能漏编。
4. **门禁以正确 VM + 正确树的实际 log 为准**：`~/run_gate_r2c.sh`（目标 `~/codex_t`、日志 `~/t_gate_r2c.log`）——禁止使用操作他窗口资源的脚本。
5. 大文件传输后**核对字节数**（0 字节静默失败发生过：v0.2.9 tarball）。

## 变更记录

| 日期 | 动作 |
|---|---|
| 2026-08-29（W1） | .133 源码 0.2.8→HEAD 同步 + binary 重建部署 0.2.9；.131 复核（已是 0.2.9）；本文件建立 |

---

## ⚠ 分窗登记（2026-08-30 修订 · P1-CONSOLIDATION-01 终验守门员裁决 1——替代上方旧口径）

旧口径"两 VM 统一 ~/codex_t / ~/codex 任何窗口不得写入"系 .133 侧规则被**错误泛化**。实测现实（07:34 守门员核验）：

```text
.131（执行 VM）：施工树 = ~/codex（执行窗口自有——0.2.12 二进制即从此构建）
                  ~/codex_t = 0.2.9 陈旧树 → 标 DEPRECATED 防误用（处置去留由用户拍板）
.133（评审 VM）：验证树 = ~/codex_t（现行约定正确）
                  ~/codex = 其他窗口资源，零触碰（run_gate.sh 串台事故根因）
```

**分窗登记规则（写死，后续所有 provenance 表按此口径）**：

| host | 施工/验证树 | 禁写区 |
|---|---|---|
| .131（执行） | `~/codex` | ——（评审/测试窗口禁写 ~/codex） |
| .133（评审） | `~/codex_t` | `~/codex` 零触碰 |

- provenance 表述口径：**系统 PATH**（`/usr/local/bin/hearth --version`）为 binary 权威；source 树按上表分窗登记（"源码树内二进制不算数"）。
- 同步动作分窗：`.131` 同步目标 = `~/codex`；`.133` 同步目标 = `~/codex_t`（tar 四排除规程不变）。
- .131 的 `~/codex_t`（0.2.9）：标 **DEPRECATED**——不擅自删除，去留由用户拍板。
