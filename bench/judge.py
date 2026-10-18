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
