// Self-contained: http server + playwright gameplay test
const http = require('http');
const fs = require('fs');
const path = require('path');
const { chromium } = require('playwright');

const ROOT = __dirname;
const MIME = { '.html': 'text/html', '.png': 'image/png', '.js': 'text/javascript' };
const srv = http.createServer((req, res) => {
  const f = path.join(ROOT, decodeURIComponent(req.url.split('?')[0]));
  fs.readFile(f, (e, data) => {
    if (e) { res.writeHead(404); res.end('nf'); return; }
    res.writeHead(200, { 'Content-Type': MIME[path.extname(f)] || 'application/octet-stream' });
    res.end(data);
  });
});

srv.listen(8125, '127.0.0.1', async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1000, height: 720 } });
  await page.goto('http://127.0.0.1:8125/contra-index.html');
  await page.waitForTimeout(800);
  await page.screenshot({ path: path.join(ROOT, 'slim-t1-start.png') });

  await page.keyboard.press('Enter');
  await page.waitForTimeout(1500);
  await page.screenshot({ path: path.join(ROOT, 'slim-t2-battle-start.png') });

  await page.keyboard.down('ArrowRight');
  for (let i = 0; i < 5; i++) { await page.keyboard.press('KeyJ'); await page.waitForTimeout(180); }
  await page.keyboard.up('ArrowRight');
  await page.screenshot({ path: path.join(ROOT, 'slim-t3-move-shoot.png') });

  await page.keyboard.press('Space');
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(ROOT, 'slim-t4-jump.png') });
  await page.waitForTimeout(600);

  for (let i = 0; i < 6; i++) { await page.keyboard.press('KeyJ'); await page.waitForTimeout(200); }
  await page.screenshot({ path: path.join(ROOT, 'slim-t5-later.png') });

  const hud = await page.evaluate(() => document.getElementById('levelDisplay')?.textContent || 'no-hud');
  console.log('HUD:', hud);
  await browser.close();
  srv.close();
  console.log('TEST-COMPLETE');
});
