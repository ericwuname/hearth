#!/usr/bin/env python3
"""EMBER-M1 三工具实现：bash / read / write（仅 Python 标准库）。

设计目标：
- 每个工具执行**不抛异常裸奔**——失败也返回结构化文本说明（供模型读取与自纠）；
- 输出统一截断策略：>64KB 时保留头 2/3 + 尾 1/3，中间以 [truncated] 标记；
- 工具参数为 OpenAI 兼容 Chat Completions 的 tool_calls 载荷（JSON 字符串或 dict）。
"""
import json
import os
import shlex
import subprocess

TRUNCATE_LIMIT = 64 * 1024  # 64KB
BASH_TIMEOUT_SECONDS = 60


def _truncate(text):
    """按字节计 >64KB 时截断：在 64KB 预算内保留头 2/3 + 尾 1/3，中间标 [truncated]。

    返回 (截断后文本, 是否发生截断)。按字符切分保证不在多字节中间截断。
    """
    encoded = text.encode("utf-8")
    if len(encoded) <= TRUNCATE_LIMIT:
        return text, False
    # 预算内分配：头 2/3、尾 1/3（按字符近似切分，保证输出总长约 64KB）
    head_chars = min(len(text), int(TRUNCATE_LIMIT * 2 // 3))
    tail_budget = TRUNCATE_LIMIT - len(text[:head_chars].encode("utf-8"))
    tail_chars = 0
    acc = 0
    # 从尾部收集字符直至达到 tail 预算
    for ch in reversed(text):
        cb = len(ch.encode("utf-8"))
        if acc + cb > max(tail_budget, 0):
            break
        acc += cb
        tail_chars += 1
    head = text[:head_chars]
    tail = text[len(text) - tail_chars:] if tail_chars else ""
    return (
        head
        + "\n[truncated 原文 %d 字节，保留头 2/3 与尾 1/3]\n" % len(encoded)
        + tail,
        True,
    )


def _result_note(text, extra=None):
    """组合主输出与可选的结构化说明。"""
    if extra:
        return text + "\n" + extra
    return text


def _parse_args(raw):
    """tool_calls 的 arguments 可能是 JSON 字符串或 dict，统一成 dict。"""
    if raw is None:
        return {}
    if isinstance(raw, dict):
        return raw
    if isinstance(raw, str):
        try:
            parsed = json.loads(raw)
            return parsed if isinstance(parsed, dict) else {}
        except (json.JSONDecodeError, TypeError):
            return {}
    return {}


# ---------------------------------------------------------------- 参数归一

def _as_args(first, second=None):
    """把 dispatcher 传入的参数归一化成 dict。

    支持两种调用形态：
    - fn(args_dict)：整个载荷是一个 dict（tool_calls arguments 解析结果）；
    - fn(param1, param2)：按位置传参。
    """
    if isinstance(first, dict):
        base = dict(first)
        if second is not None and "content" not in base:
            base["content"] = second
        return base
    d = {"path": first}
    if second is not None:
        d["content"] = second
    return d

def bash(command):
    """执行 shell 命令；超时 60s；stdout+stderr 合并返回；输出超 64KB 截断。

    失败（超时/非零退出）返回结构化说明而非抛异常。
    """
    # 执行窗修复（2026-09-11）：兼容"字符串直给"调用形态——原 _as_args 对非 dict
    # 一律归一成 {"path": ...}，导致 bash("ls") 被判"缺少 command 参数"（M1 实测：
    # 所有 bash 调用返回 51 字符错误文本）。
    if isinstance(command, str):
        cmd = command.strip()
    else:
        args = _as_args(command)
        cmd = str(args.get("command") or "").strip()
    if not cmd:
        return "错误: bash 工具缺少 command 参数。示例: bash(command=\"ls -la\")"
    try:
        proc = subprocess.run(
            cmd,
            shell=True,
            capture_output=True,
            text=True,
            timeout=BASH_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        return _result_note(
            "[timeout] 命令在 %ds 内未完成，已被强制终止。" % BASH_TIMEOUT_SECONDS,
            "建议: 缩小命令范围（如加 head/tail 限制输出、减少数据量），或拆成多个小命令重试。",
        )
    except (OSError, ValueError) as e:
        return "错误: 无法执行命令 %r: %s。建议: 检查命令拼写与 shell 语法。" % (cmd[:120], e)
    out = _truncate((proc.stdout or "") + (proc.stderr or ""))
    code_note = "exit code: %d" % proc.returncode
    extra = None
    if proc.returncode != 0:
        extra = "[non-zero exit] %s — 命令未成功。请阅读上方输出定位原因后修正。" % code_note
    return _result_note(out[0] + "\n" + code_note, extra)


# ---------------------------------------------------------------- read

def read(path):
    """读文件内容；超 64KB 同样截断；失败返回结构化说明。"""
    args = _as_args(path)
    p = str(args.get("path") or "").strip()
    if not p:
        return "错误: read 工具缺少 path 参数。示例: read(path=\"app.log\")"
    try:
        with open(p, "r", encoding="utf-8", errors="replace") as f:
            text = f.read()
    except FileNotFoundError:
        return "错误: 文件不存在: %s。建议: 用 bash(ls) 确认文件名与路径。" % p
    except IsADirectoryError:
        return "错误: 目标是目录而非文件: %s。建议: 先 bash(ls %s) 看目录内容。" % (p, p)
    except PermissionError:
        return "错误: 无读取权限: %s。" % p
    except OSError as e:
        return "错误: 读取失败 %s: %s。" % (p, e)
    text, truncated = _truncate(text)
    note = ""
    if truncated:
        note = "\n[truncated] 文件超过 64KB，已截断（头 2/3 + 尾 1/3）。需要完整内容请用 bash 分段查看。"
    return text + note


# ---------------------------------------------------------------- write

def write(path, content=None):
    """写文件（自动建父目录）；返回写了多少字节。content 可缺省（清空/仅建目录）。"""
    args = _as_args(path, content)
    p = str(args.get("path") or "").strip()
    c = args.get("content")
    if c is None:
        c = ""
    if not isinstance(c, str):
        c = json.dumps(c, ensure_ascii=False) if c is not None else ""
    if not p:
        return "错误: write 工具缺少 path 参数。示例: write(path=\"out.txt\", content=\"hi\")"
    parent = os.path.dirname(p)
    if parent:
        try:
            os.makedirs(parent, exist_ok=True)
        except OSError as e:
            return "错误: 无法创建父目录 %s: %s。" % (parent, e)
    try:
        with open(p, "w", encoding="utf-8") as f:
            written = f.write(c)
    except PermissionError:
        return "错误: 无写入权限: %s。" % p
    except OSError as e:
        return "错误: 写入失败 %s: %s。" % (p, e)
    nbytes = len(c.encode("utf-8"))
    return "已写入 %s：%d 字符 / %d 字节。" % (p, written, nbytes)
