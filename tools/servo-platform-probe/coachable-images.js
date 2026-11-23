// Runs in the real page: expose each gallery through its own controls, then
// scroll lazy images into view and verify decoded dimensions without rewriting DOM.
(() => {
    if (document.readyState === "loading") return false;
    if (!window.infinityFontReport) {
        window.infinityFontReport = true;
        console.log(JSON.stringify({fonts:[...document.fonts].map(font=>[font.family,font.weight,font.status])}));
    }
    const images = [...document.images].filter(image => image.src);
    const pending = images.find(image => !image.complete || image.naturalWidth === 0);
    if (!pending) {
        if (window.scrollY !== 0) { window.scrollTo(0, 0); return false; }
        return images.length > 0 && document.readyState === "complete";
    }
    const ancestors = [];
    for (let element = pending.parentElement; element; element = element.parentElement)
        ancestors.push(element);
    for (const element of ancestors.reverse()) {
        if (!element.hidden) continue;
        const label = element.getAttribute("aria-labelledby");
        const kind = element.getAttribute("data-experience-visual");
        const control = label ? document.getElementById(label) :
            kind ? document.querySelector('[data-experience-tab="' + kind + '"]') : null;
        if (control) { control.click(); return false; }
    }
    pending.scrollIntoView({block: "center"});
    return false;
})()
