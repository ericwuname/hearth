#!/usr/bin/env python3
"""EMBER M0 — 单轮问答 CLI（标准库实现）。

用法: python3 ember.py "问题"
协议: POST {base_url}/chat/completions（Agnes OpenAI 兼容）
"""
import json
import os
import sys
import time
import urllib.error
import urllib.request

CONFIG_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "config.json")
TIMEOUT_SECONDS = 60
RETRY_DELAYS = [2, 4, 8]  # 指数退避 2/4/8s，共 3 次重试（最多 4 次请求）


def load_config():
    """读取 config.json；若 key 为空则从环境变量 APIHUB_AGNES_AI_API_KEY 补全并写回（保持 600 权限）。"""
    try:
        with open(CONFIG_PATH, "r", encoding="utf-8") as f:
            cfg = json.load(f)
    except FileNotFoundError:
        print("错误: 缺少配置文件 %s" % CONFIG_PATH, file=sys.stderr)
        print("建议: 先创建 config.json，含三键 base_url / model / key（key 可留空，运行时会从环境变量 APIHUB_AGNES_AI_API_KEY 读取）。", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print("错误: config.json 不是合法 JSON (%s)" % e, file=sys.stderr)
        print("建议: 检查文件内容（base_url/model/key 三键，UTF-8）。", file=sys.stderr)
        sys.exit(1)
    for field in ("base_url", "model", "key"):
        if field not in cfg:
            print("错误: config.json 缺少键 '%s'" % field, file=sys.stderr)
            print("建议: 文件需包含 base_url / model / key 三个键。", file=sys.stderr)
            sys.exit(1)
    if not cfg.get("key"):
        env_key = os.environ.get("APIHUB_AGNES_AI_API_KEY", "").strip()
        if not env_key:
            print("错误: config.json 中 key 为空，且环境变量 APIHUB_AGNES_AI_API_KEY 未设置。", file=sys.stderr)
            print("建议: 先 export APIHUB_AGNES_AI_API_KEY=... 再运行（或手动填入 config.json，注意该文件权限须为 600、不入 git）。", file=sys.stderr)
            sys.exit(1)
        cfg["key"] = env_key
        try:
            with open(CONFIG_PATH, "w", encoding="utf-8") as f:
                json.dump(cfg, f, ensure_ascii=False, indent=2)
            os.chmod(CONFIG_PATH, 0o600)
        except OSError:
            pass  # 写回失败不影响本次运行（内存中已有 key）
    return cfg


def chat_once(cfg, question, attempt):
    """单次请求；返回答案文本。失败抛异常（URLError/HTTPError）。"""
    url = cfg["base_url"].rstrip("/") + "/chat/completions"
    body = json.dumps({
        "model": cfg["model"],
        "messages": [{"role": "user", "content": question}],
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
        raise RuntimeError("响应缺少 choices（HTTP %s，前 200 字符: %s...）" % (resp.status, json.dumps(payload, ensure_ascii=False)[:200]))
    return (choices[0].get("message") or {}).get("content") or ""


def ask(cfg, question):
    last_err = None
    for i, delay in enumerate([None] + RETRY_DELAYS):
        try:
            return chat_once(cfg, question, i)
        except urllib.error.HTTPError as e:
            detail = ""
            try:
                detail = e.read().decode("utf-8", "replace")[:300]
            except Exception:
                pass
            if e.code in (401, 403):  # 鉴权失败不重试
                raise RuntimeError("认证失败 (HTTP %s): %s" % (e.code, detail)) from None
            last_err = "HTTP %s %s" % (e.code, detail)
        except urllib.error.URLError as e:
            last_err = "网络不可达: %s" % e.reason
        except (TimeoutError, RuntimeError, json.JSONDecodeError, OSError) as e:
            last_err = str(e)
        if delay is None:
            delay = RETRY_DELAYS[0]
        if i + 1 < len(RETRY_DELAYS) + 1:  # 还有下次
            print("请求失败（第 %d 次）: %s — 等待 %ds 后重试…" % (i + 1, last_err, delay), file=sys.stderr)
            time.sleep(delay)
        else:
            print("请求失败（第 %d 次）: %s" % (i + 1, last_err), file=sys.stderr)
    raise RuntimeError("重试 %d 次后仍失败，最后一次错误: %s" % (len(RETRY_DELAYS), last_err))


def main():
    if len(sys.argv) != 2 or not sys.argv[1].strip():
        print("用法: python3 ember.py \"问题\"", file=sys.stderr)
        sys.exit(1)
    question = sys.argv[1].strip()
    cfg = load_config()
    try:
        answer = ask(cfg, question)
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
