#!/usr/bin/env python3
"""Hearth PTY Driver — SimUser Stage 1 (P4-REVALIDATION-01 Node 01).

开发侧工具，不进 Core。驱动 hearth repl/chat/resume 的真实 PTY 交互。

P0 三版失真陷阱清单——逐条规避（三版废品换来的数据）：
  T1. 管道 stdin = DenyAllNonInteractive + EOF 后 REPL 忙等刷 prompt（56MB）
      → 本驱动使用**真实 PTY**（isatty=true，交互式审批策略）。
  T2. `script -qec` 输入消费竞争（191MB 刷屏）
      → 输入**定步调**：send 一行后必须 wait_turn（等 Task completed/failed
        或 prompt 空闲标记），未等完绝不发下一行——保证用户输入只被消费一次。
  T3. resume goal 语义：`hearth resume <id> "<text>"` 恢复**旧 goal**（text 只
      入队消息）→ 本驱动不假设 "resume 后 goal = 新输入"，goal 状态以日志为
      准（goal_revision 事件采集）。

API（总包 §3.2）：send / expect / wait_terminal / capture / timeout / interrupt。
"""
from __future__ import annotations

import os
import pty
import select
import subprocess
import time
from dataclasses import dataclass, field
from typing import List, Optional, Tuple

TURN_TERMINAL_MARKERS = ("Task completed", "Task failed")
REPL_PROMPT = "hearth> "


@dataclass
class DriverConfig:
    cmd: List[str]                      # e.g. ["hearth", "repl"] or ["hearth","chat","goal"]
    cwd: Optional[str] = None
    env_extra: Optional[dict] = None    # HEARTH_ALLOW_NO_CGROUP / HEARTH_DEBUG_PLANNER_INPUT ...
    turn_timeout: int = 600             # 单轮上限（超时触发 timeout 标记）
    quiet_read: float = 0.25            # PTY 轮询间隔（秒）


@dataclass
class DriverRun:
    log: List[str] = field(default_factory=list)          # 全量终端侧文本（含 ANSI）
    events: List[dict] = field(default_factory=list)      # 结构化事件（send/terminal/timeout/interrupt）
    session_ids: List[str] = field(default_factory=list)  # 从输出提取的 session uuid


class HearthDriver:
    """真实 PTY 驱动。用法：
        d = HearthDriver(DriverConfig(["hearth","repl"], cwd="/home/wutao"))
        d.start(); d.send_and_wait_turn("目标"); ... ; d.close()
    """

    def __init__(self, cfg: DriverConfig, run: Optional[DriverRun] = None):
        self.cfg = cfg
        self.run = run or DriverRun()
        self.master: Optional[int] = None
        self.proc: Optional[subprocess.Popen] = None
        self._buf = ""
        self.last_terminal: Optional[str] = None

    # ---- lifecycle ----
    def start(self) -> None:
        env = dict(os.environ)
        env.setdefault("TERM", "xterm")
        if self.cfg.env_extra:
            env.update(self.cfg.env_extra)
        self.master, slave = pty.openpty()
        self.proc = subprocess.Popen(
            self.cfg.cmd,
            cwd=self.cfg.cwd,
            stdin=slave,
            stdout=slave,
            stderr=slave,
            preexec_fn=os.setsid,   # 新会话 → 该子进程持有 PTY（isatty=true）
            close_fds=True,
            env=env,
        )
        os.close(slave)
        self.run.events.append({"t": time.time(), "type": "start", "cmd": self.cfg.cmd})

    def close(self) -> None:
        try:
            if self.proc and self.proc.poll() is None:
                self.interrupt()
                time.sleep(1.0)
                self.proc.terminate()
        except Exception:
            pass
        if self.master is not None:
            try:
                os.close(self.master)
            except OSError:
                pass
        self.run.events.append({"t": time.time(), "type": "close"})

    # ---- 核心 API ----
    def send(self, line: str) -> None:
        """写入一行输入（真实 PTY：isatty=true，不触发审批降级）。
        ⚠ 调用方随后必须 wait_turn()——T2 输入消费竞争的规避即在此。"""
        os.write(self.master, (line + "\n").encode("utf-8"))
        self.run.events.append({"t": time.time(), "type": "send", "line": line[:200]})

    def _pump(self, timeout: float) -> bool:
        """读 PTY 到内部缓冲；返回 True=有新数据。"""
        r, _, _ = select.select([self.master], [], [], timeout)
        if not r:
            return False
        try:
            chunk = os.read(self.master, 65536).decode("utf-8", errors="replace")
        except OSError:
            return False
        if not chunk:
            return False
        self._buf += chunk
        self.run.log.append(chunk)
        for m in ("session ", "(session "):
            pass
        import re
        for mm in re.finditer(r"session ([0-9a-f-]{36}|[0-9a-f]{8})", self._buf):
            sid = mm.group(1)
            if sid not in self.run.session_ids:
                self.run.session_ids.append(sid)
        return True

    def expect(self, patterns: List[str], timeout: float) -> Tuple[Optional[str], str]:
        """等待任一 pattern 出现于 PTY 流（**仅搜索新增数据**——T2 变体修复：
        全量搜索会让初始 banner 的 prompt 提前命中，导致输入在轮结束前发出而丢失）。"""
        deadline = time.time() + timeout
        start = len(self._buf)
        while time.time() < deadline:
            window = self._buf[start:]
            for p in patterns:
                if p in window:
                    return p, self._buf
            self._pump(0.25)
        return None, self._buf

    def wait_turn(self, timeout: Optional[int] = None) -> Optional[str]:
        """等当前轮结束（Task completed/failed 出现）或超时。
        返回终态 marker（completed/failed/timeout）。
        T2 规避核心：send() 之后必须调用本方法再发下一行。"""
        t = timeout if timeout is not None else self.cfg.turn_timeout
        deadline = time.time() + t
        start = len(self._buf)
        while time.time() < deadline:
            for m in TURN_TERMINAL_MARKERS:
                if m in self._buf[start:]:
                    self.last_terminal = "completed" if m.endswith("completed") else "failed"
                    self.run.events.append({
                        "t": time.time(),
                        "type": "terminal",
                        "terminal": self.last_terminal,
                    })
                    # 吸干残留（报告路径等），再等 prompt 空闲
                    self.expect([REPL_PROMPT], 20)
                    return self.last_terminal
            self._pump(0.5)
        self.run.events.append({"t": time.time(), "type": "timeout", "timeout": t})
        self.last_terminal = "timeout"
        return "timeout"

    def send_and_wait_turn(self, line: str, timeout: Optional[int] = None) -> Optional[str]:
        self.send(line)
        return self.wait_turn(timeout)

    def capture(self) -> str:
        """当前全量终端侧文本（用户侧 Reality 原始记录）。"""
        return self._buf

    def interrupt(self) -> None:
        """Ctrl-C（超时/放弃路径——标记 timeout/DRIVER-INDUCED 由 analyzer 判）。"""
        try:
            os.write(self.master, b"\x03")
            self.run.events.append({"t": time.time(), "type": "interrupt"})
        except OSError:
            pass


# ---- 便捷入口：三种形态（T3 语义已在注释与事件中显式） ----

def drive_repl(inputs: List[str], cwd: str, log_path: str,
               env_extra: Optional[dict] = None, turn_timeout: int = 600) -> DriverRun:
    """REPL 多轮：逐条 send → wait_turn（严格定步调）。
    R4.1 引擎身份纪律（2026-09-05）：被测引擎二进制由 env HEARTH_BIN 显式指定
    （默认 "hearth"=PATH——**跑测必须显式传绝对路径**，防 PATH 解析到旧版安装）。"""
    import os
    engine = (env_extra or {}).pop("HEARTH_BIN", None) or os.environ.get("HEARTH_BIN", "hearth")
    d = HearthDriver(DriverConfig([engine, "repl"], cwd=cwd,
                                  env_extra=env_extra, turn_timeout=turn_timeout))
    d.start()
    try:
        d.expect([REPL_PROMPT], 60)  # 等 REPL 就绪
        for line in inputs:
            d.send_and_wait_turn(line)
        d.send("/quit")
        time.sleep(2.0)
    finally:
        d.close()
    with open(log_path, "w", encoding="utf-8") as f:
        f.write(d.capture())
    import json
    with open(log_path + ".events.json", "w", encoding="utf-8") as f:
        json.dump(d.run.events, f, ensure_ascii=False, indent=1)
    return d.run


if __name__ == "__main__":
    import sys
    print("usage: see drive_repl() / P4 Node 03 scenario runner", file=sys.stderr)
