#!/usr/bin/env python3
"""Precompute exact context-fill payloads (dev-time only).

The benchmark used to build context fills at request time by downloading a
corpus, tokenizing it, and walking a convergence loop — slow (and broken for
large server-mode fills). This script instead bakes the filler text once, at
development time, so the server can serve a precomputed string straight from
the embedded binary with no tokenizer/network work.

It writes one file per supported context size to ``payloads/context-{k}k.txt``
(k = kilo-tokens, matching ``src/tests.rs::CONTEXT_SIZES_K``). Each file is
EXACTLY ``k * 1024`` tokens under a single reference tokenizer
(``Qwen/Qwen3.8-27B`` by default). Because tokenization is model-specific, a
different model may count a payload as a few percent off — the reported prompt
tokens stay exact (they come from the model's usage), only the *nominal* size
label can drift. That is the accepted trade-off for instant fills.

The output is deterministic (no randomness): every size is a contiguous prefix
of the same lorem corpus, so the files are reproducible across runs.

Usage:
    python3 scripts/gen_context_payloads.py
    python3 scripts/gen_context_payloads.py --tokenizer deepseek-ai/DeepSeek-V3
    python3 scripts/gen_context_payloads.py --tokenizer /path/to/tokenizer.json
"""

import argparse
import os
import sys

try:
    from tokenizers import Tokenizer
except ImportError:
    sys.exit("requires the `tokenizers` Python package (pip install tokenizers)")

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(REPO_ROOT, "payloads")
# Must mirror src/tests.rs::CONTEXT_SIZES_K (and the FE fill selector).
SIZES_K = [1, 2, 4, 8, 16, 32, 64, 128, 192, 256, 384, 512]
DEFAULT_REF = "Qwen/Qwen3.8-27B"
# Local cache the dev box already has; falls back to an HF download otherwise.
CACHED_REF = os.path.join(REPO_ROOT, "velobench_data", "tokenizers", "Qwen_Qwen3.8-27B.json")

LOREM_PARAGRAPHS = [
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
    "Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
    "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.",
    "Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.",
    "Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo.",
    "Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt.",
    "Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et dolore magnam aliquam quaerat voluptatem.",
    "Ut enim ad minima veniam, quis nostrum exercitationem ullam corporis suscipit laboriosam, nisi ut aliquid ex ea commodi consequatur.",
]


def load_tokenizer(path_or_id: str) -> Tokenizer:
    if os.path.isfile(path_or_id):
        return Tokenizer.from_file(path_or_id)
    return Tokenizer.from_pretrained(path_or_id)


def build_lorem(tok: Tokenizer, target_tokens: int) -> str:
    """Deterministic lorem filler long enough for target_tokens, cycling paragraphs."""
    block = " ".join(LOREM_PARAGRAPHS) + " "
    # ~5.6 chars/token for lorem; pad generously and grow if still short.
    est_chars = target_tokens * 6
    text = block
    while len(text) < est_chars:
        text += block
    ids = tok.encode(text, add_special_tokens=False).ids
    while len(ids) < target_tokens:
        text += block
        ids = tok.encode(text, add_special_tokens=False).ids
    return text


def main() -> int:
    ap = argparse.ArgumentParser(description="Precompute exact context-fill payloads.")
    ap.add_argument(
        "--tokenizer",
        default=(CACHED_REF if os.path.isfile(CACHED_REF) else DEFAULT_REF),
        help="tokenizer.json path or HF repo id defining 'exact N tokens'",
    )
    ap.add_argument("--out", default=OUT_DIR, help="output directory (default: repo/payloads)")
    args = ap.parse_args()

    tok = load_tokenizer(args.tokenizer)
    print(f"reference tokenizer: {args.tokenizer}")

    max_tokens = max(SIZES_K) * 1024
    lorem = build_lorem(tok, max_tokens)
    all_ids = tok.encode(lorem, add_special_tokens=False).ids
    if len(all_ids) < max_tokens:
        sys.exit(f"lorem corpus only produced {len(all_ids)} tokens (need {max_tokens})")

    os.makedirs(args.out, exist_ok=True)
    total_bytes = 0
    print(f"{'size':>6}  {'tokens':>8}  {'bytes':>10}")
    for k in SIZES_K:
        n = k * 1024
        text = tok.decode(all_ids[:n], skip_special_tokens=False)
        check = len(tok.encode(text, add_special_tokens=False).ids)
        if check != n:
            sys.exit(f"size {k}K: decode/re-encode drift {check} != {n}")
        path = os.path.join(args.out, f"context-{k}k.txt")
        with open(path, "w", encoding="utf-8") as fh:
            fh.write(text)
        total_bytes += os.path.getsize(path)
        print(f"{k:>5}K  {n:>8}  {os.path.getsize(path):>10}")

    print(f"\nwrote {len(SIZES_K)} payloads to {args.out} ({total_bytes / 1024 / 1024:.1f} MiB total)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
