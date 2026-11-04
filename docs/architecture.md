# Architecture

## Scope

tokenmeter counts the tokens of a text in `o200k_base` and splits the start of the text into groups of tokens on grapheme cluster boundaries. `count` and `encode` match tiktoken's `encode_ordinary`, special tokens included as plain text. `split` returns at most `limit` groups and whether the text goes on. Output is pinned by `testdata/o200k.tsv`.

## Components

```text
data/o200k_base.tiktoken ─► build.rs ─► compact table
                                              │
                                      tokenmeter-core     pure functions; no I/O, no unsafe
                                              │           CoreBPE from tiktoken-rs, unicode-segmentation
                                      tokenmeter-ffi      C ABI; the only unsafe code
                                  ┌───────────┴────────────┐
           tokenmeter.wasm + tokenmeter.js        TokenMeterFFI.xcframework + TokenMeter (Swift)
```

Dependencies point down only. Bindings add no policy: they marshal arguments, map status codes and cut group text from the caller's own bytes.

## Table

`build.rs` checks the SHA-256 of `data/o200k_base.tiktoken` against the hash tiktoken pins and stops the build on a mismatch. It writes a compact table, one length byte and the token bytes per rank, which `include_bytes!` embeds, so the module needs neither a request nor a file. On first use, or earlier through `prepare`, the table becomes a `CoreBPE` once per process. Nearly all of `tokenmeter.wasm` is this table.

`bpe-openai` was tried first: linear in time, but its serialized `o200k_base` with prebuilt automata is 32 MB, too large for a page.

## Groups

`split` walks the tokens and closes a group where a token ends on a grapheme cluster boundary. Emoji with modifiers, flags and `\r\n` never straddle groups, and a group's tokens never cross its edges, so a group can be shown whole and labelled with its count.

## C ABI

- Text is UTF-8 in memory from `tokenmeter_alloc`, released with `tokenmeter_dealloc`.
- `tokenmeter_count` returns the count, or `TOKENMETER_NOT_UTF8` for input that is not UTF-8.
- `tokenmeter_split` fills a `tokenmeter_groups`: a status, the token count, `truncated` and `groups_len` pairs of group end in bytes and token count, released with `tokenmeter_split_free`. Group text does not cross the boundary. Input longer than 32-bit offsets reach is `TOKENMETER_INVALID_ARGUMENT`.
- Calls run on any thread; the immutable table is all they share.
- A panic in `tokenmeter_split` never crosses the boundary: native builds return `TOKENMETER_INTERNAL`. `tokenmeter_count` has no status, and a panic in it aborts. In wasm32 a panic traps the instance.
- `tokenmeter_encoding` names the encoding; `tokenmeter_version` identifies the build, and bindings refuse a library whose version differs from their own.
- cbindgen generates `include/tokenmeter.h` from `tokenmeter-ffi`; Miri runs the ABI tests except the one that builds the table, which takes Miri longer than CI allows; the marshalling it covers is tested on hand-made splits.

## Bindings

**WebAssembly.** `tokenmeter.wasm` is the `cdylib` of `tokenmeter-ffi` with no generated glue, optimized with `wasm-opt -O3`. `tokenmeter.js` is one ES module without static imports, so it runs in a page, a worker and Node; it reads a file only for a `file:` URL. `load` prepares the table at once. A lone surrogate is refused with `invalid_utf8`, since `TextEncoder` would count U+FFFD instead. Results are read from memory only after the call, which may have grown it.

**Swift.** `TokenMeterFFI.xcframework` holds one universal static library for `arm64` and `x86_64` with the header as module `TokenMeterFFI` under `Headers/TokenMeterFFI/`, which keeps its module map distinct from other Rust libraries in the same app. `Package.swift` uses a local build when present, otherwise the release binary pinned by URL and checksum. `TokenMeter` calls the same functions; `split` throws a typed `TokenMeterError`.

## Conformance

`testdata/o200k.tsv` holds 2700 texts with the token IDs tiktoken gives them. `cargo xtask corpus` re-encodes it with the tiktoken release named in `testdata/o200k.sha256` and repins the digest; CI checks the pin. The Rust tests compare IDs and counts, the WebAssembly and Swift tests compare counts.

## Distribution

| Artifact | Consumer |
|---|---|
| `tokenmeter-core` crate | crates.io |
| `tokenmeter-vX.Y.Z-npm.tgz`, package `@meritt/tokenmeter-wasm` | npmjs.com |
| `tokenmeter-vX.Y.Z-xcframework.zip` | SwiftPM, through `Package.swift`, macOS 26 |

The release set is reproducible: the tarball and the zip have fixed entry order, times and modes. The checksum pinned in `Package.swift` before tagging is therefore the checksum of the tagged build.
