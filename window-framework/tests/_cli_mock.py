#!/usr/bin/env python3
"""Zero-token mock SSE service for codex-cli UX verification (Y2).
Scenarios: 1=normal done, 2=approval pending, 3=error then resume.
"""
import json, sys, time
from http.server import BaseHTTPRequestHandler, HTTPServer

SCENARIO = int(sys.argv[1]) if len(sys.argv) > 1 else 1


class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def _send_sse(self, events):
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        for et, data in events:
            self.wfile.write(f"event: {et}\n".encode())
            self.wfile.write(f"data: {json.dumps(data)}\n".encode())
            self.wfile.write(b"\n")
            self.wfile.flush()
            time.sleep(0.05)

    def do_POST(self):
        if self.path == "/api/v1/sessions":
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"id": "mock-sess-1"}).encode())
        elif self.path == "/api/v1/sessions/mock-sess-1/messages":
            if SCENARIO == 1:
                self._send_sse([
                    ("Phase", "plan"),
                    ("ToolCall", {"name": "bash", "args": {"cmd": "ls"}}),
                    ("ToolResult", {"output": "file1\nfile2"}),
                    ("Done", {"steps": 3, "ok": True}),
                ])
            elif SCENARIO == 2:
                self._send_sse([
                    ("Phase", "act"),
                    ("ToolCall", {"name": "bash", "args": {"cmd": "rm old.txt"}}),
                    ("need_approval", {"approval_id": "ap-1", "action": "rm old.txt"}),
                ])
            elif SCENARIO == 3:
                self._send_sse([
                    ("Phase", "plan"),
                    ("ToolCall", {"name": "bash", "args": {"cmd": "cargo build"}}),
                    ("Error", "compile error: unresolved import"),
                ])
            else:
                self._send_sse([("Done", {"steps": 1, "ok": True})])
        elif self.path == "/api/v1/sessions/mock-sess-1":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"id": "mock-sess-1", "status": "created"}).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def do_GET(self):
        if self.path == "/healthz":
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.end_headers()
            self.wfile.write(b"OK")
        elif self.path == "/api/v1/sessions/mock-sess-1":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"id": "mock-sess-1", "status": "created"}).encode())
        else:
            self.send_response(404)
            self.end_headers()


HTTPServer(("127.0.0.1", int(sys.argv[2]) if len(sys.argv) > 2 else 3999), H).serve_forever()
