# 给 Gemini 的运行指令 v1（直接跑，不是设计）

> 纠正：之前让你"设计测试 / 出报告 / 出文件"全部是错的。你有自己的执行环境，
> 直接把 codex-rust 的整套测试跑起来、报告真实绿/红结果即可。不要规格、不要文件树、不要前端。

---

## 你要做什么

在**你自己的环境**里，把 codex-rust 的真实测试套件跑通，返回**实际输出**（pass/fail + 日志），不是一份设计文档，也不是一个看板页面。

---

## 执行步骤（在你环境里直接跑）

1. **拿到代码**：clone 仓库，或你环境里已有的 codex-rust 副本。
2. **本地门禁（不需要任何密钥）**：
   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
   预期全绿（~146 单测 + xray 14 条 wiring 断言）。这是"代码本身没坏"的底线。
3. **全量基准（需要 provider key，放 `.env`）**：
   ```bash
   cp .env.example .env   # 填入你的 provider key（deepseek / gemini / zhipu）
   cargo build
   cd codex_work && RUST_LOG=debug ./target/debug/service > /tmp/svc.log 2>&1 &
   python bench/runner.py batch --runs 1 --resume
   ```
   这会真实驱动 T00–T19（以及已落地的 T24），每个 task 经 `verify.sh` 判 `VERIFY_PASS`/`FAIL`。
4. **报告**：每个 task 的判定、失败原因、`SUMMARY: X/Y passed (Z%)` 整体通过率、关键日志片段。

---

## 红线

- **禁止建前端 / 看板 / UI。** 交付物 = 真实测试输出 + 你的结论文本。
- **禁止只写"测试设计报告"。** 你跑了才有测试；没跑就说没跑。
- 任一步跑不了（缺 key / 网络不通 / 编译挂），**明确说出来**，不要编造结果。
- 若发现阻断性 bug 导致不绿（例如已知 `do_plan_inner` 对 LLM 错误零重试，T12 实测 6 连败），**你可以修代码使其变绿**，但必须在报告里写清改了哪几个文件、为什么。

---

## 一句话总结

`cargo test` 先全绿 → 起服务 → `bench/runner.py batch` 跑全量 → 把真实 VERIFY_PASS/FAIL 贴回来。完事。
