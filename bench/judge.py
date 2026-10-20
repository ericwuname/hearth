#!/usr/bin/env python3
"""bench/judge.py —— 判据结果判定（M0 / D-222）。

**为什么单独成模块**：judge 是「通过/失败」的唯一终审，必须可被单元测试直接钉住；
放进 runner.py 会连带 `paramiko` 等第三方依赖，无法在无 VM 环境下测试。

修复前的语义（在 runner.py 内联）等价于：

    success = "VERIFY_PASS" in verify_out

即**只看输出子串、完全忽略退出码**。后果：一个 `cargo test` 失败、却仍打印
`VERIFY_PASS` 的判据（T02/T03 即如此）会被记成「成功」——实测 T03 在 2/2 测试
失败的情况下被判通过。

修复后的语义（D-222）：**退出码为 0** 且**存在独立的 VERIFY_PASS 行**，二者缺一不可。
"""

VERIFY_PASS_TOKEN = "VERIFY_PASS"

# runner 在远端执行的命令形如：
#   cd <ws> && source ~/.cargo/env 2>/dev/null; bash __verify.sh 2>&1; echo "__HEARTH_RC=$?"
# 退出码由这行回传（`exec_command` 拿不到远端退出码，只能自己带回来）。
RC_MARKER = "__HEARTH_RC="


def split_rc(output: str):
    """从 verify 输出中取出回传的退出码。

    Returns:
        (去掉 rc 行的输出, rc 或 None)。若同一输出里出现多次，取**最后一次**
        （防判据自身打印了相似字面量）；解析失败一律 None ⇒ 后续判失败。
    """
    rc = None
    kept = []
    for line in output.splitlines():
        s = line.strip()
        if s.startswith(RC_MARKER):
            try:
                rc = int(s[len(RC_MARKER):].strip())
            except ValueError:
                rc = None
            continue
        kept.append(line)
    return "\n".join(kept), rc


def judge_verify(verify_out: str, rc):
    """判定一次 verify.sh 执行是否算「任务成功」。

    三条同时成立才算成功：
      ① `rc == 0`（修复前忽略退出码 ⇒ fail-open）；
      ② `rc` 必须可判定——`None`（取不到）一律判失败，不得 fail-open；
      ③ 输出中存在**独立成行**的 `VERIFY_PASS` 标记（防「判据什么都没做也退出 0」，
         也防标记只是散落在说明文字里的子串）。

    Args:
        verify_out: verify.sh 的完整 stdout+stderr。
        rc: verify.sh 的退出码；None 表示无法取得。

    Returns:
        bool
    """
    if not isinstance(rc, int) or isinstance(rc, bool):
        return False
    if rc != 0:
        return False
    for line in verify_out.splitlines():
        if line.strip().startswith(VERIFY_PASS_TOKEN):
            return True
    return False


# ============================================================================
# 会话终态 × 外部判据 —— 交叉分类（M1 / D-229）
#
# **为什么要单独做这个**：本仓最该防的病是"**自报成功**与**实际成功**不一致"。
# 内部审计口径里它叫"假完成"（session 报 done，外部判据判失败）。业界同源：
# 有研究显示 agent 在没读完文件的情况下 67.9% 的运行会发出误导性陈述，其中 80.4%
# 是可证伪的过度声称。要让这个数字可测，必须由**两个互不可见的系统**分别判定，
# 再**跨进程比对**——这就是下面这个纯函数在做的事。
#
# **归因限制（必须记住）**：`false_completion` 为真只说明"两者不一致"，**不区分**
#   ① agent 过度声称（真问题）；② 判据过严（harness 问题）。
# 要压住 ② 得靠 M0 建立的反向证据（`bench/solutions/` 证明"正确解必判通过"）。
# ============================================================================

# 会话自报"完成"的终态（其余终态见 `agent-core/src/terminal.rs` 的九态封闭集）
SELF_REPORTED_DONE_PHASES = ("done", "completed")


def classify_run(claim, external_ok):
    """把「**agent 自报的结论**」与「外部判据的结论」交叉分类。

    **D-230 订正（重要）**：初版这里用的是 HTTP `/status` 的 `phase`。那是**错的**口径：
    `phase == "done"` 只表示"循环正常终止"，**不表示** agent 声称自己做成了。
    实测证据：31 条录制语料里 `done.report.ok == true` 的只有 **1** 条，另 30 条自报
    `status="budget_exhausted"`；而 `phase` 全部是 `done` ⇒ 旧口径会把 30 条"agent 自己
    承认没做完"的会话算成"自报完成"，把反向不一致也一起吞掉。

    Args:
        claim: **agent 自报**——`True`=说做成了；`False`=说没做成（如 `budget_exhausted`）；
               `None`=未知（没有 done 事件 / 取不到）。
        external_ok: 外部判据是否判通过。

    Returns:
        dict：claim_ok / claim_known / external_ok / false_completion / verdict_conflict /
              unknown_claim
    """
    ok = bool(external_ok)
    return {
        "claim_ok": claim is True,
        "claim_known": claim is not None,
        "external_ok": ok,
        # **假完成**：agent 说做成了，外部判据说没成
        "false_completion": (claim is True) and not ok,
        # **反向不一致**：agent 说没做成，外部判据说成了（多见于预算耗尽但活干完了）
        "verdict_conflict": (claim is False) and ok,
        "unknown_claim": claim is None,
    }


def summarize_runs(records):
    """从逐条记录汇总一等指标。

    `false_completion_rate` 的分母是**自称做成的会话数**（不是总数）：
    要回答的是"**自称成功的里面，有多少是假的**"。
    分母为 0 时**不返回 0.0**，而是 `None`——本仓口径：未知 ≠ 0（同 D-176/D-219）。
    """
    total = len(records)
    claimed = sum(1 for r in records if r.get("claim_ok"))
    ext_ok = sum(1 for r in records if r.get("external_ok"))
    fc = sum(1 for r in records if r.get("false_completion"))
    vc = sum(1 for r in records if r.get("verdict_conflict"))
    unk = sum(1 for r in records if r.get("unknown_claim"))
    return {
        "runs": total,
        "claim_ok": claimed,
        "external_ok": ext_ok,
        "false_completion": fc,
        "false_completion_rate": (fc / claimed) if claimed else None,
        "verdict_conflict": vc,
        "unknown_claim": unk,
    }
