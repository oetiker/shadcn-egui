// Write one SVG per Hugeicons icon export, for icongen.
//   node hugeicons-svg.mjs <package dir> <out dir>
// <package dir> holds the extracted npm tarball (its `package/` folder).
// Each icon export is an array of [tag, attrs] pairs (P0c notes).
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

const [pkgDir, outDir] = process.argv.slice(2);
if (!pkgDir || !outDir) {
  console.error("usage: hugeicons-svg.mjs <package dir> <out dir>");
  process.exit(2);
}
const root = join(pkgDir, "package");
const meta = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const dot = meta.exports?.["."];
const fromExports = typeof dot === "string" ? dot : (dot?.import?.default ?? dot?.import ?? dot?.default);
const entry = meta.module ?? fromExports ?? meta.main;
const mod = await import(pathToFileURL(join(root, entry)).href);

// Tick02Icon -> tick-02, UnfoldMoreIcon -> unfold-more
const kebab = (s) => s.replace(/([a-z])([A-Z0-9])/g, "$1-$2").replace(/([0-9])([A-Z])/g, "$1-$2").toLowerCase();
const attr = (k) => k.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase());
const esc = (v) => String(v).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");

mkdirSync(outDir, { recursive: true });
const seen = new Set();
for (const [exportName, data] of Object.entries(mod)) {
  if (!exportName.endsWith("Icon") || !Array.isArray(data)) continue;
  // The index also re-exports aliases (renamed icons); only names that have
  // their own file in the package are real icons.
  if (!existsSync(join(dirname(join(root, entry)), `${exportName}.js`))) continue;
  const name = kebab(exportName.slice(0, -"Icon".length));
  if (seen.has(name)) continue; // two files that kebab to the same name
  seen.add(name);
  const body = data
    .map(([tag, attrs]) => {
      const a = Object.entries(attrs)
        .filter(([k]) => k !== "key")
        .map(([k, v]) => `${attr(k)}="${esc(v)}"`)
        .join(" ");
      return `<${tag} ${a}/>`;
    })
    .join("");
  writeFileSync(
    join(outDir, `${name}.svg`),
    `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor">${body}</svg>\n`,
  );
}
console.error(`hugeicons-svg: wrote ${seen.size} icons to ${outDir}`);
