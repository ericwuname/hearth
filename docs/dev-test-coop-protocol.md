# 开发-测试窗口协作协议 v1.0

> 问题：测试窗口跑全量（应力/基准/回放）占用 VM 30-60 分钟，执行窗口此时无法编译或跑门禁。
> 目标：两个窗口在**同一台 VM** 上高效并行，不互相踩，结果自动流转。

---

## §1 核心设计：三区隔离 + 端口分时 + 自动化接力

```
VM (192.168.220.131)
│
├── ~/codex_dev/           ← 开发区（执行窗口独占）
│     port 3000             ← 开发用 service
│     源码：latest HEAD
│     target/ 预编译
│
├── ~/codex_test/          ← 测试区（测试窗口独占）
│     port 3001             ← 测试用 service（不同端口）
│     源码：v22.0 frozen worktree
│     预编译二进制复用 dev 的 target/（增量）
│
├── ~/codex_work/           ← 结果交换区（共享可读写）
│     test-findings/         ← 测试窗口写报告
│     test-queue/            ← 待跑测试队列（执行窗口投递）
│     gate-logs/             ← 门禁日志归档
│
└── .env                     ← 共享（key 只读）
```

### 为什么这样设计

| 问题 | 解法 |
|---|---|
| 两个窗口抢一个 service 端口 | dev=3000, test=3001——共存 |
| 测试把源码改了 | worktree 冻结，不 commit |
| 编译互相踩 target/ | 共享 target/ 缓存（增量编译），但各自的 Cargo.lock 独立 |
| VM 资源争抢 | 测试窗口跑基准时，执行窗口只做**不编译**的活（写文档、改 Python 脚本） |
| 测试发现需人手传递 | 结果自动写入 `~/codex_work/test-findings/`，执行窗口 `pull` 即可 |

---

## §2 操作协议：投递 → 执行 → 取回

### 执行窗口投递测试任务

```bash
# 执行窗口：我要跑什么测试
echo "WT1,WT2,RT1,RT2,RT4,CT1,CT2" > ~/codex_work/test-queue/pending.txt
# 或指定完整场景文件
cp docs/test-plan-audit-v22.md ~/codex_work/test-queue/
```

### 测试窗口领任务 + 跑

```bash
# 测试窗口登录后第一条命令
cd ~/codex_test

# 1. 检查队列
cat ~/codex_work/test-queue/pending.txt

# 2. 启动测试 service（3001 端口，不干扰 3000 的 dev service）
cd ~/codex_test
source ~/.cargo/env
# 如果 dev service 已在 3000 跑，直接 3001 启动
nohup ./target/release/service --port 3001 > ~/codex_test/service.log 2>&1 &

# 3. 跑测试
python3 bench/runner.py batch --runs 1
# 或红队测试
./run_audit_tests.sh WT1 WT2 RT1 RT2

# 4. 写结果到交换区
echo "WT1: PASS | WT2: 🔴 vuln found (详见报告)" \
  > ~/codex_work/test-findings/result-$(date +%Y%m%d-%H%M).txt
```

### 执行窗口取结果

```bash
# 执行窗口随时拉最新结果
cat ~/codex_work/test-findings/*.txt | grep "🔴\|🟡"
# 或
ls -t ~/codex_work/test-findings/ | head -5  # 最近 5 份报告
```

### 测试窗口跑完清理

```bash
# 跑完后停掉自己的 service（不影响 dev 的 3000）
fuser -k 3001/tcp
# 标记队列完成
mv ~/codex_work/test-queue/pending.txt ~/codex_work/test-queue/done-$(date +%Y%m%d-%H%M).txt
```

---

## §3 三类测试的分类执行策略

| 测试类型 | 场景 | 需要什么 | 是否阻塞开发 | 建议窗口 |
|---|---|---|---|---|
| **零 token 测试** | WT1-WT19, RT1-RT6/RT8/RT9, CT1-CT6 | Python + mock，不调 LLM | ❌ 不阻塞——随时跑 | **开发期间跑** |
| **VM 门禁** | fmt + clippy + test + wiring | cargo build + VM，3-5 分钟 | 🟡 短暂阻塞（增量编译快） | **开发者提交前自跑** |
| **烧 token 测试** | RT7 应力 (deepseek 24次) + 真 LLM dogfooding | 独占 service + LLM API | 🔴 长期占用（30-60 分钟） | **夜间批量跑** |
| **基准全量** | 20×2 deepseek 基准 | 独占 service + LLM API | 🔴 长期占用（40 分钟） | **季度体检用** |

**关键规则**：
- 零 token 测试——开发期间随手跑，就在 dev 区跑，不用切到 test 区
- VM 门禁——每次 commit 前在 dev 区跑
- 烧 token 测试——**只由测试窗口在 test 区跑，且只跑当前队列里的任务**
- 基准全量——**不在 dev 区跑！只由测试窗口在 test 区跑！**

---

## §4 冲突场景处理手册

| 场景 | 谁等谁 | 怎么办 |
|---|---|---|
| 开发在编译，测试要跑门禁 | 测试等 3-5 分钟（增量编译完） | 测试先跑零 token 测试（不占编译） |
| 测试在跑基准（30 分钟），开发要编译 | 开发切换到不编译的活（改 Python/写文档/审代码） | 或：另开一个代码目录 + 单独 cargo build |
| 测试发现 🔴 bug，开发要马上修 | 开发**不在 test 区修**——在 dev 区修，修完重新编译，把新二进制+源码推到 test 区让测试复测 | dev 修好 → `cp ~/codex_dev/target/release/service ~/codex_test/target/release/` |
| 两个窗口同时写入交换区 | 每个文件用 `$(date +%s)` 后缀，不覆盖 | 自动化脚本按时间排序读最新 |
| VM 磁盘满了 | 清理 `~/codex_test/target/`（共享 dev 的 target，test 不需要全量编译产物） | `du -sh ~/codex_test/target/` → 如果 > 500MB 就清 |

---

## §5 自动化脚本（给执行窗口实现）

### `vm-sync.sh` — 建立三区

```bash
#!/bin/bash
# 在 VM 上执行：创建三区隔离结构

# 1. 开发区（如果已存在只更新）
cd ~/codex_dev && git pull origin v23-dev

# 2. 测试区（frozen worktree）
git worktree add ~/codex_test v22.0
cd ~/codex_test
# 复用 dev 的 target 缓存（软链接）
ln -s ~/codex_dev/target ~/codex_test/target

# 3. 交换区
mkdir -p ~/codex_work/{test-findings,test-queue,gate-logs}

# 4. 测试 service 专用端口
cp ~/codex_dev/.env ~/codex_test/.env
echo "PORT=3001" >> ~/codex_test/.env
```

### `post-test.sh` — 测试窗口提交结果

```bash
#!/bin/bash
# 测试窗口跑完所有测试后执行

TIMESTAMP=$(date +%Y%m%d-%H%M)

# 收集结果
echo "=== $(date) ===" > ~/codex_work/test-findings/$TIMESTAMP.txt
echo "scenarios: $(cat ~/codex_work/test-queue/pending.txt 2>/dev/null || echo 'manual')" >> ~/codex_work/test-findings/$TIMESTAMP.txt
echo "passed: $(grep -c PASS ~/codex_test/last_test.log 2>/dev/null || echo '?')" >> ~/codex_work/test-findings/$TIMESTAMP.txt
echo "🔴: $(grep -c '🔴\|RED\|vuln\|FAIL' ~/codex_test/last_test.log 2>/dev/null || echo '0')" >> ~/codex_work/test-findings/$TIMESTAMP.txt

# 清理自己
fuser -k 3001/tcp 2>/dev/null
mv ~/codex_work/test-queue/pending.txt ~/codex_work/test-queue/done-$TIMESTAMP.txt 2>/dev/null

echo "done. results in ~/codex_work/test-findings/$TIMESTAMP.txt"
```

### `pull-results.sh` — 执行窗口取结果

```bash
#!/bin/bash
# 执行窗口随时跑：拉最新测试发现

echo "=== 最新测试结果 ==="
ls -t ~/codex_work/test-findings/ | head -1 | xargs cat

echo ""
echo "=== 所有 🔴 发现 ==="
grep -h "🔴\|RED\|vuln" ~/codex_work/test-findings/*.txt 2>/dev/null | tail -20
```

---

## §6 给执行窗口的实现清单

| 脚本 | 功能 | 谁跑 | 时机 |
|---|---|---|---|
| `vm-sync.sh` | 创建三区隔离结构 | 执行窗口（一次性） | 项目初始化 |
| `post-test.sh` | 测试完成提交结果 + 清理 | 测试窗口 | 每次测试结束 |
| `pull-results.sh` | 拉最新测试发现 | 执行窗口 | 需要时随时跑 |
| `queue-test.sh` | 投递测试任务到队列 | 执行窗口 | 有好测试场景要跑时 |

**不需要复杂调度器**——三区物理隔离 + 三个脚本 + 交换区文件传递，足够两个窗口高效协作。

---

## §7 关键铁律

1. **测试窗口永不在 dev 区跑任何东西**——test 区是 test 区，dev 区是 dev 区
2. **测试窗口的 codex_test 是 frozen worktree**, 永远不 commit
3. **端口隔离**：dev=3000, test=3001，永不混用
4. **烧 token 测试只由测试窗口在 test 区跑**，执行窗口不要自己跑基准
5. **结果只通过交换区传递**——不要靠人在聊天里转述"T03 挂了"
6. **磁盘监控**：`~/codex_test/target/` 不能超过 500MB（和 dev 的 target 软链接后基本不占）
