#!/usr/bin/env python3
"""EMBER M1 — 工具循环版单轮问答 CLI（标准库 + tools.py 三工具）。

施工溯源（诚实申报）：
- tools.py：hearth-slim 施工（.133，2026-09-11）；
- 本文件（工具循环接线）：**执行窗补全**——hearth 因环境网络抖动（问题卡03）
  未能完成最后一步接线，执行窗接续完成，接口与设计遵循 M1 任务卡。

用法:
    python3 ember.py "问题"        # 带工具循环；模型可自主调 bash/read/write
协议: POST {base_url}/chat/completions（Agnes OpenAI 兼容）
"""
import json
import os
import sys
import time
import urllib.error
import urllib.request

from tools import bash as tool_bash
from tools import read as tool_read
from tools import write as tool_write

CONFIG_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "config.json")
# 执行窗调整（2026-09-11，M1 验收实测）：thinking 模型 + max_tokens=65536 的重推理轮
# 响应常超 60s（'The read operation timed out' 反复重试）——提到 180s。
TIMEOUT_SECONDS = 180
RETRY_DELAYS = [2, 4, 8]
MAX_TOOL_ROUNDS = 20  # 防死循环上限

# ── 工具 schema（五要素：何时用/参数/示例/边界/错误解读） ──
TOOLS_SCHEMA = [
    {
        "type": "function",
        "function": {
            "name": "bash",
            "description": (
                "执行 shell 命令并返回 stdout+stderr 合并输出。\n"
                "何时用: 运行程序、查看文件（grep/head/wc）、列目录、跑测试等。\n"
                "参数: command (string) —— 完整命令。示例: bash(command=\"wc -l app.log\")。\n"
                "边界: 超时 60 秒；输出超 64KB 截断；失败也返回文本（看 exit code/错误说明），不抛异常。\n"
                "错误解读: 输出含 'exit code: N'（N≠0 为失败）或 '超时' 字样。"
            ),
            "parameters": {
                "type": "object",
                "properties": {"command": {"type": "string", "description": "完整 shell 命令"}},
                "required": ["command"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "read",
            "description": (
                "读取文件内容（文本）。\n"
                "何时用: 查看源码/日志/配置等文件内容。\n"
                "参数: path (string)。示例: read(path=\"app.log\")。\n"
                "边界: 超 64KB 截断；二进制文件可能不可读。\n"
                "错误解读: 返回以 '错误:' 开头表示失败（文件不存在/无权限）。"
            ),
            "parameters": {
                "type": "object",
                "properties": {"path": {"type": "string", "description": "文件路径"}},
                "required": ["path"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "write",
            "description": (
                "写文件（自动创建父目录），整体覆盖。\n"
                "何时用: 生成代码/报告/工具等产物。\n"
                "参数: path (string), content (string)。示例: write(path=\"analyze.py\", content=\"...\")。\n"
                "边界: 覆盖写（非追加）；content 为完整文件内容。\n"
                "错误解读: 返回以 '错误:' 开头表示失败（无权限等）。"
            ),
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"},
                    "content": {"type": "string", "description": "完整文件内容"},
                },
                "required": ["path", "content"],
            },
        },
    },
]

TOOL_IMPL = {"bash": tool_bash, "read": tool_read, "write": tool_write}


def load_config():
    """读取 config.json；key 为空时从环境变量补全并写回（600 权限）。"""
    try:
        with open(CONFIG_PATH, "r", encoding="utf-8") as f:
            cfg = json.load(f)
    except FileNotFoundError:
        print("错误: 缺少配置文件 %s" % CONFIG_PATH, file=sys.stderr)
        print("建议: 先创建 config.json，含三键 base_url / model / key。", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print("错误: config.json 不是合法 JSON (%s)" % e, file=sys.stderr)
        print("建议: 检查文件内容（base_url/model/key 三键，UTF-8）。", file=sys.stderr)
        sys.exit(1)
    for field in ("base_url", "model", "key"):
        if field not in cfg:
            print("错误: config.json 缺少键 '%s'" % field, file=sys.stderr)
            sys.exit(1)
    if not cfg.get("key"):
        env_key = os.environ.get("APIHUB_AGNES_AI_API_KEY", "").strip()
        if not env_key:
            print("错误: config.json 中 key 为空，且环境变量 APIHUB_AGNES_AI_API_KEY 未设置。", file=sys.stderr)
            sys.exit(1)
        cfg["key"] = env_key
        try:
            with open(CONFIG_PATH, "w", encoding="utf-8") as f:
                json.dump(cfg, f, ensure_ascii=False, indent=2)
            os.chmod(CONFIG_PATH, 0o600)
        except OSError:
            pass
    return cfg


def chat_once(cfg, messages):
    """单次请求；返回 assistant message dict。"""
    url = cfg["base_url"].rstrip("/") + "/chat/completions"
    body = json.dumps({
        "model": cfg["model"],
        "messages": messages,
        "tools": TOOLS_SCHEMA,
        "tool_choice": "auto",
        "max_tokens": 65536,
        "temperature": 0,
        "chat_template_kwargs": {"enable_thinking": True},
    }).encode("utf-8")
    req = urllib.request.Request(url, data=body, method="POST", headers={
        "Content-Type": "application/json",
        "Authorization": "Bearer %s" % cfg["key"],
    })
    with urllib.request.urlopen(req, timeout=TIMEOUT_SECONDS) as resp:
        payload = json.loads(resp.read().decode("utf-8"))
    choices = payload.get("choices") or []
    if not choices:
        raise RuntimeError("响应缺少 choices（前 200 字符: %s）" % json.dumps(payload, ensure_ascii=False)[:200])
    return choices[0].get("message") or {}


def call_model(cfg, messages):
    """带重试的模型调用（401/403 不重试；瞬时错误 2/4/8s 退避）。"""
    last_err = None
    for i in range(4):
        delay = RETRY_DELAYS[min(i, len(RETRY_DELAYS) - 1)]
        try:
            return chat_once(cfg, messages)
        except urllib.error.HTTPError as e:
            detail = ""
            try:
                detail = e.read().decode("utf-8", "replace")[:300]
            except Exception:
                pass
            if e.code in (401, 403):
                raise RuntimeError("认证失败 (HTTP %s): %s" % (e.code, detail)) from None
            last_err = "HTTP %s %s" % (e.code, detail)
        except urllib.error.URLError as e:
            last_err = "网络不可达: %s" % e.reason
        except (TimeoutError, RuntimeError, json.JSONDecodeError, OSError) as e:
            last_err = str(e)
        if i < 3:
            print("[retry] 请求失败（第 %d 次）: %s — 等待 %ds 后重试…" % (i + 1, last_err, delay), file=sys.stderr)
            time.sleep(delay)
        else:
            print("[retry] 请求失败（第 %d 次）: %s" % (i + 1, last_err), file=sys.stderr)
    raise RuntimeError("重试 3 次后仍失败，最后一次错误: %s" % last_err)


def run_tool(name, raw_args):
    """执行工具调用；返回结果文本（永不抛异常）。"""
    fn = TOOL_IMPL.get(name)
    if fn is None:
        return "错误: 未知工具 '%s'（可用: bash / read / write）" % name
    if isinstance(raw_args, str):
        try:
            args = json.loads(raw_args) if raw_args.strip() else {}
        except json.JSONDecodeError as e:
            return "错误: 工具参数不是合法 JSON (%s): %s" % (e, raw_args[:120])
    elif isinstance(raw_args, dict):
        args = raw_args
    else:
        args = {}
    try:
        return fn(**args)
    except TypeError as e:
        return "错误: 工具 %s 参数不匹配 (%s)；收到: %s" % (name, e, json.dumps(args, ensure_ascii=False)[:200])
    except Exception as e:  # 兜底：工具内部异常也结构化返回
        return "错误: 工具 %s 执行异常: %s" % (name, e)


def solve(cfg, question):
    """工具循环主体：多轮调工具直到模型交卷。返回最终答案文本。"""
    messages = [{"role": "user", "content": question}]
    empty_nudges = 0
    for round_no in range(1, MAX_TOOL_ROUNDS + 1):
        msg = call_model(cfg, messages)
        tool_calls = msg.get("tool_calls") or []
        content = msg.get("content") or ""
        if tool_calls:
            # 保留 assistant 消息（含 tool_calls）——tool 结果必须与之配对
            messages.append({
                "role": "assistant",
                "content": content,
                "tool_calls": tool_calls,
            })
            for tc in tool_calls:
                fn_info = tc.get("function") or {}
                name = fn_info.get("name") or ""
                raw_args = fn_info.get("arguments")
                t0 = time.time()
                print("[tool] %s(%s)" % (name, (str(raw_args) or "")[:160]), file=sys.stderr)
                result = run_tool(name, raw_args)
                dt = time.time() - t0
                print("[tool] ← %s（%.1fs，%d 字符）" % (name, dt, len(result)), file=sys.stderr)
                messages.append({
                    "role": "tool",
                    "tool_call_id": tc.get("id") or "",
                    "content": result,
                })
            continue
        # 无工具调用 = 交卷
        if content.strip():
            return content
        # 空正文（thinking 占满/模型异常）→ 催一次，最多 2 次
        empty_nudges += 1
        if empty_nudges > 2:
            return "（模型连续返回空正文——可能 thinking 占满输出预算，请重试）"
        messages.append({"role": "user", "content": "请直接给出最终答案（不要再调用工具）。"})
    return "（达到 %d 轮工具循环上限，任务未收敛——请缩小问题范围重试）" % MAX_TOOL_ROUNDS


def main():
    if len(sys.argv) != 2 or not sys.argv[1].strip():
        print('用法: python3 ember.py "问题"', file=sys.stderr)
        sys.exit(1)
    question = sys.argv[1].strip()
    cfg = load_config()
    try:
        answer = solve(cfg, question)
    except RuntimeError as e:
        print("错误: %s" % e, file=sys.stderr)
        if "认证失败" in str(e):
            print("建议: 检查 config.json 的 key 或环境变量 APIHUB_AGNES_AI_API_KEY 是否正确。", file=sys.stderr)
        else:
            print("建议: 检查网络连通性（目标 %s）与 base_url 配置；稍后可重试。" % cfg["base_url"], file=sys.stderr)
        sys.exit(1)
    except KeyboardInterrupt:
        print("\n已中断。", file=sys.stderr)
        sys.exit(130)
    print(answer)


if __name__ == "__main__":
    main()
