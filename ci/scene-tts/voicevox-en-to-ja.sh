#!/usr/bin/env bash
#
# Translate English prose to Japanese and synthesize it with a local VOICEVOX
# engine for GameTerm Scene Mode's command TTS backend.
#
# Required:
#   GAMETERM_SCENE_TTS_OUTPUT          WAV output path GameTerm will play.
#
# Optional:
#   GAMETERM_SCENE_TTS_TRANSLATE_COMMAND
#                                      Command that reads English from stdin and
#                                      writes Japanese text to stdout.
#   GAMETERM_SCENE_TTS_SPEAKER         Speaker/context label from GameTerm.
#   GAMETERM_SCENE_TTS_SOURCE          Source label from GameTerm.
#   VOICEVOX_HOST                      Default: 127.0.0.1
#   VOICEVOX_PORT                      Default: 50021
#   VOICEVOX_SPEAKER                   Default: 14 (冥鳴ひまり / ノーマル)
#
# Example GameTerm launch:
#   GAMETERM_SCENE_TTS_BACKEND=command \
#   GAMETERM_SCENE_TTS_COMMAND=/Users/julianabeleda/env/gameterm/ci/scene-tts/voicevox-en-to-ja.sh \
#   GAMETERM_SCENE_TTS_PLAYER='afplay {output}' \
#   VOICEVOX_SPEAKER=14 \
#   gameterm start
#
# Example with an explicit translator:
#   GAMETERM_SCENE_TTS_TRANSLATE_COMMAND='codex exec "Translate the following English text to natural Japanese. Output only the Japanese translation."' \
#   GAMETERM_SCENE_TTS_BACKEND=command \
#   GAMETERM_SCENE_TTS_COMMAND=/Users/julianabeleda/env/gameterm/ci/scene-tts/voicevox-en-to-ja.sh \
#   GAMETERM_SCENE_TTS_PLAYER='afplay {output}' \
#   VOICEVOX_SPEAKER=14 \
#   gameterm start

set -euo pipefail

fail() {
  printf '%s\n' "$*" >&2
  exit 1
}

need_command() {
  command -v "$1" >/dev/null 2>&1 || fail "$1 is required"
}

trim_text() {
  sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'
}

translate_with_configured_command() {
  local text="$1"
  local command_text="$2"

  printf '%s' "$text" | bash -lc "$command_text"
}

translate_with_codex() {
  local text="$1"
  local prompt

  prompt='Translate the following English text to natural Japanese. Output only the Japanese translation.'
  printf '%s' "$text" | codex exec "$prompt"
}

translate_to_japanese() {
  local text="$1"

  if [[ -n "${GAMETERM_SCENE_TTS_TRANSLATE_COMMAND:-}" ]]; then
    translate_with_configured_command "$text" "$GAMETERM_SCENE_TTS_TRANSLATE_COMMAND"
    return
  fi

  if command -v codex >/dev/null 2>&1; then
    translate_with_codex "$text"
    return
  fi

  fail "translation command not configured; set GAMETERM_SCENE_TTS_TRANSLATE_COMMAND"
}

need_command curl
need_command jq

output_path="${GAMETERM_SCENE_TTS_OUTPUT:-}"
[[ -n "$output_path" ]] || fail "GAMETERM_SCENE_TTS_OUTPUT is required"

english_text="$(cat | trim_text)"
if [[ -z "$english_text" ]]; then
  exit 0
fi

output_dir="$(dirname "$output_path")"
mkdir -p "$output_dir"
rm -f "$output_path"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/gameterm-voicevox.XXXXXX")"
query_json="$tmp_dir/audio-query.json"
wav_tmp="$tmp_dir/output.wav"
translate_err="$tmp_dir/translate.err"
voicevox_err="$tmp_dir/voicevox.err"

cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

if ! japanese_text="$(translate_to_japanese "$english_text" 2>"$translate_err" | trim_text)"; then
  detail="$(cat "$translate_err" | trim_text)"
  [[ -n "$detail" ]] || detail="translation failed"
  fail "$detail"
fi
[[ -n "$japanese_text" ]] || fail "translation returned empty text"

host="${VOICEVOX_HOST:-127.0.0.1}"
port="${VOICEVOX_PORT:-50021}"
speaker="${VOICEVOX_SPEAKER:-14}"
base_url="http://${host}:${port}"

if ! curl -fsS --connect-timeout 2 --max-time 5 "$base_url/version" >/dev/null 2>"$voicevox_err"; then
  fail "VOICEVOX engine not reachable at ${host}:${port} - is it running?"
fi

encoded_text="$(printf '%s' "$japanese_text" | jq -sRr @uri)"

if ! curl -fsS \
  --connect-timeout 5 \
  --max-time 30 \
  -X POST \
  "${base_url}/audio_query?speaker=${speaker}&text=${encoded_text}" \
  -o "$query_json" \
  2>"$voicevox_err"; then
  detail="$(cat "$voicevox_err" | trim_text)"
  [[ -n "$detail" ]] || detail="VOICEVOX audio_query failed"
  fail "VOICEVOX audio_query failed: $detail"
fi

if ! jq -e type "$query_json" >/dev/null 2>&1; then
  fail "VOICEVOX audio_query returned invalid JSON"
fi

if ! curl -fsS \
  --connect-timeout 5 \
  --max-time 60 \
  -X POST \
  -H 'Content-Type: application/json' \
  --data-binary "@${query_json}" \
  "${base_url}/synthesis?speaker=${speaker}" \
  -o "$wav_tmp" \
  2>"$voicevox_err"; then
  detail="$(cat "$voicevox_err" | trim_text)"
  [[ -n "$detail" ]] || detail="VOICEVOX synthesis failed"
  fail "VOICEVOX synthesis failed: $detail"
fi

if [[ ! -s "$wav_tmp" ]]; then
  fail "VOICEVOX synthesis produced empty audio"
fi

mv "$wav_tmp" "$output_path"
[[ -s "$output_path" ]] || fail "TTS output file is empty"
