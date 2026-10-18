#!/usr/bin/env python3
"""判据「送达 + 执行 + 判定」的唯一实现（M0 / D-224）。

**为什么把它抽出来**：判定口径曾在两处各写一份（`runner.py` 与 `replay.py`）。D-222 只修了
`runner.py`；`replay.py` 仍留着旧口径 `"VERIFY_PASS" in verify_out`（**忽略退出码**），
而且**不上传判据库** ⇒ 新版 `verify.sh`（`. "$(dirname "$0")/verify_lib.sh"`）在**回放路径**
上会一律报「缺少 verify_lib.sh」并判失败。这就是「同能力多处各写一份」的直接代价。

本模块把这件事收口成一份实现，两条路径共用；传输层用两个可调用对象注入，于是本地可用
假实现做契约测试（无需 SSH，见 `bench/tests/test_verify_exec.py`）：

    put(name: str, data: bytes) -> None    # 把文件送进工作区
    exec_(cmd: str) -> str                 # 在工作区内执行命令并取回输出

约定：**判据与判据库必须一起送达**（缺库不会静默放行，但会把「环境缺件」误报成「任务失败」）。
"""
import pathlib

import judge

BENCH_DIR = pathlib.Path(__file__).resolve().parent
DEFAULT_LIB = BENCH_DIR / "verify_lib.sh"

VERIFY_SCRIPT_NAME = "__verify.sh"
VERIFY_LIB_NAME = "verify_lib.sh"


def stage(put, verify_path, lib_path=None):
    """把判据与判据库一起送达工作区。"""
    put(VERIFY_SCRIPT_NAME, pathlib.Path(verify_path).read_bytes())
    put(VERIFY_LIB_NAME, pathlib.Path(lib_path or DEFAULT_LIB).read_bytes())


def run(exec_):
    """执行判据并取回 (去掉 rc 行的输出, rc 或 None)。"""
    raw = exec_(f'bash {VERIFY_SCRIPT_NAME} 2>&1; echo "{judge.RC_MARKER}$?"')
    return judge.split_rc(raw)


def verify_task(put, exec_, verify_path, lib_path=None):
    """送达 → 执行 → 判定；返回 (success, 输出, rc)。"""
    stage(put, verify_path, lib_path)
    body, rc = run(exec_)
    return judge.judge_verify(body, rc), body, rc


# ---------------------------------------------------------------- SSH 传输层
# 只做鸭子类型调用（`client.open_sftp()` / `client.exec_command()`），因此本模块
# 不依赖 paramiko：测试与本地路径都能用假实现替换。

def sftp_put(ftp, ws):
    """把 put(name, data) 适配到 paramiko SFTP。"""
    def _put(name, data):
        with ftp.file(f"{ws}/{name}", "wb") as fh:
            fh.write(data)
    return _put


def ssh_exec(client, ws):
    """把 exec_(cmd) 适配到 paramiko 的 exec_command（含 cargo 环境与工作目录）。"""
    def _exec(cmd):
        _, stdout, _ = client.exec_command(
            f"cd {ws} && source ~/.cargo/env 2>/dev/null; {cmd}"
        )
        return stdout.read().decode(errors="replace")
    return _exec
