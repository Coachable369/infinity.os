// Mechanical, idempotent reflow of the authored sidebar; preserves other template bytes.
import fs from 'node:fs';
const path = 'assets/boot/settings-screens.infinityui';
const names = ['General','ThemesSkins','UsersAccounts','AIVoice','PrivacySecurity','Devices','Network','NodesMesh','Storage','About','Input'];
const source = fs.readFileSync(path, 'utf8');
let count = 0;
const result = source.replace(/("codeIdentifier" : "navigation(\w+)",[\s\S]*?"frame" : \{)([\s\S]*?)(\n\s*\})/g,
  // ------------------------=
  // FUNC: reflow
  // DESC: Applies kit gutters and twelve-unit row gaps to item, icon and label frames.
  // ------------------=
  function reflow(match, prefix, name, frame, suffix) {
    const base = name.replace(/(Icon|Label)$/, '');
    const index = names.indexOf(base);
    if (index < 0) return match;
    const icon = name.endsWith('Icon'), label = name.endsWith('Label');
    const values = {x: icon ? 87 : label ? 123 : 71,
      y: 170 + index*62 + (icon || label ? 13 : 0),
      width: icon ? 24 : label ? 150 : 218, height: icon || label ? 24 : 50};
    count++;
    return prefix + frame.replace(/"(x|y|width|height)" : \d+/g, (_, key) => `"${key}" : ${values[key]}`) + suffix;
  });
if (count !== 11*11*3) throw new Error(`Unexpected sidebar element count: ${count}`);
JSON.parse(result);
fs.writeFileSync(path, result);
