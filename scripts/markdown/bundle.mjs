// Development-only asset generation. Node/npm are not used by the desktop app.
import { build } from 'esbuild';
import { copyFile, mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
const here = path.dirname(fileURLToPath(import.meta.url));
const output = path.resolve(here, '../../assets/markdown');
await mkdir(output, { recursive: true });
await build({
  absWorkingDir: here, entryPoints: ['renderer.js'], bundle: true, minify: true,
  format: 'iife', platform: 'browser', target: ['safari15', 'chrome100'],
  outfile: path.join(output, 'renderer.js'), legalComments: 'linked',
});
// Preserve the official TeaVM engine verbatim: re-minifying its generated
// state-machine labels can produce JavaScript that browsers cannot parse.
for (const name of ['plantuml.js', 'themes.js', 'emoji.js', 'openiconic.js']) {
  await copyFile(path.join(here, 'node_modules/@plantuml/core', name), path.join(output, name));
}
for (const name of ['renderer.js', 'plantuml.js']) {
  execFileSync(process.execPath, ['--check', '--input-type=module'], {
    input: await readFile(path.join(output, name)),
    stdio: ['pipe', 'pipe', 'pipe'],
  });
}
// Include full license texts for all packages that can be in the bundled JS.
const notices = [];
async function scan(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    if (!entry.isDirectory() || entry.name.startsWith('.')) continue;
    const folder = path.join(dir, entry.name);
    if (entry.name.startsWith('@')) { await scan(folder); continue; }
    let pkg;
    try { pkg = JSON.parse(await readFile(path.join(folder, 'package.json'), 'utf8')); }
    catch { continue; }
    const files = (await readdir(folder)).filter(name => /^(licen[sc]e|copying|notice)(\.|$)/i.test(name));
    for (const file of files) {
      try { notices.push(`\n## ${pkg.name} ${pkg.version} (${pkg.license || 'see license'})\n\n${await readFile(path.join(folder, file), 'utf8')}`); }
      catch { /* directories with the same name are not license files */ }
    }
    try { await scan(path.join(folder, 'node_modules')); } catch { /* no nested dependencies */ }
  }
}
await scan(path.join(here, 'node_modules'));
await writeFile(path.join(output, 'THIRD_PARTY_LICENSES.txt'), notices.join('\n'));
