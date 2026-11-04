import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath, pathToFileURL } from 'node:url';

const consumer = process.env.TOKENMETER_PACKAGE;

async function entry() {
  if (!consumer) return new URL('../dist/tokenmeter.js', import.meta.url).href;
  const manifest = resolve(consumer, 'package.json');
  const name = Object.keys(JSON.parse(await readFile(manifest, 'utf8')).dependencies).find((d) => d === '@meritt/tokenmeter-wasm');
  return pathToFileURL(createRequire(manifest).resolve(name)).href;
}

const { load, TokenMeterError } = await import(await entry());
const tokens = await load();

const corpusPath = process.env.TOKENMETER_CORPUS ?? fileURLToPath(new URL('../../../testdata/o200k.tsv', import.meta.url));
const corpus = (await readFile(corpusPath, 'utf8'))
  .split('\n')
  .filter(Boolean)
  .map((line) => {
    const [ids, hex] = line.split('\t');
    return { text: Buffer.from(hex, 'hex').toString('utf8'), tokens: ids ? ids.split(',').length : 0 };
  });

test('counts a phrase from the site', () => {
  assert.equal(tokens.encoding, 'o200k_base');
  assert.equal(tokens.count('Пользователи — это наше всё.'), 9);
  assert.equal(tokens.count(''), 0);
});

test('splits into groups that join into the text', () => {
  const text = '👩🏽‍💻 é 🇷🇺 ไทย\r\nfn main() {}';
  const split = tokens.split(text);

  assert.equal(split.groups.map(group => group.text).join(''), text);
  assert.equal(split.tokens, tokens.count(text));
  assert.equal(split.truncated, false);
  assert.ok(split.groups.some(group => group.text.includes('👩🏽‍💻')));
});

test('stops at the limit', () => {
  const split = tokens.split('слово '.repeat(1000), 200);

  assert.equal(split.groups.length, 200);
  assert.equal(split.truncated, true);
});

test('refuses lone surrogates', () => {
  assert.throws(() => tokens.count('a\ud800b'), TokenMeterError);
});

test('counts the corpus as tiktoken', () => {
  assert.ok(corpus.length > 0);
  for (const c of corpus) {
    assert.equal(tokens.count(c.text), c.tokens, JSON.stringify(c.text));
  }
});
