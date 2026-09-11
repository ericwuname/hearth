#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Google Gemini 3.5 Flash 对话工具（美化版 + 思考动画）
支持彩色对话气泡、限流自动重试、倒计时进度条、旋转光标
"""

import os
import sys
import json
import time
import re
import threading
import requests


# ========== 终端颜色支持 ==========
try:
    if os.name == 'nt':
        import ctypes
        kernel32 = ctypes.windll.kernel32
        kernel32.SetConsoleMode(kernel32.GetStdHandle(-11), 7)
except:
    pass

COLOR_RESET = "\033[0m"
COLOR_GREEN = "\033[92m"
COLOR_BLUE = "\033[94m"
COLOR_YELLOW = "\033[93m"
COLOR_RED = "\033[91m"
COLOR_CYAN = "\033[96m"
COLOR_BOLD = "\033[1m"
COLOR_DIM = "\033[2m"

def supports_color():
    if os.name == 'nt':
        return True
    return hasattr(sys.stdout, 'isatty') and sys.stdout.isatty()

USE_COLOR = supports_color()

def color(text, code):
    if USE_COLOR:
        return code + text + COLOR_RESET
    return text


# ========== API Key ==========
def get_api_key():
    api_key = "AQ.<REDACTED-GEMINI-KEY>"
    if not api_key:
        print(color("❌ 错误：未找到 GEMINI_API_KEY。请设置环境变量。", COLOR_RED))
        print(color("   示例: $env:GEMINI_API_KEY='你的Key'", COLOR_YELLOW))
        sys.exit(1)
    return api_key


# ========== 倒计时进度条 ==========
def countdown(seconds, message=""):
    bar_length = 25
    for i in range(seconds, 0, -1):
        filled = int((seconds - i + 1) / seconds * bar_length)
        bar = "█" * filled + "░" * (bar_length - filled)
        sys.stdout.write(f"\r{color('⏳', COLOR_YELLOW)} {message} {color(f'{i:2d}s', COLOR_YELLOW)} [{bar}]")
        sys.stdout.flush()
        time.sleep(1)
    sys.stdout.write("\r" + " " * 90 + "\r")
    sys.stdout.flush()


# ========== 旋转光标动画 ==========
class Spinner:
    def __init__(self, message="🤔 思考中"):
        self.chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
        self.stop_event = threading.Event()
        self.thread = threading.Thread(target=self._spin, daemon=True)
        self.message = message

    def _spin(self):
        idx = 0
        while not self.stop_event.is_set():
            sys.stdout.write(f"\r{color(self.chars[idx % len(self.chars)], COLOR_YELLOW)} {color(self.message, COLOR_DIM)}")
            sys.stdout.flush()
            time.sleep(0.1)
            idx += 1
        sys.stdout.write("\r" + " " * 70 + "\r")
        sys.stdout.flush()

    def start(self):
        self.thread.start()

    def stop(self):
        self.stop_event.set()
        self.thread.join(timeout=1)


# ========== Gemini API 调用（含 spinner）==========
def ask_gemini(prompt, model="gemini-3.5-flash", max_retries=3):
    api_key = get_api_key()
    url = "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
    headers = {"Content-Type": "application/json", "Authorization": f"Bearer {api_key}"}
    payload = {"model": model, "messages": [{"role": "user", "content": prompt}]}

    for attempt in range(1, max_retries + 1):
        try:
            spinner = Spinner(f"第 {attempt} 次请求中...")
            spinner.start()

            response = requests.post(url, headers=headers, json=payload, timeout=30)
            spinner.stop()

            if response.status_code == 429:
                wait_time = 45
                try:
                    err_msg = response.json().get("error", {}).get("message", "")
                    match = re.search(r"retry in (\d+\.?\d*)s", err_msg)
                    if match:
                        wait_time = float(match.group(1)) + 3
                except:
                    pass
                wait_time = min(int(wait_time), 120)
                print(color(f"\n⚠️ 触发限流（429），第 {attempt}/{max_retries} 次重试", COLOR_YELLOW))
                countdown(wait_time, "等待限流解除")
                continue

            response.raise_for_status()
            data = response.json()
            return data["choices"][0]["message"]["content"]

        except requests.exceptions.Timeout:
            spinner.stop()
            print(color(f"\n⏱️ 请求超时，第 {attempt}/{max_retries} 次重试...", COLOR_YELLOW))
            if attempt < max_retries:
                countdown(5, "5秒后重试")
            else:
                print(color("❌ 多次超时，放弃请求。", COLOR_RED))
                return None

        except requests.exceptions.RequestException as e:
            spinner.stop()
            print(color(f"\n❌ 网络请求失败: {e}", COLOR_RED))
            if hasattr(e, 'response') and e.response is not None:
                print(color(f"HTTP 状态码: {e.response.status_code}", COLOR_RED))
                print(color(f"响应内容: {e.response.text[:300]}", COLOR_DIM))
            if attempt < max_retries:
                countdown(10, "10秒后重试")
            else:
                print(color("❌ 多次失败，放弃请求。", COLOR_RED))
                return None

        except (KeyError, IndexError, json.JSONDecodeError) as e:
            spinner.stop()
            print(color(f"\n❌ 解析响应失败: {e}", COLOR_RED))
            print(color(f"原始响应: {response.text[:300]}", COLOR_DIM))
            return None

    print(color("❌ 超过最大重试次数，请求失败。", COLOR_RED))
    return None


# ========== 对话气泡 ==========
def print_user_bubble(text):
    lines = text.split('\n')
    print(color(f"{COLOR_BOLD}┌─ 你 ───────────────────────────────", COLOR_GREEN))
    for line in lines:
        print(color(f"│ {line}", COLOR_GREEN))
    print(color(f"└────────────────────────────────────", COLOR_GREEN))

def print_ai_bubble(text):
    lines = text.split('\n')
    print(color(f"{COLOR_BOLD}┌─ Gemini ────────────────────────────", COLOR_BLUE))
    for line in lines:
        print(color(f"│ {line}", COLOR_BLUE))
    print(color(f"└────────────────────────────────────", COLOR_BLUE))


# ========== 主界面 ==========
def print_header():
    header = f"""
{color('╔══════════════════════════════════════╗', COLOR_CYAN)}
{color('║', COLOR_CYAN)}   {color('✨ Google Gemini 3.5 Flash ✨', COLOR_BOLD + COLOR_YELLOW)}   {color('║', COLOR_CYAN)}
{color('║', COLOR_CYAN)}      {color('智能对话 · 思考动画 · 自动重试', COLOR_DIM)}      {color('║', COLOR_CYAN)}
{color('╚══════════════════════════════════════╝', COLOR_CYAN)}
    """
    print(header)
    print(color("输入问题，按 Enter 发送。输入 ", COLOR_DIM) + color("exit", COLOR_RED) + color(" 退出。\n", COLOR_DIM))


def main():
    print_header()
    while True:
        try:
            raw_input = input(color(">>> ", COLOR_GREEN))
        except (EOFError, KeyboardInterrupt):
            print(color("\n👋 再见！", COLOR_CYAN))
            break

        user_text = raw_input.strip()
        if user_text.lower() in ("exit", "quit", "q"):
            print(color("👋 再见！", COLOR_CYAN))
            break
        if not user_text:
            continue

        print_user_bubble(user_text)
        reply = ask_gemini(user_text)
        if reply:
            print_ai_bubble(reply)
        else:
            print(color("（无法获取回复，请稍后再试）", COLOR_RED))
        print()


if __name__ == "__main__":
    main()