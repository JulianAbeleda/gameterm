#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ci/gameterm-scene-vn-demo.sh generate --output-dir PATH [OPTIONS]
  ci/gameterm-scene-vn-demo.sh install [OPTIONS]
  ci/gameterm-scene-vn-demo.sh doctor [OPTIONS]

Options:
  --source PATH                 VN script source.
  --source-dialect rpy          VN script dialect. Default: rpy.
  --source-title TEXT           Source title metadata.
  --source-version TEXT         Source version metadata.
  --asset-catalog PATH          Open asset catalog JSON.
  --asset-source-root PATH      Local extracted asset root.
  --output-dir PATH             Output directory for generate or doctor.
  --config-home PATH            Config root for install or doctor.
  --strict-images               Require doctor to validate real PNG files.
  --force                       Overwrite existing output/install files.
  --skip-assets                 Generate a script-only demo.
  -h, --help                    Show this help.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="${repo_root}/ci/fixtures/gameterm-scene"
command="${1:-}"
if [[ $# -gt 0 ]]; then
  shift
fi

source_path="${fixture_root}/renpy-demo-source.rpy"
source_dialect="rpy"
source_title="GameTerm Ren'Py Demo Fixture"
source_version="fixture"
asset_catalog="${fixture_root}/renpy-demo-open-assets.json"
asset_source_root=""
output_dir=""
config_home="${XDG_CONFIG_HOME:-${HOME}/.config}"
strict_images=0
force=0
skip_assets=0
tmp_paths=()

cleanup() {
  set +u
  for path in "${tmp_paths[@]}"; do
    if [[ -n "${path}" && -d "${path}" ]]; then
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
    --source-dialect)
      require_value "$1" "${2:-}"
      source_dialect="$2"
      shift 2
      ;;
    --source-title)
      require_value "$1" "${2:-}"
      source_title="$2"
      shift 2
      ;;
    --source-version)
      require_value "$1" "${2:-}"
      source_version="$2"
      shift 2
      ;;
    --asset-catalog)
      require_value "$1" "${2:-}"
      asset_catalog="$2"
      shift 2
      ;;
    --asset-source-root)
      require_value "$1" "${2:-}"
      asset_source_root="$2"
      shift 2
      ;;
    --output-dir)
      require_value "$1" "${2:-}"
      output_dir="$2"
      shift 2
      ;;
    --config-home)
      require_value "$1" "${2:-}"
      config_home="$2"
      shift 2
      ;;
    --strict-images)
      strict_images=1
      shift
      ;;
    --force)
      force=1
      shift
      ;;
    --skip-assets)
      skip_assets=1
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

case "${command}" in
  generate|install|doctor)
    ;;
  -h|--help)
    usage
    exit 0
    ;;
  "")
    echo "command is required: generate, install, or doctor" >&2
    usage >&2
    exit 2
    ;;
  *)
    echo "unknown command: ${command}" >&2
    usage >&2
    exit 2
    ;;
esac

if [[ "${source_dialect}" != "rpy" ]]; then
  echo "unsupported --source-dialect ${source_dialect}; expected rpy" >&2
  exit 2
fi

copy_generated_tree() {
  local source_dir="$1"
  local target_dir="$2"
  mkdir -p "${target_dir}"
  while IFS= read -r -d '' file; do
    local rel="${file#${source_dir}/}"
    local target="${target_dir}/${rel}"
    if [[ -e "${target}" && "${force}" -ne 1 ]]; then
      echo "refusing to overwrite existing file without --force: ${target}" >&2
      exit 1
    fi
  done < <(find "${source_dir}" -type f -print0)

  while IFS= read -r -d '' file; do
    local rel="${file#${source_dir}/}"
    local target="${target_dir}/${rel}"
    mkdir -p "$(dirname "${target}")"
    cp "${file}" "${target}"
  done < <(find "${source_dir}" -type f -print0)
}

validate_generated_dir() {
  local dir="$1"
  local scene="${dir}/default.json"
  local sprites="${dir}/sprites.json"
  local doctor_output
  local doctor_args
  doctor_output="$(mktemp /tmp/gameterm-scene-vn-demo-doctor.XXXXXX)"
  doctor_args=(
    --scene "${scene}"
    --sprites "${sprites}"
  )
  if [[ "${strict_images}" -eq 1 ]]; then
    doctor_args+=(--strict-images)
  fi

  cargo run -q -p gameterm-visual --example scene_validate -- "${scene}" >/dev/null
  if command -v jq >/dev/null 2>&1; then
    jq -e '.sprites | type == "array"' "${sprites}" >/dev/null
  fi
  "${repo_root}/ci/gameterm-scene-doctor.sh" "${doctor_args[@]}" >"${doctor_output}"
  grep -q "Doctor summary: 0 error(s)" "${doctor_output}"
  rm -f "${doctor_output}"
}

generate_to_dir() {
  local dir="$1"
  local scene="${dir}/default.json"
  local sprites="${dir}/sprites.json"
  local script_attribution="${dir}/vn-demo-script-attribution.json"
  local asset_attribution="${dir}/vn-demo-asset-attribution.json"
  local bindings="${dir}/vn-demo-bindings.json"
  local asset_output_root="${dir}/assets/vn-demo"
  local script_args=()

  mkdir -p "${dir}"

  if [[ "${skip_assets}" -eq 0 && -n "${asset_source_root}" ]]; then
    local asset_args=(
      --catalog "${asset_catalog}"
      --source-root "${asset_source_root}"
      --output-root "${asset_output_root}"
      --sprite-manifest "${sprites}"
      --attribution "${asset_attribution}"
      --bindings "${bindings}"
      --base-manifest "${fixture_root}/sprites.json"
    )
    cargo run -q -p gameterm-visual --example scene_vn_asset_intake -- "${asset_args[@]}"
    script_args+=(--bindings "${bindings}" --asset-root "${asset_output_root}")
  else
    cp "${fixture_root}/sprites.json" "${sprites}"
  fi

  cargo run -q -p gameterm-visual --example scene_vn_script_import -- \
    --source "${source_path}" \
    --output "${scene}" \
    --attribution "${script_attribution}" \
    --source-dialect "${source_dialect}" \
    --source-title "${source_title}" \
    --source-version "${source_version}" \
    --title "VN Script Demo Import" \
    "${script_args[@]}"

  validate_generated_dir "${dir}"
}

cd "${repo_root}"

case "${command}" in
  generate)
    if [[ -z "${output_dir}" ]]; then
      echo "--output-dir is required for generate" >&2
      exit 2
    fi
    tmp_dir="$(mktemp -d /tmp/gameterm-scene-vn-demo-generate.XXXXXX)"
    tmp_paths+=("${tmp_dir}")
    generate_to_dir "${tmp_dir}"
    copy_generated_tree "${tmp_dir}" "${output_dir}"
    echo "Generated Scene VN demo: ${output_dir}"
    ;;
  install)
    tmp_dir="$(mktemp -d /tmp/gameterm-scene-vn-demo-install.XXXXXX)"
    tmp_paths+=("${tmp_dir}")
    install_dir="${config_home}/gameterm/scenes"
    generate_to_dir "${tmp_dir}"
    copy_generated_tree "${tmp_dir}" "${install_dir}"
    echo "Installed Scene VN demo: ${install_dir}"
    ;;
  doctor)
    if [[ -n "${output_dir}" ]]; then
      doctor_dir="${output_dir}"
    else
      doctor_dir="${config_home}/gameterm/scenes"
    fi
    doctor_args=(
      --scene "${doctor_dir}/default.json" \
      --sprites "${doctor_dir}/sprites.json"
    )
    if [[ "${strict_images}" -eq 1 ]]; then
      doctor_args+=(--strict-images)
    fi
    "${repo_root}/ci/gameterm-scene-doctor.sh" "${doctor_args[@]}"
    ;;
esac
