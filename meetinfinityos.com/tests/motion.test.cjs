const assert = require("node:assert/strict");
const { test } = require("node:test");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { runInNewContext } = require("node:vm");
const application = readFileSync(join(__dirname, "../app.js"), "utf8");
const navigation = readFileSync(join(__dirname, "../navigation.js"), "utf8");

// ------------------------=
// FUNC: element
// DESC: Supplies observable DOM state for production motion-controller regression tests.
// ------------------=
function element() {
  const classes = new Set();
  const attributes = new Map();
  const listeners = new Map();
  const children = new Map();
  return {
    attributes, listeners, style: {
      // ------------------------=
      // FUNC: setProperty
      // DESC: Accepts the chapter progress custom property without a layout engine.
      // ------------------=
      setProperty() {}
    }, dataset: {}, disabled: false, scrollHeight: 900,
    classList: {
      // ------------------------=
      // FUNC: add
      // DESC: Records an applied class.
      // ------------------=
      add(value) { classes.add(value); },
      // ------------------------=
      // FUNC: remove
      // DESC: Removes an applied class.
      // ------------------=
      remove(value) { classes.delete(value); },
      // ------------------------=
      // FUNC: contains
      // DESC: Reads the observable class state.
      // ------------------=
      contains(value) { return classes.has(value); },
      // ------------------------=
      // FUNC: toggle
      // DESC: Applies the explicit state supplied by production code.
      // ------------------=
      toggle(value, enabled) { if (enabled) classes.add(value); else classes.delete(value); }
    },
    // ------------------------=
    // FUNC: setAttribute
    // DESC: Records accessible state changes.
    // ------------------=
    setAttribute(key, value) { attributes.set(key, value); },
    // ------------------------=
    // FUNC: getAttribute
    // DESC: Returns current accessible state.
    // ------------------=
    getAttribute(key) { return attributes.get(key) ?? null; },
    // ------------------------=
    // FUNC: addEventListener
    // DESC: Connects real production handlers to the fixture.
    // ------------------=
    addEventListener(key, handler) { listeners.set(key, handler); },
    // ------------------------=
    // FUNC: focus
    // DESC: Observes keyboard focus restoration after menu dismissal.
    // ------------------=
    focus() { this.focused = true; },
    // ------------------------=
    // FUNC: querySelector
    // DESC: Supplies a stable descendant for the motion icon.
    // ------------------=
    querySelector(key) { if (!children.has(key)) children.set(key, element()); return children.get(key); },
    // ------------------------=
    // FUNC: getContext
    // DESC: Exercises the supported no-canvas fallback without rendering mocks.
    // ------------------=
    getContext() { return null; },
    // ------------------------=
    // FUNC: getBoundingClientRect
    // DESC: Supplies stable layout bounds for the production scroll coordinator.
    // ------------------=
    getBoundingClientRect() { return { top: 0, left: 0, width: 1440, height: 900 }; }
  };
}

// ------------------------=
// FUNC: fixture
// DESC: Runs the unchanged production application against controllable media and storage interfaces.
// ------------------=
function fixture({ reduced = false, saved = null, denied = false, navigationOnly = false } = {}) {
  const document = element();
  document.documentElement = element();
  document.documentElement.classList.add("no-js");
  // ------------------------=
  // FUNC: querySelectorAll
  // DESC: Leaves unrelated feature collections empty in this focused motion fixture.
  // ------------------=
  document.querySelectorAll = function querySelectorAll() { return []; };
  const media = element();
  media.matches = reduced;
  const storage = {
    // ------------------------=
    // FUNC: getItem
    // DESC: Models both saved preferences and restricted browser storage.
    // ------------------=
    getItem() { if (denied) throw new Error("Storage denied"); return saved; },
    // ------------------------=
    // FUNC: setItem
    // DESC: Observes persistence without requiring a browser profile.
    // ------------------=
    setItem(key, value) { if (denied) throw new Error("Storage denied"); saved = value; }
  };
  // ------------------------=
  // FUNC: Observer
  // DESC: Keeps unrelated visibility observation inert in the controller fixture.
  // ------------------=
  function Observer() {}
  // ------------------------=
  // FUNC: observe
  // DESC: Accepts observer registration without synthetic layout events.
  // ------------------=
  Observer.prototype.observe = function observe() {};
  const window = element();
  window.innerHeight = 900;
  window.scrollY = 0;
  // ------------------------=
  // FUNC: matchMedia
  // DESC: Exposes a controllable operating-system reduced-motion preference.
  // ------------------=
  window.matchMedia = function matchMedia() { return media; };
  runInNewContext(navigation, { document });
  if (!navigationOnly) runInNewContext(application, { document, window, localStorage: storage, Image: element,
    ResizeObserver: Observer, IntersectionObserver: Observer });
  return { document, media, button: document.querySelector(".motion-control"), storage };
}

// ------------------------=
// FUNC: reducedMotionAtStartup
// DESC: Verifies system reduction disables motion and cannot be overridden by saved play preference.
// ------------------=
function reducedMotionAtStartup() {
  const value = fixture({ reduced: true, saved: "playing" });
  assert.equal(value.button.disabled, true);
  assert.equal(value.button.getAttribute("aria-pressed"), "true");
  assert.equal(value.document.documentElement.classList.contains("motion-paused"), true);
}
test("system reduced motion takes precedence at startup", reducedMotionAtStartup);

// ------------------------=
// FUNC: preferenceTransitions
// DESC: Verifies user pause survives system preference changes and explicit resume persists.
// ------------------=
function preferenceTransitions() {
  const value = fixture();
  value.button.listeners.get("click")();
  assert.equal(value.storage.getItem(), "paused");
  value.media.matches = true;
  value.media.listeners.get("change")();
  assert.equal(value.button.disabled, true);
  value.media.matches = false;
  value.media.listeners.get("change")();
  assert.equal(value.button.disabled, false);
  assert.equal(value.button.getAttribute("aria-pressed"), "true");
  value.button.listeners.get("click")();
  assert.equal(value.storage.getItem(), "playing");
  assert.equal(value.button.getAttribute("aria-pressed"), "false");
}
test("user pause survives media preference changes", preferenceTransitions);

// ------------------------=
// FUNC: restrictedStorage
// DESC: Verifies blocked storage does not break navigation initialization or the motion control.
// ------------------=
function restrictedStorage() {
  const value = fixture({ denied: true });
  assert.equal(value.document.documentElement.classList.contains("js"), true);
  assert.equal(value.document.documentElement.classList.contains("no-js"), false);
  value.button.listeners.get("click")();
  assert.equal(value.button.getAttribute("aria-pressed"), "true");
}
test("restricted storage preserves working controls", restrictedStorage);

// ------------------------=
// FUNC: menuKeyboardTransitions
// DESC: Verifies shared navigation opens, dismisses and restores focus without a WebGL application.
// ------------------=
function menuKeyboardTransitions() {
  for (const navigationOnly of [true, false]) {
    const { document } = fixture({ navigationOnly });
    const menu = document.querySelector(".menu-control");
    const nav = document.querySelector("#main-nav");
    menu.listeners.get("click")();
    assert.equal(nav.classList.contains("open"), true);
    assert.equal(menu.getAttribute("aria-expanded"), "true");
    document.listeners.get("keydown")({ key: "ArrowDown" });
    assert.equal(nav.classList.contains("open"), true);
    document.listeners.get("keydown")({ key: "Escape" });
    assert.equal(nav.classList.contains("open"), false);
    assert.equal(menu.getAttribute("aria-expanded"), "false");
    assert.equal(menu.focused, true);
  }
}
test("navigation works independently and alongside the homepage with Escape focus restoration", menuKeyboardTransitions);

// ------------------------=
// FUNC: menuLinkTransitions
// DESC: Verifies navigation closes on a selected destination but not on empty menu space.
// ------------------=
function menuLinkTransitions() {
  const { document } = fixture({ navigationOnly: true });
  const menu = document.querySelector(".menu-control");
  const nav = document.querySelector("#main-nav");
  menu.listeners.get("click")();
  // ------------------------=
  // FUNC: closest
  // DESC: Models a click that does not belong to a navigation link.
  // ------------------=
  nav.listeners.get("click")({ target: { closest() { return null; } } });
  assert.equal(nav.classList.contains("open"), true);
  // ------------------------=
  // FUNC: closest
  // DESC: Models clicking a descendant of a destination link.
  // ------------------=
  nav.listeners.get("click")({ target: { closest() { return {}; } } });
  assert.equal(nav.classList.contains("open"), false);
  assert.equal(menu.getAttribute("aria-expanded"), "false");
}
test("navigation closes only when a destination is selected", menuLinkTransitions);
