const VERSION = '@VERSION@';
const STATUS = ['ok', 'invalid_utf8', 'invalid_argument', 'internal'];
const OK = 0;
// tokenmeter_groups on wasm32: status, tokens, truncated, then groups as (pointer, length).
const SPLIT_SIZE = 20;

export class TokenMeterError extends Error {
  constructor(code) {
    super(code);
    this.name = 'TokenMeterError';
    this.code = code;
  }
}

export async function load(source = new URL('./tokenmeter.wasm', import.meta.url)) {
  const module = await compile(source);
  const { exports } = await WebAssembly.instantiate(module, {});
  return new TokenMeter(exports);
}

async function compile(source) {
  if (source instanceof WebAssembly.Module) return source;
  if (typeof source === 'string') source = new URL(source, import.meta.url);
  if (source instanceof URL) {
    if (source.protocol === 'file:') {
      const { readFile } = await import('node:fs/promises');
      return WebAssembly.compile(await readFile(source));
    }
    source = fetch(source);
  }
  if (source instanceof Promise || source instanceof Response) {
    return WebAssembly.compileStreaming(source);
  }
  return WebAssembly.compile(source);
}

const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });

function encode(text) {
  if (typeof text !== 'string') throw new TokenMeterError('invalid_argument');
  // A lone surrogate has no UTF-8 form; TextEncoder would count U+FFFD instead.
  if (!text.isWellFormed()) throw new TokenMeterError('invalid_utf8');
  return encoder.encode(text);
}

class TokenMeter {
  #x;

  constructor(exports) {
    this.#x = exports;
    const version = typeof exports.tokenmeter_version === 'function' ? this.#cString(exports.tokenmeter_version()) : '';
    if (version !== VERSION) throw new TokenMeterError('version');
    this.version = VERSION;
    this.encoding = this.#cString(exports.tokenmeter_encoding());
    exports.tokenmeter_prepare();
  }

  count(text) {
    const tokens = this.#with(encode(text), (p, n) => this.#x.tokenmeter_count(p, n) >>> 0);
    if (tokens === 0xffffffff) throw new TokenMeterError('invalid_utf8');
    return tokens;
  }

  split(text, limit = 200) {
    if (!Number.isSafeInteger(limit) || limit < 0) throw new TokenMeterError('invalid_argument');
    const bytes = encode(text);
    const x = this.#x;
    const out = x.tokenmeter_alloc(SPLIT_SIZE);
    if (out === 0) throw new TokenMeterError('internal');
    try {
      this.#with(bytes, (p, n) => x.tokenmeter_split(p, n, limit, out));
      // Read only after the call: memory may have grown and detached old views.
      const view = new DataView(x.memory.buffer, out, SPLIT_SIZE);
      const status = view.getUint32(0, true);
      if (status !== OK) {
        throw new TokenMeterError(STATUS[status] ?? 'internal');
      }
      const pairs = new Uint32Array(x.memory.buffer, view.getUint32(12, true), view.getUint32(16, true) * 2);
      const groups = [];
      let start = 0;
      for (let i = 0; i < pairs.length; i += 2) {
        groups.push({ text: decoder.decode(bytes.subarray(start, pairs[i])), tokens: pairs[i + 1] });
        start = pairs[i];
      }
      const result = { tokens: view.getUint32(4, true), groups, truncated: view.getUint32(8, true) !== 0 };
      x.tokenmeter_split_free(out);
      return result;
    } finally {
      x.tokenmeter_dealloc(out, SPLIT_SIZE);
    }
  }

  #with(bytes, use) {
    const x = this.#x;
    const n = bytes.length;
    const p = x.tokenmeter_alloc(n);
    if (n > 0 && p === 0) throw new TokenMeterError('invalid_argument');
    try {
      new Uint8Array(x.memory.buffer, p, n).set(bytes);
      return use(p, n);
    } finally {
      x.tokenmeter_dealloc(p, n);
    }
  }

  #cString(p) {
    const memory = new Uint8Array(this.#x.memory.buffer);
    let end = p;
    while (memory[end] !== 0) end++;
    return decoder.decode(memory.subarray(p, end));
  }
}
