// ------------------------=
// FUNC: findRenderedText
// DESC: Selects a bounded visible DOM match and returns packed position/count without modifying page markup.
// ------------------=
((query, requested) => {
  const selection = window.getSelection();
  selection.removeAllRanges();
  if (!query || !document.body) return 0;
  const nodes = [], starts = [];
  let text = '', node;
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  while ((node = walker.nextNode())) {
    const parent = node.parentElement;
    if (!parent || /^(SCRIPT|STYLE|NOSCRIPT|TEXTAREA|INPUT)$/.test(parent.tagName)) continue;
    const style = getComputedStyle(parent);
    if (style.visibility === 'hidden' || style.display === 'none') continue;
    const range = document.createRange();
    range.selectNodeContents(node);
    if (!range.getClientRects().length) continue;
    starts.push(text.length); nodes.push(node); text += node.data;
    if (text.length > 2000000) return -1;
  }
  // ------------------------=
  // FUNC: fold
  // DESC: Folds ASCII case without shifting UTF-16 DOM range offsets.
  // ------------------=
  const fold = value => value.replace(/[A-Z]/g,
    // ------------------------=
    // FUNC: lowercaseCharacter
    // DESC: Converts one ASCII uppercase match without changing its length.
    // ------------------=
    c => c.toLowerCase());
  const needle = fold(query), haystack = fold(text), matches = [];
  for (let at = 0; matches.length < 65535;) {
    const found = haystack.indexOf(needle, at);
    if (found < 0) break;
    matches.push(found); at = found + needle.length;
  }
  if (!matches.length) return 0;
  const index = requested % matches.length, start = matches[index], end = start + query.length;
  let first = 0, last = 0;
  for (let i = 0; i < nodes.length; i++) {
    if (starts[i] <= start) first = i;
    if (starts[i] < end) last = i;
  }
  const range = document.createRange();
  range.setStart(nodes[first], start - starts[first]);
  range.setEnd(nodes[last], end - starts[last]);
  selection.addRange(range);
  nodes[first].parentElement.scrollIntoView({block: 'center', inline: 'nearest'});
  return (index + 1) * 65536 + matches.length;
})
