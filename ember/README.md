# EMBER-M0 · 单轮问答 CLI

用法：
```bash
python3 ember.py "你的问题"
```
首次运行提示生成 `config.json`（从环境变量 APIHUB_AGNES_AI_API_KEY 读取，权限 600，已加入 .gitignore 永不入库）。

仅支持 Chat Completions 协议，非流式；内置 3 次指数退避重试（2/4/8s），超时 60s。
