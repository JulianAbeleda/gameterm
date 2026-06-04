#!/usr/bin/env bash
#
# Install a local CTranslate2 English -> Japanese translator for Scene TTS.
# The venv and model are written under .cache/scene-tts, which is gitignored.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cache_dir="$repo_root/.cache/scene-tts"
venv_dir="$cache_dir/ct2-venv"
model_dir="$cache_dir/models/fugumt-en-ja-ct2-int8"
python_bin="${GAMETERM_SCENE_TTS_CT2_BOOTSTRAP_PYTHON:-$(command -v python3.12 || command -v python3)}"

[[ -n "$python_bin" ]] || {
  printf 'python3.12 or python3 is required\n' >&2
  exit 1
}

mkdir -p "$cache_dir/models"
"$python_bin" -m venv "$venv_dir"
"$venv_dir/bin/python" -m pip install --upgrade pip setuptools wheel
"$venv_dir/bin/python" -m pip install ctranslate2 transformers sentencepiece huggingface_hub sacremoses torch
"$venv_dir/bin/ct2-transformers-converter" \
  --model staka/fugumt-en-ja \
  --output_dir "$model_dir" \
  --quantization int8 \
  --copy_files generation_config.json source.spm target.spm special_tokens_map.json tokenizer_config.json vocab.json \
  --force

"$script_dir/ct2-en-to-ja.sh" --ready
