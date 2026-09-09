# 给 Gemini 的运行指令 v2（已按环境确认校准）

> 环境已确认：你 env 里**已有 codex-rust 代码**；用 **gemini 自己当 provider**（你自己的 key）；
> **先 `cargo test` 全绿，再跑完整 bench 基准**；**只报告结果，不改源码**。

---

## 执行步骤（在你环境里直接跑）

1. **进入代码目录**：`cd` 到已有的 codex-rust 仓库根。
2. **本地门禁（不需要任何密钥）**：
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
   预期全绿（~146 单测 + xray 14 条 wiring 断言）。这是"代码本身没坏"的底线。
3. **全量基准（gemini provider）**：
   ```bash
   # 确保 .env 里有你的 gemini key（GEMINI_API_KEY 或对应字段）
   cargo build
   cd codex_work && RUST_LOG=debug ./target/debug/service > /tmp/svc.log 2>&1 &
   # 等 /readyz 起来
   curl -s http://localhost:3000/readyz
   BENCH_PROVIDER=gemini python bench/runner.py batch --runs 1 --resume
   ```
   这会真实驱动 T00–T19（以及已落地的 T24），每个 task 经 `verify.sh` 判 `VERIFY_PASS`/`FAIL`。
4. **报告真实结果**：每个 task 的判定、失败原因、`SUMMARY: X/Y passed (Z%)` 整体通过率、关键日志片段（如 `/tmp/svc.log` 里 `tool=` 与报错行）。

---

## 红线

- **禁止建前端 / 看板 / UI。** 交付物 = 真实测试输出 + 你的结论文本。
- **只报告，不改码。** 发现阻断性 bug（例如已知 `do_plan_inner` 对 LLM 错误零重试，T12 实测 6 连败）只报原因与疑似根因，**不要编辑源码**。
- 任一步跑不了（key 缺失 / 网络不通 / 编译挂），**明确说出来**，不要编造结果。

---

## 一句话总结

`cd` 进仓库 → `cargo test` 全绿 → 起服务 → `BENCH_PROVIDER=gemini python bench/runner.py batch` → 把真实 VERIFY_PASS/FAIL 贴回来。完事。
