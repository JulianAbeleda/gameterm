#!/usr/bin/env bash
#
# Translate English prose to Japanese with a local CTranslate2 model.
#
# Setup:
#   ci/scene-tts/setup-ct2-en-ja.sh
#
# Usage:
#   printf 'hello' | ci/scene-tts/ct2-en-to-ja.sh
#
# Optional:
#   GAMETERM_SCENE_TTS_CT2_PYTHON     Python with ctranslate2 + transformers.
#   GAMETERM_SCENE_TTS_CT2_MODEL_DIR  CTranslate2 model directory.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"

python_bin="${GAMETERM_SCENE_TTS_CT2_PYTHON:-$repo_root/.cache/scene-tts/ct2-venv/bin/python}"
model_dir="${GAMETERM_SCENE_TTS_CT2_MODEL_DIR:-$repo_root/.cache/scene-tts/models/fugumt-en-ja-ct2-int8}"

if [[ "${1:-}" == "--ready" ]]; then
  [[ -x "$python_bin" ]] || exit 1
  [[ -s "$model_dir/model.bin" ]] || exit 1
  [[ -s "$model_dir/source.spm" ]] || exit 1
  [[ -s "$model_dir/target.spm" ]] || exit 1
  [[ -s "$model_dir/vocab.json" ]] || exit 1
  exit 0
fi

[[ -x "$python_bin" ]] || {
  printf 'CTranslate2 Python is missing; run ci/scene-tts/setup-ct2-en-ja.sh\n' >&2
  exit 1
}

exec "$python_bin" "$script_dir/ct2-en-to-ja.py" --model-dir "$model_dir" "$@"
