#!/usr/bin/env python3
"""M1-1 录制器契约门禁：结构合法 + 脱敏 + 可回放性前置。

守三类失败：
  ① **结构**：录出的 fixture 必须能被 llm-replay 解析（`meta`/`history.goal`/
     `history.messages`，且至少含一条 tool_result 边界的 turn）——否则回放侧
     `parse_fixture` 返回 None，表现是"回放跑绿但 agent 什么都没做"；
  ② **脱敏**：fixture 不许带本机绝对路径（实测 `tool_result.output` 会回显
     `wrote N bytes to C:\\Users\\<user>\\…`）——公开仓入库卫生；
  ③ **保真元数据**：`meta.live_verify_ok` 必须在场（`replay_fidelity.py` 拿它当对照基准）；
     缺失等于"录了但没记当时的判据结论"，保真检查就退化成自己跟自己比。

跑法：python -m unittest discover -s bench/tests -p "test_*.py"
"""
import json
import pathlib
import sys
import unittest

BENCH = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(BENCH))

import run_local  # noqa: E402

LIVE_DIR = BENCH / "replay" / "live"


class TestSanitizeTranscript(unittest.TestCase):
    def test_work_root_and_home_prefixes_are_redacted(self):
        work = pathlib.Path.home() / "proj" / "target" / "local-run"
        doc = {
            "messages": [
                {"event_type": "tool_result",
                 "payload": {"output": f"wrote 12 bytes to {work}\\sessions\\abc\\ANSWER.md"}},
                {"event_type": "tool_result",
                 "payload": {"output": f"read {str(work).replace(chr(92), '/')}/sessions/abc/x.rs"}},
                {"event_type": "phase", "payload": {"phase": "Plan"}},
            ]
        }
        out = run_local.sanitize_transcript(doc, work)
        blob = json.dumps(out, ensure_ascii=False)
        self.assertNotIn(str(work), blob, "工作区绝对前缀必须被替换")
        self.assertNotIn(str(pathlib.Path.home()), blob, "家目录绝对前缀必须被替换")
        # 长前缀优先 ⇒ 工作区应落成 <WORK>，而不是被 <HOME> 抢先截断
        self.assertIn("<WORK>", blob)
        # 非路径字段原样保留（脱敏不许改语义）
        self.assertEqual(out["messages"][2]["payload"]["phase"], "Plan")

    def test_no_absolute_paths_left_in_plain_windows_form(self):
        work = pathlib.Path.home() / "ws"
        out = run_local.sanitize_transcript(
            {"s": f"x {pathlib.Path.home()}\\a\\b y"}, work
        )
        self.assertEqual(out["s"], "x <HOME>\\a\\b y")


class TestRecordedFixtures(unittest.TestCase):
    """对**已入库**的实录 fixture 做一次静态体检（没有实录时跳过并如实打印）。"""

    def setUp(self):
        self.files = sorted(LIVE_DIR.glob("*.json")) if LIVE_DIR.is_dir() else []

    def test_live_traces_exist(self):
        self.assertGreaterEqual(
            len(self.files), 3, "M1-1 要求至少 3 条真实 trace（bench/replay/live/）"
        )

    def test_schema_and_metadata(self):
        for p in self.files:
            with self.subTest(fixture=p.name):
                d = json.loads(p.read_text(encoding="utf-8"))
                meta, hist = d.get("meta") or {}, d.get("history") or {}
                self.assertIsInstance(hist.get("goal"), str, "history.goal 必须是字符串")
                self.assertTrue(hist["goal"].strip(), "history.goal 不得为空")
                msgs = hist.get("messages")
                self.assertIsInstance(msgs, list, "history.messages 必须是数组")
                self.assertGreater(len(msgs), 0, "history.messages 不得为空")
                self.assertIn("live_verify_ok", meta, "必须记录活体时的判据结论（保真对照基准）")
                self.assertEqual(meta.get("fixture_schema"), 1)
                self.assertTrue(str(meta.get("source", "")).startswith("live-"))
                events = {m.get("event_type") for m in msgs}
                self.assertIn("tool_result", events, "至少要有一次工具结果（否则回放无 turn）")
                self.assertIn("tool_call", events, "至少要有一次工具调用")

    def test_no_local_absolute_paths(self):
        # **注意 JSON 转义**：文件里 Windows 路径长成 `C:\\Users\\<user>`（双反斜杠），
        # 只查 `C:\Users\<user>`（单反斜杠）会**漏判**——本门禁初版就漏了，实测靠
        # 「手工 grep 发现 fixture 里有 Users」才暴露。三种形态都查：原样 / JSON 转义 / 正斜杠。
        home = pathlib.Path.home()
        needles = {
            str(home),
            json.dumps(str(home))[1:-1],  # 转义形态（双反斜杠）
            str(home).replace("\\", "/"),
        }
        needles.discard("")
        for p in self.files:
            with self.subTest(fixture=p.name):
                blob = p.read_text(encoding="utf-8")
                leaked = [n for n in needles if n in blob]
                self.assertEqual(
                    leaked, [], f"{p.name} 带本机家目录前缀 ⇒ 录制脱敏漏了"
                )


if __name__ == "__main__":
    unittest.main()
