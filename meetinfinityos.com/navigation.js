// ------------------------=
// FUNC: initializeNavigation
// DESC: Shares accessible mobile navigation across the homepage and reading pages.
// ------------------=
(function initializeNavigation() {
  "use strict";
  const root = document.documentElement;
  const menu = document.querySelector(".menu-control");
  const nav = document.querySelector("#main-nav");
  if (!menu || !nav) return;
  root.classList.remove("no-js");
  root.classList.add("js");

  // ------------------------=
  // FUNC: setMenu
  // DESC: Keeps the menu visibility, icon and accessible state synchronized.
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
  // DESC: Closes navigation when following an anchor or changing pages.
  // ------------------=
  function menuNavigation(event) { if (event.target.closest("a")) setMenu(false); }
  // ------------------------=
  // FUNC: dismissMenu
  // DESC: Closes navigation on Escape and returns keyboard focus to its trigger.
  // ------------------=
  function dismissMenu(event) {
    if (event.key === "Escape" && nav.classList.contains("open")) {
      setMenu(false);
      menu.focus();
    }
  }
  menu.addEventListener("click", toggleMenu);
  nav.addEventListener("click", menuNavigation);
  document.addEventListener("keydown", dismissMenu);
})();
