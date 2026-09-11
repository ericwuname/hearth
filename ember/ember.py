#!/usr/bin/env python3
"""EMBER M2 — 会话持久化版（记性）：跨进程记忆 + 续干。

用法:
    python3 ember.py "问题"              # 新会话（带工具循环）
    python3 ember.py -c "追问"           # 加载最近会话继续（记得上次的活）
    python3 ember.py --list              # 列出历史会话
    python3 ember.py -c --list           # 同 --list

会话文件: .ember/sessions/<session_id>.jsonl（每行一条消息 JSON）
最近指针: .ember/last_session
"""
import json
import os
import sys
import time
import urllib.error
import urllib.request
import uuid

from tools import bash as tool_bash
from tools import edit as tool_edit
from tools import read as tool_read
from tools import write as tool_write

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
CONFIG_PATH = os.path.join(BASE_DIR, "config.json")
SESSIONS_DIR = os.path.join(BASE_DIR, ".ember", "sessions")
LAST_POINTER = os.path.join(BASE_DIR, ".ember", "last_session")
TIMEOUT_SECONDS = 180
RETRY_DELAYS = [2, 4, 8]
MAX_TOOL_ROUNDS = 20
HISTORY_CHAR_BUDGET = int(os.environ.get("EMBER_HISTORY_BUDGET", "64000"))
KEEP_RECENT_MSGS = 12  # 压缩时保留的最近消息条数（≈6 轮）

TOOLS_SCHEMA = [
    {
        "type": "function",
        "function": {
            "name": "bash",
            "description": (
                "执行 shell 命令并返回 stdout+stderr 合并输出。\n"
                "何时用: 运行程序、查看文件（grep/head/wc）、列目录、跑测试等。\n"
                "参数: command (string)。示例: bash(command=\"wc -l app.log\")。\n"
                "边界: 超时 60 秒；输出超 64KB 截断；失败也返回文本，不抛异常。\n"
                "错误解读: 输出含 'exit code: N'（N≠0 为失败）或 '[timeout]'。"
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
                "读取文件内容（文本），支持分页。\n"
                "何时用: 查看源码/日志/配置；大文件用 offset+limit 分块阅读。\n"
                "参数: path (string, 必填); offset (int, 可选, 起始行号 1 起, 缺省=第 1 行); limit (int, 可选, 读取行数, 缺省=读到文件尾)。\n"
                "示例: read(path=\"app.log\", offset=101, limit=50)。\n"
                "边界: 无 offset/limit 时超 64KB 截断；有分页时返回行号范围标注; 末尾不足 limit 时返回实际行数。\n"
                "错误解读: 返回以 '错误:' 开头表示失败。"
            ),
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"},
                    "offset": {"type": "integer", "description": "起始行号（1 起）；None 表示从第 1 行"},
                    "limit": {"type": "integer", "description": "读取行数；None 表示读到文件尾"},
                },
                "required": ["path"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "edit",
            "description": (
                "精确替换文件中的一段文本（大文件小改动首选，省 token）。\n"
                "何时用: 只改文件里的一小段（改配置值/改函数体/修一行），不要整文件重写。\n"
                "参数: path (string), old (string, 要被替换的原文, 必须唯一), new (string)。\n"
                "示例: edit(path=\"a.py\", old=\"timeout=60\", new=\"timeout=180\")。\n"
                "边界: old 必须在文件里**恰好出现一次**；出现 0 次或多处都会被拒绝（防误改）。\n"
                "错误解读: 返回以 '错误:' 开头——'未找到'→先 read 复制原文；'出现 N 次'→扩大 old 上下文。"
            ),
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"},
                    "old": {"type": "string", "description": "要被替换的原文（须唯一）"},
                    "new": {"type": "string", "description": "替换成的新文本"},
                },
                "required": ["path", "old", "new"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "write",
            "description": (
                "写文件（自动创建父目录），整体覆盖。\n"
                "何时用: 生成代码/报告等产物。参数: path, content (string)。\n"
                "示例: write(path=\"analyze.py\", content=\"...\")。\n"
                "边界: 覆盖写；content 为完整内容。错误解读: '错误:' 开头为失败。"
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

TOOL_IMPL = {"bash": tool_bash, "read": tool_read, "write": tool_write, "edit": tool_edit}


def load_config():
    try:
        with open(CONFIG_PATH, "r", encoding="utf-8") as f:
            cfg = json.load(f)
    except FileNotFoundError:
        print("错误: 缺少配置文件 %s" % CONFIG_PATH, file=sys.stderr)
        print("建议: 先创建 config.json，含三键 base_url / model / key。", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print("错误: config.json 不是合法 JSON (%s)" % e, file=sys.stderr)
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


# ── 会话持久化（M2） ──

def save_session(session_id, messages):
    """全量写入会话 JSONL + 更新 last_session 指针。"""
    try:
        os.makedirs(SESSIONS_DIR, exist_ok=True)
        path = os.path.join(SESSIONS_DIR, "%s.jsonl" % session_id)
        with open(path, "w", encoding="utf-8") as f:
            for m in messages:
                f.write(json.dumps(m, ensure_ascii=False) + "\n")
        with open(LAST_POINTER, "w", encoding="utf-8") as f:
            f.write(session_id)
        # M5: 断点可见——记录最后一条工具动作，供 -c 恢复时提示
        last_tool = next((m for m in reversed(messages) if m.get("role") == "tool"), None)
        if last_tool is not None:
            note = str(last_tool.get("content") or "")[:300]
            try:
                with open(os.path.join(BASE_DIR, ".ember", "last_action.txt"), "w",
                          encoding="utf-8") as f:
                    f.write(note)
            except OSError:
                pass
    except OSError as e:
        print("[warn] 会话落盘失败: %s" % e, file=sys.stderr)


def load_latest_session():
    """返回 (session_id, messages)；无历史 → (None, [])。"""
    try:
        with open(LAST_POINTER, "r", encoding="utf-8") as f:
            sid = f.read().strip()
    except OSError:
        sid = ""
    if not sid:
        # 指针缺失 → 退化取最新 jsonl
        try:
            files = [f for f in os.listdir(SESSIONS_DIR) if f.endswith(".jsonl")]
            if not files:
                return None, []
            newest = max(files, key=lambda f: os.path.getmtime(os.path.join(SESSIONS_DIR, f)))
            sid = newest[:-6]
        except OSError:
            return None, []
    path = os.path.join(SESSIONS_DIR, "%s.jsonl" % sid)
    messages = []
    try:
        with open(path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    messages.append(json.loads(line))
    except (OSError, json.JSONDecodeError) as e:
        print("[warn] 读取会话失败: %s" % e, file=sys.stderr)
        return None, []
    return (sid if messages else None), messages


def list_sessions():
    """列出会话：(id8, mtime, 首条 user 消息摘要)。"""
    rows = []
    try:
        for fn in os.listdir(SESSIONS_DIR):
            if not fn.endswith(".jsonl"):
                continue
            p = os.path.join(SESSIONS_DIR, fn)
            summary = ""
            try:
                with open(p, "r", encoding="utf-8") as f:
                    for line in f:
                        try:
                            m = json.loads(line)
                        except json.JSONDecodeError:
                            continue
                        if m.get("role") == "user":
                            summary = (m.get("content") or "")[:40].replace("\n", " ")
                            break
            except OSError:
                pass
            rows.append((fn[:-6], os.path.getmtime(p), summary))
    except OSError:
        return []
    rows.sort(key=lambda r: -r[1])
    return rows


def chat_plain(cfg, messages, max_tokens=800):
    """不带工具 schema 的纯文本调用（摘要等内部用途）。"""
    url = cfg["base_url"].rstrip("/") + "/chat/completions"
    body = json.dumps({
        "model": cfg["model"],
        "messages": messages,
        "max_tokens": max_tokens,
        "temperature": 0,
    }).encode("utf-8")
    req = urllib.request.Request(url, data=body, method="POST", headers={
        "Content-Type": "application/json",
        "Authorization": "Bearer %s" % cfg["key"],
    })
    with urllib.request.urlopen(req, timeout=TIMEOUT_SECONDS) as resp:
        payload = json.loads(resp.read().decode("utf-8"))
    ch = payload.get("choices") or []
    if not ch:
        raise RuntimeError("摘要调用无 choices")
    return (ch[0].get("message") or {}).get("content") or ""


def _summarize_messages(cfg, dropped):
    """把被压缩掉的中间消息概括成 ≤400 字要点；失败返回 None（调用方兜底）。"""
    if not dropped:
        return None
    body = []
    for m in dropped:
        role = m.get("role", "?")
        txt = m.get("content") or ""
        if not isinstance(txt, str):
            txt = json.dumps(txt, ensure_ascii=False)
        body.append("%s: %s" % (role, txt[:1500]))
    prompt = ("把下面这段对话历史压缩成不超过 400 字的要点，覆盖：做过什么、产物文件路径、"
              "已知结论、未完事项。只输出摘要正文，不要客套：\n\n" + "\n".join(body)[:20000])
    try:
        out = chat_plain(cfg, [{"role": "user", "content": prompt}])
        out = (out or "").strip()
        return out if out else None
    except Exception:
        return None


def compress_history(cfg, messages):
    """M5：超预算时智能压缩——首条目标 + 最近若干条原文 + 中间历史摘要（模型生成）。
    摘要失败则退回'丢最旧'（不中断任务），两种情况都向 stderr 投影。"""
    def size(ms):
        return sum(len(json.dumps(m, ensure_ascii=False)) for m in ms)

    total = size(messages)
    if total <= HISTORY_CHAR_BUDGET:
        return messages
    first_user = next((i for i, m in enumerate(messages) if m.get("role") == "user"), 0)
    head = messages[:first_user + 1]
    rest = messages[first_user + 1:]
    if len(rest) <= KEEP_RECENT_MSGS:
        return messages
    recent = rest[-KEEP_RECENT_MSGS:]
    middle = rest[:-KEEP_RECENT_MSGS]
    summary = _summarize_messages(cfg, middle)
    if summary:
        compacted = head + [{"role": "user",
                             "content": "[历史摘要] " + summary}] + recent
        print("[compact] 压缩 %d 条 → 摘要 %d 字（预算 %d，压缩前 %d 字）"
              % (len(middle), len(summary), HISTORY_CHAR_BUDGET, total), file=sys.stderr)
    else:
        compacted = head + recent
        print("[compact] 摘要调用失败——退回丢最旧（丢弃 %d 条，预算 %d，压缩前 %d 字）"
              % (len(middle), HISTORY_CHAR_BUDGET, total), file=sys.stderr)
    return compacted


def trim_history(messages, budget=HISTORY_CHAR_BUDGET):
    """预算控制：总量超 budget 时从最旧丢（保留首条 user 目标消息）。"""
    def size(ms):
        return sum(len(json.dumps(m, ensure_ascii=False)) for m in ms)

    if size(messages) <= budget or len(messages) <= 2:
        return messages
    first_user = next((m for m in messages if m.get("role") == "user"), None)
    kept = list(messages)
    while size(kept) > budget and len(kept) > 2:
        # 从最旧开始丢（跳过首条 user 目标）
        dropped = False
        for i in range(1, len(kept)):
            if kept[i] is not first_user:
                del kept[i]
                dropped = True
                break
        if not dropped:
            break
    return kept


# ── 模型调用 ──

def chat_once(cfg, messages):
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
    raise RuntimeError("重试 3 次后仍失败，最后一次错误: %s" % last_err)


def run_tool(name, raw_args):
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
    except Exception as e:
        return "错误: 工具 %s 执行异常: %s" % (name, e)


def solve(cfg, question, history=None):
    """工具循环；返回 (answer_text, full_messages)。"""
    messages = list(history) if history else []
    messages.append({"role": "user", "content": question})
    messages = compress_history(cfg, messages)
    empty_nudges = 0
    for _ in range(MAX_TOOL_ROUNDS):
        msg = call_model(cfg, messages)
        tool_calls = msg.get("tool_calls") or []
        content = msg.get("content") or ""
        if tool_calls:
            messages.append({"role": "assistant", "content": content, "tool_calls": tool_calls})
            for tc in tool_calls:
                fn_info = tc.get("function") or {}
                name = fn_info.get("name") or ""
                raw_args = fn_info.get("arguments")
                t0 = time.time()
                print("[tool] %s(%s)" % (name, (str(raw_args) or "")[:160]), file=sys.stderr)
                result = run_tool(name, raw_args)
                print("[tool] ← %s（%.1fs，%d 字符）" % (name, time.time() - t0, len(result)), file=sys.stderr)
                messages.append({"role": "tool", "tool_call_id": tc.get("id") or "", "content": result})
            continue
        if content.strip():
            messages.append({"role": "assistant", "content": content})
            return content, messages
        empty_nudges += 1
        if empty_nudges > 2:
            return "（模型连续返回空正文——可能 thinking 占满输出预算，请重试）", messages
        messages.append({"role": "user", "content": "请直接给出最终答案（不要再调用工具）。"})
    return "（达到 %d 轮工具循环上限，任务未收敛——请缩小问题范围重试）" % MAX_TOOL_ROUNDS, messages


USAGE = """用法:
  python3 ember.py "问题"        # 新会话（带工具循环）
  python3 ember.py -c "追问"     # 继续最近会话（记得上次的活）
  python3 ember.py --list        # 列出历史会话"""


def main():
    args = sys.argv[1:]
    if not args:
        print(USAGE, file=sys.stderr)
        sys.exit(1)

    # --list
    if args[0] in ("--list", "-l"):
        rows = list_sessions()
        if not rows:
            print("（暂无历史会话）")
            return
        print("历史会话（%d 个）:" % len(rows))
        for sid, mtime, summary in rows[:20]:
            ts = time.strftime("%m-%d %H:%M", time.localtime(mtime))
            print("  %s  %s  %s" % (sid[:8], ts, summary))
        return

    # -c / --continue
    if args[0] in ("-c", "--continue"):
        if len(args) < 2 or not args[1].strip():
            print("用法: python3 ember.py -c \"追问\"", file=sys.stderr)
            sys.exit(1)
        question = args[1].strip()
        sid, history = load_latest_session()
        if not history:
            print("[info] 未找到历史会话——按新会话处理", file=sys.stderr)
            sid = None
        elif sid:
            print("[info] 已加载会话 %s（%d 条历史消息）" % (sid[:8], len(history)), file=sys.stderr)
        # M5: 断点提示——上次中断时的最后动作，注入为首条追问的前置上下文
        if history:
            try:
                with open(os.path.join(BASE_DIR, ".ember", "last_action.txt"),
                          "r", encoding="utf-8") as f:
                    note = f.read().strip()
                if note:
                    history.append({"role": "user",
                                    "content": "[断点提示] 上次中断时的最后动作（供你接着做）: " + note})
                    print("[resume] 已注入断点提示（%d 字）" % len(note), file=sys.stderr)
            except OSError:
                pass
        cfg = load_config()
        try:
            answer, messages = solve(cfg, question, history)
        except RuntimeError as e:
            print("错误: %s" % e, file=sys.stderr)
            sys.exit(1)
        session_id = sid or str(uuid.uuid4())
        save_session(session_id, messages)
        print(answer)
        return

    # 默认：新会话
    if len(args) != 1 or not args[0].strip():
        print(USAGE, file=sys.stderr)
        sys.exit(1)
    question = args[0].strip()
    cfg = load_config()
    try:
        answer, messages = solve(cfg, question)
    except RuntimeError as e:
        print("错误: %s" % e, file=sys.stderr)
        if "认证失败" in str(e):
            print("建议: 检查 config.json 的 key 或环境变量 APIHUB_AGNES_AI_API_KEY。", file=sys.stderr)
        else:
            print("建议: 检查网络连通性（目标 %s）与 base_url 配置；稍后可重试。" % cfg["base_url"], file=sys.stderr)
        sys.exit(1)
    except KeyboardInterrupt:
        print("\n已中断（会话已保存）。", file=sys.stderr)
        sys.exit(130)
    session_id = str(uuid.uuid4())
    save_session(session_id, messages)
    print(answer)


if __name__ == "__main__":
    main()
