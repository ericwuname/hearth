import paramiko, shlex, time, sys

HOST, USER, PW = '192.168.220.131', 'wutao', '123456'
H = '~/codex/target/release/hearth'
LOG = 'C:/tmp/hearth_telemetry_r2.txt'

def classify(rc, out, timeout_rc=124):
    o = out.lower()
    if rc == timeout_rc:
        return 'HANG'
    if rc != 0:
        if any(k in o for k in ['authentication_error', 'invalid', '401', '403', '5xx', 'connection', 'timeout', 'timed out', 'Could not', 'name or service']):
            return 'BACKEND_ERR'
        if any(k in o for k in ['panic', 'thread ', 'stack backtrace', 'segmentation', 'abort', 'double panic']):
            return 'CRASH'
        if any(k in o for k in ['task failed', '任务失败', '✗', 'error:', 'usage:']):
            return 'TASK_FAIL'
        return 'CRASH'
    # rc == 0
    if any(k in out for k in ['Task completed', '✓', '任务完成']):
        return 'SUCCESS'
    return 'TASK_FAIL'

def main():
    s = paramiko.SSHClient(); s.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    s.connect(HOST, username=USER, password=PW, timeout=20)
    def run(cmd, to=260):
        t0 = time.time()
        i, o, e = s.exec_command(cmd, timeout=to)
        out = o.read().decode(errors='replace'); err = e.read().decode(errors='replace')
        rc = o.channel.recv_exit_status()
        return rc, out, err, time.time() - t0
    def newest_sid():
        _, ls, _, _ = run("ls -t ~/.config/hearth/sessions/*.jsonl 2>/dev/null | head -1")
        p = ls.strip().split('/')[-1].replace('.jsonl', '')
        return p

    lines = []
    def log(x): 
        lines.append(x); print(x, flush=True)

    log('=== Hearth 可靠性遥测 R2 (resume + 长任务) | 通道=DeepSeek config | %s ===' % time.strftime('%Y-%m-%d %H:%M'))
    log('config: provider=deepseek (VM 实际配置); resume 走 config（不接受全局 flag）')
    log('')

    # ---------- Part A: resume 续跑 10 轮 ----------
    log('########## PART A: RESUME 续跑 ##########')
    tally_a = {}
    for i in range(1, 11):
        g1 = '用一句话回答：法国的首都是哪个城市？只输出城市名。'
        c1 = f'timeout 150 {H} chat --budget 20 {shlex.quote(g1)} 2>&1'
        rc1, out1, _, d1 = run(c1, to=170)
        cls1 = classify(rc1, out1)
        sid = newest_sid() if rc1 == 0 else '(none)'
        g2 = '接着上面，列举这个国家的 3 个著名城市，用逗号分隔。'
        if rc1 == 0 and sid != '(none)':
            c2 = f'timeout 150 {H} resume {sid} --budget 20 {shlex.quote(g2)} 2>&1'
            rc2, out2, _, d2 = run(c2, to=170)
            cls2 = classify(rc2, out2)
        else:
            rc2, out2, d2, cls2 = -1, '(skipped: chat failed)', 0.0, 'SKIP'
        tally_a[cls2] = tally_a.get(cls2, 0) + 1
        log(f'A{i:02d} chat={cls1}(rc={rc1},{d1:.1f}s) resume={cls2}(rc={rc2},{d2:.1f}s) sid={sid}')
        log(f'     chat_tail: {out1[-140:].strip()!r}')
        log(f'     res_tail : {out2[-160:].strip()!r}')
    log('PART A tally: ' + ', '.join(f'{k}={v}' for k, v in sorted(tally_a.items())))
    log('')

    # ---------- Part B: 长任务压测 10 轮 ----------
    log('########## PART B: 长任务压测 (高 step 预算, 推理密集型, 不依赖外部工具) ##########')
    tally_b = {}
    for j in range(1, 11):
        g = ('请详细设计一个 Rust 命令行工具「log-aggregator」，要求分 5 个模块描述：'
             '1) 配置加载 2) 日志解析 3) 过滤与聚合 4) 输出格式化 5) 主流程编排。'
             '每个模块给出完整函数签名、关键数据结构、核心实现思路，并说明模块间如何通信。'
             '最后给出 main 函数的完整执行流程。要求论述详尽、不少于 700 字。')
        c = f'timeout 200 {H} chat --budget 40 {shlex.quote(g)} 2>&1'
        rc, out, _, d = run(c, to=220)
        cls = classify(rc, out)
        tally_b[cls] = tally_b.get(cls, 0) + 1
        log(f'B{j:02d} {cls} rc={rc} {d:.1f}s')
        log(f'     tail: {out[-160:].strip()!r}')
    log('PART B tally: ' + ', '.join(f'{k}={v}' for k, v in sorted(tally_b.items())))
    log('')

    # ---------- 总账 ----------
    log('=== TALLY ===')
    log('PART A (resume): ' + ', '.join(f'{k}={v}' for k, v in sorted(tally_a.items())))
    log('PART B (longtask): ' + ', '.join(f'{k}={v}' for k, v in sorted(tally_b.items())))
    s.close()
    with open(LOG, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))
    print('WROTE', LOG)

if __name__ == '__main__':
    main()
