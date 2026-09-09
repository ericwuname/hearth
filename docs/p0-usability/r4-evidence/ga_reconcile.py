# -*- coding: utf-8 -*-
"""G-A/W-A 对账：10 题各自的回答首段（hearth> 〉 之后到下一 hearth> 之前的首个文本块）"""
import io, re, json

lines = io.open(r"C:\Users\87465\Desktop\codex-rust-v1.0-final\docs\p0-usability\r4-evidence\replay\r4-ga-blind.log", encoding="utf-8", errors="replace").read().splitlines()
inputs = json.load(io.open(r"C:\Users\87465\Desktop\codex-rust-v1.0-final\docs\p0-usability\r4-evidence\replay\inputs_GA.json", encoding="utf-8"))["inputs"]

# 按 hearth> 〉 分段
segments = []  # (input_text, [answer lines])
cur_in = None
cur_ans = []
for ln in lines:
    m = re.match(r"hearth> 〉(.*)", ln)
    if m:
        if cur_in is not None:
            segments.append((cur_in, cur_ans))
        cur_in = m.group(1).strip()
        cur_ans = []
    elif cur_in is not None:
        # 跳过纯控制行
        if ln.strip():
            cur_ans.append(ln)
if cur_in is not None:
    segments.append((cur_in, cur_ans))

out = io.open(r"C:\Users\87465\Desktop\codex-rust-v1.0-final\docs\p0-usability\r4-evidence\analysis-GA-draft.md", "w", encoding="utf-8")
out.write("# R4 证据 · G-A/W-A 盲测 10 题对账（执行窗机器提取，首句判定归砺）\n\n")
out.write("> 引擎 70730e7｜.133｜BP-2 全新 session（单 session 10 轮连续）｜日志 r4-ga-blind.log\n\n")
out.write("| # | 输入（前 30 字） | 回答首 2 行（去 ANSI 前缀符号） | 初筛 |\n|---|---|---|---|\n")
clean = lambda s: re.sub(r"\x1b\[[0-9;?]*[A-Za-z]", "", s).strip()
direct_kw = ["完成", "失败", "正常", "通过", "是", "有", "没", "问题", "错误", "进度", "情况", "原因", "数量", "个", "目前", "当前", "已在", "测试", "项目"]
count_direct = 0
for i, (inp, ans) in enumerate(segments):
    ans_text = [clean(x) for x in ans if clean(x) and not clean(x).startswith(("⚙", "📄", "💭", "▸", "◂"))]
    first2 = " ⏎ ".join(ans_text[:2])[:150] if ans_text else "（无文本回答）"
    # 初筛：首 2 行是否含结论性关键词（机器初筛，终判归砺）
    joined = " ".join(ans_text[:2])
    is_direct = any(k in joined for k in direct_kw) and "hearth>" not in joined
    if is_direct:
        count_direct += 1
    mark = "初筛=直答" if is_direct else "初筛=待判"
    out.write(f"| {i+1} | {inp[:30]} | {first2} | {mark} |\n")
out.write(f"\n**机器初筛直答计数：{count_direct}/10**（预注册线 ≥8；**终判归砺逐题复核**——初筛关键词法有误判可能，仅作排序参考）\n")
out.close()
print(f"segments={len(segments)} direct_screen={count_direct}")
