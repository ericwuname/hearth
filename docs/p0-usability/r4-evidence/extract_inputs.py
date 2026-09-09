# -*- coding: utf-8 -*-
"""R4 双会话重放 · 用户轮次提取（预注册：逐字提取不润色，弱指令保留原样）
产出: r4-evidence/replay/inputs_A.json / inputs_B.json（sha256 记录于文件头注释）
"""
import io, json, hashlib, sys, os

BASE = r"C:\Users\87465\Desktop\codex-rust-v1.0-final\release"
OUT = r"C:\Users\87465\Desktop\codex-rust-v1.0-final\docs\p0-usability\r4-evidence\replay"

def extract(path):
    """提取语料中全部用户轮次（hearth> 〉 前缀行），逐字保留。"""
    inputs = []
    with io.open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            s = line.rstrip("\r\n")
            if s.startswith("hearth> 〉"):
                u = s[len("hearth> 〉"):].strip()
                if u:
                    inputs.append(u)
    return inputs

def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()

os.makedirs(OUT, exist_ok=True)
for tag, fn in [("A", "手工测试v0.2.23-2026-0.9-0.3.txt"), ("B", "手工测试v0.2.23-2026-0.9-0.4.txt")]:
    p = os.path.join(BASE, fn)
    inputs = extract(p)
    digest = sha256(p)
    out = os.path.join(OUT, f"inputs_{tag}.json")
    payload = {
        "corpus_file": fn,
        "corpus_sha256": digest,
        "extraction_rule": "lines starting with 'hearth> 〉', verbatim (no rewrite)",
        "turn_count": len(inputs),
        "inputs": inputs,
    }
    with io.open(out, "w", encoding="utf-8") as f:
        json.dump(payload, f, ensure_ascii=False, indent=1)
    print(f"[{tag}] {fn} sha256={digest[:16]}… turns={len(inputs)} -> {out}")
    for i, u in enumerate(inputs[:6]):
        print(f"   {i+1}: {u[:60]}")
    if len(inputs) > 6:
        print(f"   … 共 {len(inputs)} 轮")
