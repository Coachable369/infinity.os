const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const http = require('node:http');
const path = require('node:path');
const { chromium } = require('playwright');
const sharp = require('sharp');

const root = path.resolve(__dirname, '..');
const output = path.join(root, 'verification', 'gallery');
const mime = { '.html': 'text/html', '.css': 'text/css', '.js': 'text/javascript', '.mjs': 'text/javascript', '.json': 'application/json', '.webp': 'image/webp', '.ttf': 'font/ttf' };

// ------------------------=
// FUNC: serveFile
// DESC: Serves only website files for the bounded local browser check.
// ------------------=
async function serveFile(request, response) {
  try {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    const file = path.resolve(root, '.' + (pathname === '/' ? '/index.html' : pathname));
    if (!file.startsWith(root + path.sep)) { response.writeHead(403).end(); return; }
    const body = await fs.readFile(file);
    response.writeHead(200, { 'Content-Type': mime[path.extname(file)] || 'application/octet-stream' }).end(body);
  } catch { response.writeHead(404).end(); }
}

// ------------------------=
// FUNC: verifyGallery
// DESC: Exercises real rendering, screenshot links, navigation and motion on desktop/mobile.
// ------------------=
async function verifyGallery() {
  await fs.mkdir(output, { recursive: true });
  const server = process.env.GALLERY_URL ? null : http.createServer(serveFile);
  if (server) await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(4178, '127.0.0.1', resolve);
  });
  const base = process.env.GALLERY_URL || 'http://127.0.0.1:4178/';
  let browser;
  const results = [];
  try {
    browser = await chromium.launch({ headless: true, args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
    for (const width of [1440, 390, 320, 768, 1920]) {
      const context = await browser.newContext({ viewport: { width, height: 1000 }, reducedMotion: 'reduce' });
      const page = await context.newPage();
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      await page.goto(new URL('#experience', base).href);
      await page.locator('#experience-title').waitFor({ state: 'visible' });
      await page.evaluate(() => document.fonts.ready);
      assert.equal(await page.locator('.screenshot-entry').count(), 3);
      const entries = page.locator('.screenshot-entry');
      for (let index = 0; index < 3; index++) {
        const entry = entries.nth(index);
        await entry.scrollIntoViewIfNeeded();
        const img = entry.locator('img');
        await img.evaluate(image => image.decode());
        const geometry = await img.evaluate(image => ({ w: image.clientWidth, h: image.clientHeight, nw: image.naturalWidth, nh: image.naturalHeight, source: image.currentSrc }));
        assert.ok(geometry.nw > 0 && geometry.nh > 0, 'Responsive image must decode');
        assert.ok(Math.abs(geometry.w / geometry.h - geometry.nw / geometry.nh) < .01, 'Screenshot must remain uncropped');
        const heading = await entry.locator('.screenshot-heading').boundingBox();
        const figure = await entry.locator('figure').boundingBox();
        assert.ok(heading.y + heading.height <= figure.y, 'Copy must not overlap the image');
        if ([1440, 390].includes(width)) await entry.screenshot({ path: path.join(output, `${width}-${index + 1}.png`) });
      }
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'No horizontal overflow');
      await page.locator('.screenshot-index a').nth(1).click();
      await page.waitForURL('**/#action-desktop');
      assert.ok((await entries.nth(1).boundingBox()).y >= 68, 'Anchor must clear the header');
      const popupPromise = page.waitForEvent('popup');
      await entries.nth(1).locator('figcaption a').click();
      const popup = await popupPromise;
      await popup.waitForLoadState();
      const response = await context.request.get(popup.url());
      assert.equal(response.status(), 200);
      const metadata = await sharp(await response.body()).metadata();
      assert.equal(metadata.width, 2542);
      assert.equal(metadata.height, 1444);
      await popup.close();
      if (width === 390) {
        await page.locator('.menu-control').click();
        assert.equal(await page.locator('.menu-control').getAttribute('aria-expanded'), 'true');
        await page.keyboard.press('Escape');
        assert.equal(await page.locator('.menu-control').getAttribute('aria-expanded'), 'false');
      }
      await page.locator('#experience').evaluate(element => element.scrollIntoView({ block: 'start', behavior: 'instant' }));
      if ([1440, 390].includes(width)) await page.screenshot({ path: path.join(output, `${width}-intro.png`) });
      assert.deepEqual(errors, []);
      results.push({ width, screenshots: 3, aspectRatios: true, fullResolutionLink: true, overflow: false, errors });
      await context.close();
    }
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const page = await context.newPage();
    await page.goto(base);
    await page.waitForFunction(() => document.documentElement.classList.contains('art-ready'));
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(1300);
    const canvas = page.locator('#infinity-canvas');
    const first = await canvas.screenshot();
    await page.waitForTimeout(400);
    const second = await canvas.screenshot();
    assert.ok(!first.equals(second), 'Canvas must animate');
    const stats = await sharp(second).stats();
    assert.ok(stats.channels.some(channel => channel.stdev > 5), 'Canvas must not be blank');
    await page.locator('.motion-control').click();
    await page.waitForTimeout(500);
    assert.equal(await page.locator('.motion-control').getAttribute('aria-pressed'), 'true');
    const paused = await canvas.screenshot();
    await page.waitForTimeout(250);
    assert.ok(paused.equals(await canvas.screenshot()), 'Pause must freeze the canvas');
    await page.screenshot({ path: path.join(output, 'hero.png') });
    await page.locator('.motion-control').click();
    await page.locator('#action-app-library').evaluate(element => element.scrollIntoView({ behavior: 'instant' }));
    await page.waitForTimeout(500);
    const frames = await canvas.getAttribute('data-frames');
    await page.waitForTimeout(300);
    assert.equal(await canvas.getAttribute('data-frames'), frames, 'Gallery must not keep offscreen 3D animation running');
    await context.close();
    const noScript = await browser.newContext({ javaScriptEnabled: false, viewport: { width: 390, height: 844 } });
    const staticPage = await noScript.newPage();
    await staticPage.goto(new URL('#experience', base).href);
    for (const image of await staticPage.locator('.screenshot-image img').all()) {
      await image.evaluate(element => element.scrollIntoView({ block: 'center', behavior: 'instant' }));
      await image.evaluate(element => element.decode());
      assert.ok(await image.isVisible());
    }
    await noScript.close();
    await fs.writeFile(path.join(output, 'results.json'), JSON.stringify({ base, results, animatedCanvas: true, nonblankCanvas: true, pause: true, noScript: true }, null, 2));
    console.log(JSON.stringify({ base, results, animatedCanvas: true, pause: true, noScript: true }));
  } finally {
    if (browser) await browser.close();
    if (server) await new Promise(resolve => server.close(resolve));
  }
}

verifyGallery().catch(error => { console.error(error); process.exitCode = 1; });
