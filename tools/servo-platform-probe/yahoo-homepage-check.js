// Opt-in live-site acceptance at the desktop viewport; never executes in the product.
// ------------------------=
// FUNC: yahooDesktopReady
// DESC: Checks the active slide and on-screen rail artwork, column boundaries and document completion.
// ------------------=
(function yahooDesktopReady() {
    if (document.readyState !== "complete") return false;
    const rail = document.getElementById("stacked-list");
    const trending = document.getElementById("trending-search");
    if (!rail || !trending) return false;
    const column = rail.getBoundingClientRect(), cards = trending.getBoundingClientRect();
    if (!column.width || cards.left < column.left - 1 || cards.right > column.right + 1) return false;
    const visible = [...trending.querySelectorAll("img")].filter(image => {
        const r = image.getBoundingClientRect();
        return r.width > 0 && r.bottom > 0 && r.top < innerHeight;
    });
    const decoded = image => image.complete && image.naturalWidth > 0;
    if (!visible.length || !visible.every(decoded)) return false;
    const shown = [...document.querySelectorAll('article[aria-hidden="false"] img')].some(image => {
        const r = image.getBoundingClientRect();
        return decoded(image) && image.naturalWidth >= 300 && r.width >= 300 &&
            r.left >= column.right && r.right <= innerWidth && r.top < innerHeight && r.bottom > 0 &&
            document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2)?.closest("article") === image.closest("article");
    });
    if (shown && !window.__infinityEvidencePaused) {
        const pause = document.querySelector('button[aria-label="Pause"]');
        if (pause) { pause.click(); window.__infinityEvidencePaused = true; return false; }
    }
    return shown;
})()
