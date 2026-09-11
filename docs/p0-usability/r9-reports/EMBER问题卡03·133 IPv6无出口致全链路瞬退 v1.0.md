# EMBER 问题卡 #3 · .133 IPv6 无出口致 hearth 全链路瞬退（2026-09-11）

- **来源**：EMBER M1 三连失败（deadline / 审批 / 瞬退）的**真根因**排查（4 小时）
- **级别**：P0 环境（跨机基础设施）+ P1 hearth 诊断能力
- **结论**：**不是 hearth 能力问题、不是 provider 问题**——是 .133 网络栈的 IPv6 陷阱 + hearth 缺乏 IPv4 回退/根因诊断

## 一、现象与排除过程（对照实验链）

| 对照 | 结果 | 推论 |
|---|---|---|
| 本机（Windows）curl Agnes | **3/3 成功**（0.7-1.2s） | Agnes 服务健康 |
| .133 curl Agnes（默认） | **3/3 成功**（0.2s） | .133 网络基本通 |
| .133 `curl -4` | **200 ✓** | IPv4 通 |
| .133 `curl -6` | **000 / 2.8ms 立即失败** | **IPv6 无出口**（只有 link-local 路由） |
| .133 DNS 解析 | **只返回 IPv6**（2409:8c44:…）+ IPv4 | AAAA 存在且优先 |
| hearth（Rust/reqwest）调用 | `error sending request` ×N → S7 长退避 | **hearth 卡在 IPv6 地址** |

**根因**：.133 IPv6 只有 link-local 路由（无全局出口），DNS 返回 AAAA；curl 有 Happy Eyeballs 回退 IPv4 故通；**hearth 的 reqwest（0.12, default-features=false, rustls-tls）连接 IPv6 即失败**，且错误被归类为 transient → S7 长退避（30s/1m/2m）× N → **1 小时+ 空转**（表面是"provider 抖动"，实为地址族问题）。

## 二、修复（三步，已验证）

1. **/etc/gai.conf** 加 `precedence ::ffff:0:0/96 100`（IPv4 优先，getaddrinfo 排序）；
2. **/etc/hosts** 钉 `111.7.87.222 api.agnes-ai.cn`（单 IPv4 解析——最强保险）；
3. **重启 hearth 进程**（旧进程的 DNS 缓存/连接池仍是 IPv6）——**关键且反直觉的一步**。

**验证**：重启后 M1 重跑 **零瞬退**（retry=0），任务正常推进（数字核验通过→写 ember.py）。

## 三、需求（喂 hearth 手术）

**PC-6 · 网络栈健壮性 + 诊断可读性（P1）**：
1. **IPv4/IPv6 回退**：reqwest 客户端启用 Happy Eyeballs（或对连接失败做地址族降级重试）——curl 能做到的，hearth 应做到；
2. **错误分类细化**：`error sending request` 应细分（DNS 无 A 记录 / 连接被拒 / 地址族不可达 / TLS 失败）——当前一律 transient → **S7 的耐心退避反而掩盖了可诊断的根因**（本次空转 1 小时的直接原因）；
3. **退避投影增强**：`[retry] … (继续失败原因: 连接 2409:8c44:… 不可达 ×5)` ——让用户/运维一眼看到是环境问题而非 provider 抖动；
4. 环境侧（.133 部署清单）：IPv6 无出口的机器必须配 gai.conf 或 hosts（写入部署文档）。

## 四、附带结论

- **M1 三连失败归因修正**：①deadline 杀（S7 退避吃预算，PC-02）②审批拒（PC-05）③瞬退（本卡）——**三层全是基础设施/机制交互问题**，hearth 自身能力（写 tools.py/核验数字/准备升级）表现良好；
- **S7 的"耐心"需要配套诊断**：否则"重试到底"会掩盖问题——与顶层"失败与根因必须显式披露"原则同向。
