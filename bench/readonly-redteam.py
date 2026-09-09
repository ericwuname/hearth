#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""RT4-R3: readonly 裁剪验证探针（sandbox 直测，VM 真跑）。

验证：
1. sandbox 内写 /tmp 外路径（readonly "/"）→ 被拒（EROFS/EPERM）
2. sandbox 内写 writable 路径（/tmp）→ 成功（landlock 规则正确区分）
3. glob（Rust 遍历）不穿透 symlink（防逃逸）

用法：cargo test -p sandbox test_rt4_readonly_probes（并入 sandbox 单测）。
"""
# 该文件是文档性入口——实际探针以 sandbox 单测 test_rt4_readonly_probes 实现
print("RT4-R3: 见 crates/sandbox/src/lib.rs test_rt4_readonly_probes（sandbox 单测）")
