# 草稿卡 · TUI-fix3（微卡）：补 panic 路径的终端恢复

> **状态：草稿，未发射。需顶层批复后方可启动。**
> 立卡人：砺·评审（2026-09-12 21:35）｜依据：fix2 在飞检视发现 **G1(P1)**

## 背景（源码锚点，非转述）

fix2 卡文原话要求：「启动 `EnableMouseCapture`、退出 `DisableMouseCapture`（**panic/退出路径也要恢复**，否则终端残留鼠标模式）」。

实测现状（`src/main.rs` md5 `b451f288…`，21:29:49 快照）：

| 项 | 现状 |
|---|---|
| 正常返回路径 | ✅ `:346-348` `DisableMouseCapture` → `disable_raw_mode()` → `LeaveAlternateScreen` |
| **panic 路径** | ❌ **完全没有**。全文件 `grep -n 'panic\|set_hook\|catch_unwind'` → **0 命中** |

**后果**：任何 Rust panic（如 `?` 传播的 IO 错误、索引越界、`unwrap`）都会让终端**卡在备用屏 + 鼠标捕获模式**，用户终端直接花掉，需手动 `reset`。对"每天打开的工具"是硬伤。

**附带发现 G2(P2)**：`:232` `scroll_offset += 3` 未钳制上界（同分支 `:238` `ScrollDown` 用的是 `saturating_sub(3)`）。

---

## 卡文（直接复制给 EMBER）

```
任务：hearth TUI 修复-3（微卡）：panic 路径的终端恢复 + 一处滚动边界钳制

【工作目录】/home/wutao/hearth-tui-new
【只改】src/main.rs（改动量预期 ≤ 25 行）
【步数上限】8 步

【必须遵守的执行纪律（前车之鉴，违反即判无效）】
1. 每条 cargo 命令前必须：export PATH=$HOME/.cargo/bin:$PATH
2. 【证据先落盘】改完代码后的第 1 个动作就是写证据文件，再去收尾/汇报。
   绝不把"写证据"放到最后一两步——历次都是在那里被轮次上限截停。
3. 证据文件路径固定为：/home/wutao/hearth-tui-new/evidence-fix3.log
   用 >> 追加，不要用 >（禁止覆盖任何已有日志）。
4. 不得新建/删除其他文件；不得动 spawn/线程/channel、历史/折叠/滚动偏移逻辑（除本卡指定的一行）。

【修复 A（P1）：panic 路径恢复终端】
- 把终端恢复逻辑抽成一个独立函数，例如：
    fn restore_terminal() {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
  注意：必须用 let _ = 吞掉错误，panic 期间再抛错会二次 panic。
- 在 main() 的**最前面**（进入 raw mode / AlternateScreen 之前）注册 panic hook：
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));
  要点：先恢复终端、再调用原 hook 打印 panic 信息（顺序不能反，否则信息会被备用屏吞掉）。
- 正常返回路径（当前 :346-348）改为直接调用 restore_terminal()，不得出现两套重复实现。
- 幂等：restore_terminal() 被调用两次必须无害。

【修复 B（P2）：滚动边界钳制】
- 把 MouseEventKind::ScrollUp 分支的 `scroll_offset += 3;`
  改为与 PageUp 一致的安全写法（若 PageUp 用 saturating_add 就跟随；
  若两处都无上界，则统一钳到 max_off 对应上界）。
- 一句话说明你最终采用的钳制口径。

【自测（全部结果写入 evidence-fix3.log，逐条标注 PASS/FAIL）】
a) cargo build --release 通过 —— 贴出末尾 2 行。
b) 静态断言（这两条必须能被改红）：
   grep -c 'set_hook' src/main.rs   → 期望 1
   grep -n 'set_hook' -A4 src/main.rs 中必须同时出现 restore_terminal 与 DisableMouseCapture
c) 真行为（env 门控，默认关闭，不得影响正常启动）：
   在 main() 里加一个仅在环境变量存在时才触发的 panic 探针，例如
     if std::env::var("HEARTH_TUI_PANIC_TEST").is_ok() { panic!("panic-restore self-test"); }
   然后：
     export PATH=$HOME/.cargo/bin:$PATH
     tmux kill-session -t f3 2>/dev/null
     tmux new-session -d -s f3 -x 120 -y 30
     tmux send-keys -t f3 'HEARTH_TUI_PANIC_TEST=1 ./target/release/hearth-tui' Enter
     sleep 3
     tmux capture-pane -t f3 -p | tail -20
   期望：抓屏里能看到 panic 信息（panic-restore self-test），
   且终端已回到普通屏（不是备用屏内容、不是花屏）。
   把抓屏原文整段贴进 evidence-fix3.log，并明确写 PASS 或 FAIL。
   ⚠️ 探针是"测试钩子"，保留在代码里可以，但必须在代码注释里标明
      "仅用于自测，默认不触发"，并在汇报里显式声明。
d) 回归（证明没改坏）：正常启动（不带该 env），发一条消息 "只回答一个数字：3+4=?" Enter，
   sleep 40 后抓屏；期望出现 hearth 的回答 7 且状态栏 done。
   把抓屏原文贴进 evidence-fix3.log。

【汇报（最后一步）】
用表格逐条给 a/b/c/d 的 PASS/FAIL + 证据文件行数，并显式声明：
- 是否 8 步内完成；
- 哪些验收项**没跑**（若有，直说，不要报"已完成"）。
禁止报「Task completed」而实际有未跑项——历次假交付都栽在这里。
```

---

## 立卡依据与风险

| 项 | 说明 |
|---|---|
| 立卡依据 | fix2 在飞检视发现 G1（`:346-348` 只有正常路径；`grep panic` = 0） |
| 为什么是微卡 | 改动 ≤25 行，落在 1 个文件；历次"大卡"（step3/step4）撞 20 轮上限未收敛 |
| 步数上限 8 | 依 H8：预算耗尽的落点总在"最后一步"，故要求**证据先落盘** |
| 是否可失败 | 自测 b 是静态断言（改坏即红）；c 是真行为（不带 hook 则终端残留，抓屏可辨） |
| 与前序关系 | 独立于 fix2；**必须等 fix2 两实例完全退出后**才能启动（并发铁律） |
| 未覆盖 | D9（记不住就猜）/ D10（steps 语义）/ D11（状态栏滞后）/ D12（预置假对话）；草稿卡 `tui-step6-draft-card.md` 另有安排 |

## 启动前检查单（给执行窗）

```
# 1) 双条件确认无并发（缺一不可）
pgrep -af ember.py            # 必须为空（注意排除 bash -lc 自匹配）
tail -3 /home/wutao/hearth-tui-new/run-fix2.log   # 必须出现 fix2 end exit=

# 2) 用带时间戳的实例名留档（防 fix2 的 stdout 丢失事故重演）
cp -f /home/wutao/hearth-tui-new/src/main.rs \
      /home/wutao/hearth-tui-new/src/main.rs.bak-before-fix3_$(date +%Y%m%d-%H%M%S)
setsid nohup bash -c 'cd /home/wutao/hearth-tui-new; \
  export PATH=$HOME/.cargo/bin:$PATH; \
  export APIHUB_AGNES_AI_API_KEY=$(python3 -c "import json;print(json.load(open(\"/home/wutao/ember/config.json\"))[\"key\"])"); \
  echo "[$(date +%H:%M:%S)] fix3 start" >> run-fix3.log; \
  timeout 1200 python3 /home/wutao/ember/ember.py "$(cat /home/wutao/tui-fix3-task.txt)" \
    >> run-fix3.log 2>> trace-fix3.err; \
  echo "[$(date +%H:%M:%S)] fix3 end exit=$?" >> run-fix3.log' \
  >/tmp/fix3-driver.log 2>&1 </dev/null &
```
