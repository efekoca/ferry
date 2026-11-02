// Downloads php-src's UPGRADING files and prints every incompatible change, deprecation and removal as JSON,
// the starting point for docs/coverage.json when a new PHP version comes out:
// node scripts/php-upgrading/split.mjs 7.0 7.1 … 8.5 > items.json
const versions = process.argv.slice(2);
if (!versions.length) {
  console.error("usage: split.mjs <version>...");
  process.exit(2);
}
const keep = /^(Backward Incompatible Changes|Deprecated Functionality|Removed Extensions and SAPIs|Changes to INI File Handling)/;
const items = [];

for (const version of versions) {
  const res = await fetch(`https://raw.githubusercontent.com/php/php-src/PHP-${version}/UPGRADING`);
  if (!res.ok) throw new Error(`UPGRADING for ${version}: ${res.status}`);
  const lines = (await res.text()).split("\n");
  const toc = new Set();
  let section = null;
  let ext = "";
  let cur = null;
  let prev = "";
  const flush = () => {
    if (!cur) return;
    cur.text = cur.text.replace(/\s+/g, " ").trim();
    const ini = cur.section.startsWith("Changes to INI");
    if (cur.text && (!ini || /removed|deprecated|no longer/i.test(cur.text))) items.push(cur);
    cur = null;
  };
  for (const line of lines) {
    const heading = line.match(/^(\d+)\. (.+?)\s*$/);
    if (heading) {
      flush();
      if (!toc.has(heading[1])) {
        toc.add(heading[1]);
        section = null;
        continue;
      }
      section = keep.test(heading[2]) ? heading[2] : null;
      ext = "";
      prev = "";
      continue;
    }
    if (!section) continue;
    if (/^(-{3,}|={3,})$/.test(line.trim())) {
      flush();
      ext = prev;
      prev = "";
      continue;
    }
    if (line.trim()) prev = line.trim();
    const star = line.match(/^\* (.*)$/);
    const area = line.match(/^- (.+?):\s*$/);
    const bullet = line.match(/^ {2}\. (.*)$/);
    if (star || bullet) {
      flush();
      cur = { version, section, ext, text: (star ?? bullet)[1] };
    } else if (area) {
      flush();
      ext = area[1];
    } else if (cur && /^\s{2,}\S/.test(line)) {
      cur.text += ` ${line.trim()}`;
    }
  }
  flush();
}

process.stdout.write(`${JSON.stringify(items, null, 1)}\n`);
