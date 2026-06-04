#!/usr/bin/env python3
"""Translate English prose to Japanese with a local CTranslate2 model."""

from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

import ctranslate2
import sentencepiece as spm


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model-dir", required=True, type=Path)
    parser.add_argument("--beam-size", default=1, type=int)
    parser.add_argument("--max-decoding-length", default=192, type=int)
    parser.add_argument("--timing", action="store_true")
    return parser


def translate(
    text: str,
    model_dir: Path,
    beam_size: int,
    max_decoding_length: int,
    timing: bool,
) -> str:
    started = time.perf_counter()
    source_sp = spm.SentencePieceProcessor(model_file=str(model_dir / "source.spm"))
    target_sp = spm.SentencePieceProcessor(model_file=str(model_dir / "target.spm"))
    tokenizer_loaded = time.perf_counter()
    translator = ctranslate2.Translator(str(model_dir), device="cpu", compute_type="int8")
    translator_loaded = time.perf_counter()

    source_tokens = source_sp.encode(text, out_type=str) + ["</s>"]
    results = translator.translate_batch(
        [source_tokens],
        beam_size=beam_size,
        max_decoding_length=max_decoding_length,
    )
    translated_tokens = results[0].hypotheses[0]
    translated = target_sp.decode(
        [token for token in translated_tokens if token != "</s>"]
    ).strip()
    finished = time.perf_counter()

    if timing:
        print(
            "ct2_timing "
            f"tokenizer={tokenizer_loaded - started:.3f}s "
            f"translator={translator_loaded - tokenizer_loaded:.3f}s "
            f"translate={finished - translator_loaded:.3f}s "
            f"total={finished - started:.3f}s",
            file=sys.stderr,
        )

    return translated


def main() -> int:
    args = build_parser().parse_args()
    text = sys.stdin.read().strip()
    if not text:
        return 0

    missing = [
        name
        for name in ("model.bin", "source.spm", "target.spm", "vocab.json")
        if not (args.model_dir / name).exists()
    ]
    if missing:
        print(
            f"CTranslate2 model is incomplete in {args.model_dir}: missing {', '.join(missing)}",
            file=sys.stderr,
        )
        return 1

    try:
        translated = translate(
            text,
            args.model_dir,
            args.beam_size,
            args.max_decoding_length,
            args.timing,
        )
    except Exception as err:
        print(f"CTranslate2 translation failed: {err}", file=sys.stderr)
        return 1

    if translated:
        print(translated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
