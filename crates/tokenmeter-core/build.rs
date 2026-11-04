use std::path::PathBuf;
use std::{env, fs};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256};

const SOURCE: &str = "data/o200k_base.tiktoken";
const SHA256: &str = "446a9538cb6c348e3516120d7c08b09f57c36495e2acfffe59a5bf8b0cfb1a2d";

fn main() {
    println!("cargo::rerun-if-changed={SOURCE}");
    println!("cargo::rerun-if-changed=build.rs");

    let data = fs::read_to_string(SOURCE).expect("data/o200k_base.tiktoken");
    let hash = Sha256::digest(data.as_bytes());
    let pinned: Vec<u8> =
        (0..SHA256.len()).step_by(2).map(|at| u8::from_str_radix(&SHA256[at..at + 2], 16).expect("hex")).collect();

    assert_eq!(hash.as_slice(), pinned, "data/o200k_base.tiktoken is not the file tiktoken pins");

    let mut table = Vec::with_capacity(data.len());

    for (expected, line) in data.lines().enumerate() {
        let (token, rank) = line.split_once(' ').expect("a token and its rank");
        let token = STANDARD.decode(token).expect("base64");
        let rank: usize = rank.parse().expect("a rank");

        assert_eq!(rank, expected, "ranks run from 0 without gaps");

        table.push(u8::try_from(token.len()).expect("a token of at most 255 bytes"));
        table.extend_from_slice(&token);
    }

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("o200k_base.bin");

    fs::write(out, table).expect("the compact table");
}
