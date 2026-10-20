// ------------------------=
// FUNC: initializeInfinity
// DESC: Coordinates the motion score and usable page controls without a framework.
// ------------------=
(function initializeInfinity() {
  "use strict";
  const root = document.documentElement;
  root.classList.remove("no-js");
  root.classList.add("js");
  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
  const motionButton = document.querySelector(".motion-control");
  const hero = document.querySelector(".hero");
  const story = document.querySelector(".opening-story");
  const canvas = document.querySelector("#infinity-canvas");
  const macro = document.querySelector(".macro-image img");
  const chapters = [...document.querySelectorAll(".chapter-nav a")];
  let userPaused = false;
  try { userPaused = localStorage.getItem("infinity-motion") === "paused"; } catch {}
  let paused = userPaused || reduced.matches;
  let renderer = null, visible = true, raf = 0, width = 1, height = 1;
  let time = 0, lastTime = 0, progress = 0, scrollQueued = false;
  let pointer = { x: 0, y: 0 }, easedPointer = { x: 0, y: 0 };
  const image = new Image();

  // ------------------------=
  // FUNC: requestDraw
  // DESC: Schedules at most one artwork frame.
  // ------------------=
  function requestDraw() {
    if (renderer && !raf) raf = requestAnimationFrame(drawArtwork);
  }
  // ------------------------=
  // FUNC: updateMotion
  // DESC: Applies system-first motion preference to GPU effects and CSS sequences.
  // ------------------=
  function updateMotion() {
    paused = userPaused || reduced.matches;
    root.classList.toggle("motion-paused", paused);
    motionButton.setAttribute("aria-pressed", String(paused));
    motionButton.setAttribute("aria-label", reduced.matches ? "Reduced motion enabled by your device" : paused ? "Resume animation" : "Pause animation");
    motionButton.title = motionButton.getAttribute("aria-label");
    motionButton.querySelector("use").setAttribute("href", paused ? "#play" : "#pause");
    motionButton.disabled = reduced.matches;
    if (paused) time = Math.max(time, 2.2);
    requestDraw();
    updateScroll();
  }
  // ------------------------=
  // FUNC: toggleMotion
  // DESC: Persists the visitor's animation preference without overriding system reduction.
  // ------------------=
  function toggleMotion() {
    userPaused = !userPaused;
    try { localStorage.setItem("infinity-motion", userPaused ? "paused" : "playing"); } catch {}
    updateMotion();
  }
  motionButton.addEventListener("click", toggleMotion);
  reduced.addEventListener("change", updateMotion);

  // ------------------------=
  // FUNC: resizeArtwork
  // DESC: Measures the stage; the 3D renderer owns its bounded framebuffer allocation.
  // ------------------=
  function resizeArtwork() {
    const bounds = canvas.getBoundingClientRect();
    width = Math.max(1, bounds.width);
    height = Math.max(1, bounds.height);
    requestDraw();
  }
  // ------------------------=
  // FUNC: drawArtwork
  // DESC: Advances visible material lighting; paused or hidden scenes consume no continuous frames.
  // ------------------=
  function drawArtwork(timestamp) {
    raf = 0;
    if (!renderer) return;
    const delta = lastTime ? Math.min(timestamp - lastTime, 48) : 0;
    lastTime = timestamp;
    if (!paused && visible && !document.hidden) time += delta / 1000;
    if (!paused) {
      easedPointer.x += (pointer.x - easedPointer.x) * .07;
      easedPointer.y += (pointer.y - easedPointer.y) * .07;
    }
    const painted = renderer.draw({ width, height, time, paused, pointer: easedPointer, progress });
    if (!painted) { root.classList.remove("art-ready"); return; }
    root.classList.add("art-ready");
    canvas.dataset.frames = String(Number(canvas.dataset.frames || 0) + 1);
    if (!paused && visible && !document.hidden) raf = requestAnimationFrame(drawArtwork);
  }
  // ------------------------=
  // FUNC: artworkLoaded
  // DESC: Loads the self-hosted 3D module while retaining the original image on unsupported browsers.
  // ------------------=
  async function artworkLoaded() {
    try {
      const art = await import("./art.js");
      renderer?.dispose();
      renderer = await art.create(canvas, image);
      if (renderer) resizeArtwork();
    } catch { canvas.dataset.renderer = "static"; }
  }
  image.addEventListener("load", artworkLoaded);
  image.src = "assets/infinity-emblem.webp";
  new ResizeObserver(resizeArtwork).observe(canvas);
  // ------------------------=
  // FUNC: contextLost
  // DESC: Restores the original static emblem when the browser loses GPU resources.
  // ------------------=
  function contextLost(event) { event.preventDefault(); root.classList.remove("art-ready"); }
  canvas.addEventListener("webglcontextlost", contextLost);
  canvas.addEventListener("webglcontextrestored", artworkLoaded);
  // ------------------------=
  // FUNC: pointerMoved
  // DESC: Lets mouse movement influence the material-light sweep without moving the logo.
  // ------------------=
  function pointerMoved(event) {
    if (event.pointerType === "touch" || paused) return;
    const rect = hero.getBoundingClientRect();
    pointer.x = (event.clientX - rect.left) / rect.width - .5;
    pointer.y = (event.clientY - rect.top) / rect.height - .5;
  }
  // ------------------------=
  // FUNC: pointerLeft
  // DESC: Returns lighting to its centered state.
  // ------------------=
  function pointerLeft() { pointer = { x: 0, y: 0 }; }
  hero.addEventListener("pointermove", pointerMoved, { passive: true });
  hero.addEventListener("pointerleave", pointerLeft);
  // ------------------------=
  // FUNC: heroVisibility
  // DESC: Suspends the GPU scene outside the viewport.
  // ------------------=
  function heroVisibility(entries) { visible = entries[0].isIntersecting; lastTime = 0; if (visible) requestDraw(); }
  new IntersectionObserver(heroVisibility).observe(story);
  // ------------------------=
  // FUNC: pageVisibility
  // DESC: Avoids background rendering and elapsed-time jumps.
  // ------------------=
  function pageVisibility() { lastTime = 0; if (!document.hidden) requestDraw(); }
  document.addEventListener("visibilitychange", pageVisibility);

  // ------------------------=
  // FUNC: revealSections
  // DESC: Reveals editorial content once without removing it from the accessible document.
  // ------------------=
  function revealSections(entries, observer) {
    for (const entry of entries) if (entry.isIntersecting) { entry.target.classList.add("visible"); observer.unobserve(entry.target); }
  }
  const reveals = new IntersectionObserver(revealSections, { threshold: .12 });
  for (const element of document.querySelectorAll(".reveal")) reveals.observe(element);
  // ------------------------=
  // FUNC: updateScroll
  // DESC: Coordinates restrained artwork approach, macro motion and chapter state with native scrolling.
  // ------------------=
  function updateScroll() {
    scrollQueued = false;
    const rect = story.getBoundingClientRect();
    progress = Math.max(0, Math.min(1, -rect.top / Math.max(1, rect.height - window.innerHeight)));
    root.style.setProperty("--story-progress", progress);
    const macroRect = macro.getBoundingClientRect();
    const drift = Math.max(-45, Math.min(45, (window.innerHeight * .5 - macroRect.top) * .08));
    macro.style.transform = paused ? "scale(1.08)" : `translateY(${drift}px) scale(1.16)`;
    let current = chapters[0];
    for (const link of chapters) if (document.querySelector(link.hash).getBoundingClientRect().top < window.innerHeight * .5) current = link;
    for (const link of chapters) {
      if (link === current) link.setAttribute("aria-current", "location");
      else link.removeAttribute("aria-current");
    }
    if (visible) requestDraw();
  }
  // ------------------------=
  // FUNC: queueScroll
  // DESC: Coalesces high-frequency scroll events to one layout pass per frame.
  // ------------------=
  function queueScroll() { if (!scrollQueued) { scrollQueued = true; requestAnimationFrame(updateScroll); } }
  window.addEventListener("scroll", queueScroll, { passive: true });
  window.addEventListener("resize", queueScroll, { passive: true });

  const menu = document.querySelector(".menu-control");
  const nav = document.querySelector("#main-nav");
  // ------------------------=
  // FUNC: setMenu
  // DESC: Keeps mobile navigation and accessible state synchronized.
  // ------------------=
  function setMenu(open) {
    nav.classList.toggle("open", open);
    menu.setAttribute("aria-expanded", String(open));
    menu.setAttribute("aria-label", open ? "Close navigation" : "Open navigation");
    menu.title = menu.getAttribute("aria-label");
    menu.querySelector("use").setAttribute("href", open ? "#close" : "#menu");
  }
  // ------------------------=
  // FUNC: toggleMenu
  // DESC: Toggles the mobile navigation panel.
  // ------------------=
  function toggleMenu() { setMenu(menu.getAttribute("aria-expanded") !== "true"); }
  // ------------------------=
  // FUNC: menuNavigation
  // DESC: Closes mobile navigation after selecting an anchor.
  // ------------------=
  function menuNavigation(event) { if (event.target.closest("a")) setMenu(false); }
  // ------------------------=
  // FUNC: dismissMenu
  // DESC: Closes navigation on Escape and returns keyboard focus.
  // ------------------=
  function dismissMenu(event) { if (event.key === "Escape" && nav.classList.contains("open")) { setMenu(false); menu.focus(); } }
  menu.addEventListener("click", toggleMenu);
  nav.addEventListener("click", menuNavigation);
  document.addEventListener("keydown", dismissMenu);

  const tabs = [...document.querySelectorAll(".feature-tab")];
  const panel = document.querySelector("#feature-panel");
  const modes = {
    voice: { app: "Infinity workspace", prompt: "“Infinity, open my text editor.”", response: "A little less between you and your next idea.", status: "VOICE / A NATURAL BEGINNING" },
    create: { app: "Text Editor", prompt: "“Help me find the first line.”", response: "A spark becomes a draft. The next move is yours.", status: "CREATE / SOMETHING STARTS HERE" },
    explore: { app: "Infinity Browser", prompt: "“Let’s explore something new.”", response: "Keep the interesting things close. Follow the next connection.", status: "EXPLORE / STAY CURIOUS" }
  };
  // ------------------------=
  // FUNC: selectFeature
  // DESC: Activates a distinct product-study scene and replays its short illustrative sequence.
  // ------------------=
  function selectFeature(tab, focus = false) {
    const mode = tab.dataset.feature;
    const data = modes[mode];
    if (!data) return;
    for (const item of tabs) {
      const selected = item === tab;
      item.classList.toggle("active", selected);
      item.setAttribute("aria-selected", String(selected));
      item.tabIndex = selected ? 0 : -1;
    }
    panel.setAttribute("aria-labelledby", tab.id);
    panel.dataset.mode = mode;
    for (const scene of document.querySelectorAll("[data-scene]")) scene.hidden = scene.dataset.scene !== mode;
    for (const key of ["app", "prompt", "response", "status"]) document.getElementById(`preview-${key}`).textContent = data[key];
    panel.classList.remove("replaying");
    void panel.offsetWidth;
    panel.classList.add("replaying");
    if (focus) tab.focus();
  }
  // ------------------------=
  // FUNC: featureClicked
  // DESC: Selects the clicked product-study mode.
  // ------------------=
  function featureClicked(event) { selectFeature(event.currentTarget); }
  // ------------------------=
  // FUNC: tabKeydown
  // DESC: Provides arrow and Home/End navigation across feature modes.
  // ------------------=
  function tabKeydown(event) {
    let index = tabs.indexOf(event.currentTarget);
    if (["ArrowDown", "ArrowRight"].includes(event.key)) index = (index + 1) % tabs.length;
    else if (["ArrowUp", "ArrowLeft"].includes(event.key)) index = (index - 1 + tabs.length) % tabs.length;
    else if (event.key === "Home") index = 0;
    else if (event.key === "End") index = tabs.length - 1;
    else return;
    event.preventDefault();
    selectFeature(tabs[index], true);
  }
  for (const tab of tabs) { tab.addEventListener("click", featureClicked); tab.addEventListener("keydown", tabKeydown); }
  for (const [index, bar] of [...document.querySelectorAll(".voice-wave i")].entries()) bar.style.setProperty("--i", index);
  updateMotion();
})();
