import sys

import tiktoken
import tiktoken.load
import tiktoken_ext.openai_public as public

table, corpus = sys.argv[1], sys.argv[2]
load = tiktoken.load.load_tiktoken_bpe
public.load_tiktoken_bpe = lambda _url, expected_hash=None: load(table, expected_hash)
encoding = tiktoken.Encoding(**public.o200k_base())

with open(corpus, encoding="utf-8") as file:
    texts = [line.split("\t")[1] for line in file.read().split("\n") if line]

lines = []
for text in texts:
    ids = encoding.encode_ordinary(bytes.fromhex(text).decode("utf-8"))
    lines.append(",".join(map(str, ids)) + "\t" + text)

with open(corpus, "w", encoding="utf-8", newline="\n") as file:
    file.write("\n".join(lines) + "\n")

print(f"{corpus}: {len(lines)} texts encoded by tiktoken {tiktoken.__version__}")
