# o200k_base corpus

`o200k.tsv` holds 2700 texts with the token IDs that OpenAI's tiktoken gives them in `o200k_base` through `encode_ordinary`. An implementation that matches the whole file encodes these texts as tiktoken does.

Each line is the IDs joined by commas (empty for an empty text), a tab and the text as hexadecimal UTF-8.

The IDs are generated, never edited by hand. `cargo xtask corpus` re-encodes the texts with the tiktoken version named in `o200k.sha256` and the table in `crates/tokenmeter-core/data`, then repins the file's SHA-256:

```bash
cargo xtask corpus
```

It runs `uv` with that tiktoken release. `cargo xtask corpus --check` compares the file with its pin.
