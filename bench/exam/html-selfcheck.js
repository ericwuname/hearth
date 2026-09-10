// S12（手术包二）：通用 HTML/游戏类无头自检——零 JS 错误 + 核心交互冒烟
// （按键/点击各一次）+ 截图留档。复用 bench/exam 既有 playwright 模式
// （http server + chromium），不新引重型依赖。
//
// 用法：node html-selfcheck.js <file.html>
// 输出：末行 JSON {"ok":bool,"errors":[...],"detail":"...","screenshot":"..."}
// 退出码：0=通过 2=有 JS 错误 3=自检异常（调用方按 JSON 末行判定）
const http = require('http');
const fs = require('fs');
const path = require('path');
const { chromium } = require('playwright');

const target = process.argv[2];
if (!target || !fs.existsSync(target)) {
  console.log(JSON.stringify({ ok: false, errors: [], detail: `目标不存在: ${target}` }));
  process.exit(3);
}
const ROOT = path.dirname(path.resolve(target));
const FILE = path.basename(target);
const MIME = {
  '.html': 'text/html',
  '.htm': 'text/html',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.json': 'application/json',
  '.svg': 'image/svg+xml',
};

const srv = http.createServer((req, res) => {
  const f = path.join(ROOT, decodeURIComponent(req.url.split('?')[0]));
  fs.readFile(f, (e, data) => {
    if (e) {
      res.writeHead(404);
      res.end('nf');
      return;
    }
    res.writeHead(200, {
      'Content-Type': MIME[path.extname(f)] || 'application/octet-stream',
    });
    res.end(data);
  });
});

(async () => {
  const errors = [];
  let browser;
  try {
    await new Promise((r) => srv.listen(0, '127.0.0.1', r));
    const port = srv.address().port;
    browser = await chromium.launch();
    const page = await browser.newPage({ viewport: { width: 1000, height: 720 } });
    page.on('pageerror', (e) => errors.push('pageerror: ' + e.message));
    page.on('console', (m) => {
      if (m.type() === 'error') errors.push('console: ' + m.text());
    });
    page.on('requestfailed', (r) => errors.push('requestfailed: ' + r.url()));
    await page.goto(`http://127.0.0.1:${port}/${FILE}`, {
      waitUntil: 'load',
      timeout: 30000,
    });
    await page.waitForTimeout(1200);
    // 核心交互冒烟：按键 + 点击各一次（页面无监听也不炸）
    try {
      await page.keyboard.press('Enter');
    } catch (_) {}
    await page.waitForTimeout(300);
    try {
      await page.mouse.click(500, 360);
    } catch (_) {}
    await page.waitForTimeout(600);
    const shot = path.join(
      process.cwd(),
      '.hearth',
      'selfcheck',
      FILE.replace(/[^A-Za-z0-9._-]+/g, '_') + '-selfcheck.png'
    );
    fs.mkdirSync(path.dirname(shot), { recursive: true });
    await page.screenshot({ path: shot });
    const ok = errors.length === 0;
    console.log(
      JSON.stringify({
        ok,
        errors,
        detail: ok
          ? `零 JS 错误（截图 ${shot}）`
          : `JS 错误 ${errors.length} 条: ${errors.slice(0, 3).join(' | ')}`,
        screenshot: shot,
      })
    );
    process.exitCode = ok ? 0 : 2;
  } catch (e) {
    console.log(
      JSON.stringify({
        ok: false,
        errors,
        detail: '浏览器自检异常: ' + (e && e.message ? e.message : String(e)),
      })
    );
    process.exitCode = 3;
  } finally {
    try {
      if (browser) await browser.close();
    } catch (_) {}
    try {
      srv.close();
    } catch (_) {}
  }
})();
