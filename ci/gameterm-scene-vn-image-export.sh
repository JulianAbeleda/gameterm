#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ci/gameterm-scene-vn-image-export.sh --source PATH --output-source-root PATH [OPTIONS]

Exports a local VN character image into the source-root layout consumed by
ci/gameterm-scene-vn-demo.sh. PSD, PNG, TIFF, and other sips-readable formats
are flattened to PNG before export.

Options:
  --source PATH              Local source image, usually a PSD.
  --output-source-root PATH  Asset source root to populate.
  --source-id TEXT           Catalog source directory. Default: 4cher_set4_vn_sprites.
  --character-id TEXT        Character filename prefix. Default: kiki.
  --expressions CSV          Expression suffixes. Default: neutral,happy,concerned,surprised,idle-0,idle-1,idle-2,idle-3,idle-4,idle-5.
  --force                    Overwrite existing exported files.
  -h, --help                 Show this help.
EOF
}

source_path=""
output_source_root=""
source_id="4cher_set4_vn_sprites"
character_id="kiki"
expressions_csv="neutral,happy,concerned,surprised,idle-0,idle-1,idle-2,idle-3,idle-4,idle-5"
force=0
tmp_paths=()

cleanup() {
  set +u
  for path in "${tmp_paths[@]}"; do
    if [[ -n "${path}" && -e "${path}" ]]; then
      rm -rf "${path}"
    fi
  done
  set -u
}
trap cleanup EXIT

require_value() {
  local flag="$1"
  local value="${2:-}"
  if [[ -z "${value}" ]]; then
    echo "${flag} requires a value" >&2
    usage >&2
    exit 2
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source)
      require_value "$1" "${2:-}"
      source_path="$2"
      shift 2
      ;;
    --output-source-root)
      require_value "$1" "${2:-}"
      output_source_root="$2"
      shift 2
      ;;
    --source-id)
      require_value "$1" "${2:-}"
      source_id="$2"
      shift 2
      ;;
    --character-id)
      require_value "$1" "${2:-}"
      character_id="$2"
      shift 2
      ;;
    --expressions)
      require_value "$1" "${2:-}"
      expressions_csv="$2"
      shift 2
      ;;
    --force)
      force=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "${source_path}" ]]; then
  echo "--source is required" >&2
  usage >&2
  exit 2
fi
if [[ -z "${output_source_root}" ]]; then
  echo "--output-source-root is required" >&2
  usage >&2
  exit 2
fi
if [[ ! -f "${source_path}" ]]; then
  echo "source image not found: ${source_path}" >&2
  exit 1
fi
if ! command -v sips >/dev/null 2>&1; then
  echo "sips is required to export local VN images on macOS" >&2
  exit 1
fi

IFS=',' read -r -a expressions <<<"${expressions_csv}"
if [[ "${#expressions[@]}" -eq 0 ]]; then
  echo "--expressions must include at least one expression" >&2
  exit 2
fi

for expression in "${expressions[@]}"; do
  if [[ -z "${expression}" ]]; then
    echo "--expressions contains an empty expression" >&2
    exit 2
  fi
done

target_dir="${output_source_root}/${source_id}"
tmp_png="$(mktemp /tmp/gameterm-scene-vn-image.XXXXXX.png)"
tmp_paths+=("${tmp_png}")
sips -s format png "${source_path}" --out "${tmp_png}" >/dev/null

if command -v file >/dev/null 2>&1; then
  if ! file "${tmp_png}" | grep -q "PNG image data"; then
    echo "exported image is not PNG image data: ${source_path}" >&2
    exit 1
  fi
fi

mkdir -p "${target_dir}"
for expression in "${expressions[@]}"; do
  target="${target_dir}/${character_id}-${expression}.png"
  if [[ -e "${target}" && "${force}" -ne 1 ]]; then
    echo "refusing to overwrite existing file without --force: ${target}" >&2
    exit 1
  fi
done

for expression in "${expressions[@]}"; do
  target="${target_dir}/${character_id}-${expression}.png"
  cp "${tmp_png}" "${target}"
  echo "Exported ${target}"
done
