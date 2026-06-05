#!/usr/bin/env bash
set -euo pipefail

# Downloads the default local Whisper model used by Scene Mode STT.
#
# Env:
#   GAMETERM_SCENE_STT_WHISPER_MODEL  Override the model path.
#   GAMETERM_SCENE_STT_MODEL_URL       Override the download URL.
#
# Default model path on macOS:
#   ~/Library/Caches/gameterm/scene-stt/models/ggml-base.en.bin

model_path="${GAMETERM_SCENE_STT_WHISPER_MODEL:-${HOME}/Library/Caches/gameterm/scene-stt/models/ggml-base.en.bin}"
model_url="${GAMETERM_SCENE_STT_MODEL_URL:-https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin}"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required to download the Whisper model" >&2
  exit 1
fi

mkdir -p "$(dirname "${model_path}")"

if [[ -s "${model_path}" ]]; then
  echo "Whisper model already exists: ${model_path}"
  exit 0
fi

tmp_path="${model_path}.tmp"
rm -f "${tmp_path}"

echo "Downloading Whisper model to ${model_path}"
if ! curl --fail --location --show-error --output "${tmp_path}" "${model_url}"; then
  rm -f "${tmp_path}"
  echo "failed to download Whisper model from ${model_url}" >&2
  exit 1
fi

if [[ ! -s "${tmp_path}" ]]; then
  rm -f "${tmp_path}"
  echo "downloaded Whisper model is empty" >&2
  exit 1
fi

mv "${tmp_path}" "${model_path}"
echo "Installed Whisper model: ${model_path}"
