const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const http = require('node:http');
const path = require('node:path');
const { chromium } = require('playwright');
const sharp = require('sharp');

const root = path.resolve(__dirname, '..');
const output = path.join(root, 'verification', 'experience');
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
// FUNC: verifyExperience
// DESC: Exercises real rendering, screenshot links, navigation and motion on desktop/mobile.
// ------------------=
async function verifyExperience() {
  await fs.mkdir(output, { recursive: true });
  const server = process.env.EXPERIENCE_URL ? null : http.createServer(serveFile);
  if (server) await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(4178, '127.0.0.1', resolve);
  });
  const base = process.env.EXPERIENCE_URL || 'http://127.0.0.1:4178/';
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
      const tabs = page.locator(".feature-tab");
      const panel = page.locator("#feature-panel");
      assert.equal(await tabs.count(), 6, "Original features and three screenshot choices remain");
      if (width > 960) {
        const selectorBounds = await page.locator(".feature-selector").boundingBox();
        const panelBounds = await panel.boundingBox();
        assert.ok(selectorBounds.x + selectorBounds.width < panelBounds.x, "Selector remains the left-hand column");
      }
      for (const mode of ["voice", "create", "explore"]) {
        await page.locator(`[data-feature="${mode}"]`).click();
        assert.equal(await panel.getAttribute("data-mode"), mode);
        assert.ok(await page.locator(`[data-scene="${mode}"]`).isVisible());
        assert.ok(await page.locator(".product-study").isVisible());
        assert.equal(await page.locator(".screenshot-scene:visible").count(), 0);
      }
      let imageHeight = null;
      for (const [index, mode] of ["sign-in", "desktop", "app-library"].entries()) {
        const tab = page.locator(`[data-feature="${mode}"]`);
        await tab.click();
        const scene = page.locator(`[data-screenshot-scene="${mode}"]`);
        assert.equal(await tab.getAttribute("aria-selected"), "true");
        assert.equal(await panel.getAttribute("aria-labelledby"), await tab.getAttribute("id"));
        assert.ok(await scene.isVisible());
        assert.ok(!(await page.locator(".product-study").isVisible()));
        assert.equal(await page.locator(".screenshot-scene:visible").count(), 1);
        const img = scene.locator("img");
        await img.evaluate(image => image.decode());
        const geometry = await img.evaluate(image => ({ w: image.clientWidth, h: image.clientHeight, nw: image.naturalWidth, nh: image.naturalHeight, fit: getComputedStyle(image).objectFit }));
        assert.ok(geometry.nw > 0 && geometry.nh > 0);
        assert.equal(geometry.fit, "contain", "Captures are never cropped");
        if (imageHeight !== null) assert.equal(geometry.h, imageHeight, "Image area stays stable between screenshot selections");
        imageHeight = geometry.h;
        const imageBounds = await scene.locator(".screenshot-image").boundingBox();
        const detailsBounds = await scene.locator(".screenshot-details").boundingBox();
        assert.ok(detailsBounds.y >= imageBounds.y + imageBounds.height - 1, "Details must not overlap the capture");
        const popupPromise = page.waitForEvent("popup");
        await scene.locator(".text-link").click();
        const popup = await popupPromise;
        await popup.waitForLoadState();
        const response = await context.request.get(popup.url());
        assert.equal(response.status(), 200);
        const metadata = await sharp(await response.body()).metadata();
        assert.equal(metadata.width, 2542);
        assert.equal(metadata.height, index === 2 ? 1436 : 1444);
        await popup.close();
        if ([1440, 390].includes(width)) {
          await page.locator("#experience").evaluate(element => element.scrollIntoView({ behavior: "instant", block: "start" }));
          await page.screenshot({ path: path.join(output, `${width}-${mode}.png`) });
        }
      }
      await tabs.first().focus();
      await page.keyboard.press("End");
      assert.equal(await tabs.last().getAttribute("aria-selected"), "true");
      assert.ok(await tabs.last().evaluate(element => element === document.activeElement));
      await page.keyboard.press("Home");
      assert.equal(await tabs.first().getAttribute("aria-selected"), "true");
      assert.ok(await page.locator('[data-scene="voice"]').isVisible(), "Returning to an original feature restores its preview");
      await page.keyboard.press("ArrowDown");
      assert.equal(await tabs.nth(1).getAttribute("aria-selected"), "true");
      await page.keyboard.press("ArrowUp");
      assert.equal(await tabs.first().getAttribute("aria-selected"), "true");
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), "No horizontal overflow");
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
    await page.locator('#closer').evaluate(element => element.scrollIntoView({ behavior: 'instant' }));
    await page.waitForTimeout(500);
    const frames = await canvas.getAttribute('data-frames');
    await page.waitForTimeout(300);
    assert.equal(await canvas.getAttribute('data-frames'), frames, 'Offscreen opening animation stops');
    await context.close();
    const noScript = await browser.newContext({ javaScriptEnabled: false, viewport: { width: 390, height: 844 } });
    const staticPage = await noScript.newPage();
    await staticPage.goto(new URL('#experience', base).href);
    const fallbackLinks = staticPage.locator(".screenshot-fallback a");
    assert.equal(await fallbackLinks.count(), 3);
    for (const link of await fallbackLinks.all()) {
      assert.ok(await link.isVisible());
      const response = await noScript.request.get(new URL(await link.getAttribute("href"), base).href);
      assert.equal(response.status(), 200);
      assert.equal((await sharp(await response.body()).metadata()).width, 2542);
    }
    await noScript.close();
    await fs.writeFile(path.join(output, 'results.json'), JSON.stringify({ base, results, animatedCanvas: true, nonblankCanvas: true, pause: true, noScript: true }, null, 2));
    console.log(JSON.stringify({ base, results, animatedCanvas: true, pause: true, noScript: true }));
  } finally {
    if (browser) await browser.close();
    if (server) await new Promise(resolve => server.close(resolve));
  }
}

verifyExperience().catch(error => { console.error(error); process.exitCode = 1; });
