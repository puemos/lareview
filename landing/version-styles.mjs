import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';

const [css, html] = await Promise.all([
  readFile('dist/styles/landing.css'),
  readFile('dist/index.html', 'utf8'),
]);
const stylesheet = /href="\.\/styles\/landing\.css(?:\?v=[a-f0-9]+)?"/g;
if ((html.match(stylesheet) ?? []).length !== 1) {
  throw new Error('Expected one local landing stylesheet to version.');
}
const version = createHash('sha256').update(css).digest('hex').slice(0, 12);
await writeFile('dist/index.html', html.replace(stylesheet, `href="./styles/landing.css?v=${version}"`));
