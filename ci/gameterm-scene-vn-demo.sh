#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-vn-demo.sh COMMAND [OPTIONS]

Owns the local Scene Mode VN demo config module.

Commands:
  generate   Generate a VN module into an explicit output directory.
  install    Generate, validate, back up, and install into Scene config.
  update     Migrate/validate the installed VN module in place.
  doctor     Validate the installed or generated VN module.
  backup     Back up the installed VN module files.
  smoke      Launch installed GameTerm, enter Scene Mode, and capture a screenshot.

Options:
  --config-home PATH       Config root. Default: XDG_CONFIG_HOME or ~/.config.
  --output-dir PATH        Output directory for generate/doctor.
  --asset-source-root PATH Local approved VN asset source root.
  --asset-catalog PATH     Asset catalog. Default: fixture open asset catalog.
  --app-path PATH          Installed app path for smoke. Default: ~/Applications/GameTerm.app.
  --output PATH            Smoke screenshot path.
  --force                  Allow overwrite for install/generate.
  --dry-run                Print update actions without writing.
  --strict-images          Require sprite files to be PNG image data in doctor.
  -h, --help               Show this help.

The VN module content belongs to config, not the app bundle:
  ${XDG_CONFIG_HOME:-~/.config}/gameterm/scenes/default.json
  ${XDG_CONFIG_HOME:-~/.config}/gameterm/scenes/sprites.json
  ${XDG_CONFIG_HOME:-~/.config}/gameterm/scenes/assets/vn-demo/...
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="${repo_root}/ci/fixtures/gameterm-scene"
command="${1:-}"
if [[ $# -gt 0 ]]; then
  shift
fi

config_home="${XDG_CONFIG_HOME:-${HOME}/.config}"
output_dir=""
asset_source_root=""
asset_catalog="${fixture_root}/vn-demo-open-assets.json"
app_path="${HOME}/Applications/GameTerm.app"
smoke_output="/tmp/gameterm-vn-config-module-smoke.png"
force=0
dry_run=0
strict_images=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config-home)
      config_home="$2"
      shift 2
      ;;
    --output-dir)
      output_dir="$2"
      shift 2
      ;;
    --asset-source-root)
      asset_source_root="$2"
      shift 2
      ;;
    --asset-catalog)
      asset_catalog="$2"
      shift 2
      ;;
    --app-path)
      app_path="$2"
      shift 2
      ;;
    --output)
      smoke_output="$2"
      shift 2
      ;;
    --force)
      force=1
      shift
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    --strict-images)
      strict_images=1
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

scene_dir() {
  printf '%s\n' "${config_home}/gameterm/scenes"
}

scene_path() {
  if [[ -n "${output_dir}" ]]; then
    printf '%s\n' "${output_dir}/default.json"
  else
    printf '%s\n' "$(scene_dir)/default.json"
  fi
}

sprites_path() {
  if [[ -n "${output_dir}" ]]; then
    printf '%s\n' "${output_dir}/sprites.json"
  else
    printf '%s\n' "$(scene_dir)/sprites.json"
  fi
}

compose_path() {
  printf '%s\n' "${config_home}/gameterm/scene-compose.json"
}

layout_path() {
  if [[ -n "${output_dir}" ]]; then
    printf '%s\n' "${output_dir}/vn-overlay-layout.json"
  else
    printf '%s\n' "$(scene_dir)/vn-overlay-layout.json"
  fi
}

backup_root() {
  printf '%s\n' "$(scene_dir)/backups"
}

timestamp() {
  date +%Y%m%d%H%M%S
}

require_command() {
  local name="$1"
  if ! command -v "${name}" >/dev/null 2>&1; then
    echo "missing required command: ${name}" >&2
    exit 2
  fi
}

require_asset_source_root() {
  if [[ -z "${asset_source_root}" ]]; then
    echo "--asset-source-root is required for ${command}" >&2
    exit 2
  fi
  if [[ ! -d "${asset_source_root}" ]]; then
    echo "asset source root does not exist: ${asset_source_root}" >&2
    exit 1
  fi
}

check_overwrite_allowed() {
  local target="$1"
  if [[ -e "${target}" && "${force}" -ne 1 ]]; then
    cat >&2 <<EOF
${target} already exists.

Rerun with --force to overwrite it.
EOF
    exit 1
  fi
}

write_native_vn_scene() {
  local target="$1"
  mkdir -p "$(dirname "${target}")"
  cat >"${target}" <<'EOF'
{
  "title": "Kiki VN Config Module",
  "background": "vn.background.school_classroom",
  "width": 16,
  "height": 9,
  "mode": {
    "mode_id": "vn-config-module",
    "label": "VN Config Module",
    "description": "Config-owned Kiki VN demo module",
    "scene_profile": "scene",
    "allowed_actions": [
      "Inspect",
      "AdvanceDialogue"
    ]
  },
  "stage": {
    "layers": [
      {
        "layer_id": "background",
        "zorder": 0,
        "displayables": [
          {
            "tag": "background",
            "sprite": "vn.background.school_classroom",
            "placement": "fullscreen",
            "zorder": 0
          }
        ]
      },
      {
        "layer_id": "characters",
        "zorder": 10,
        "displayables": [
          {
            "tag": "kiki",
            "sprite": "vn.character.kiki.neutral",
            "placement": "center",
            "zorder": 0
          }
        ]
      }
    ]
  },
  "variables": [
    {
      "key": "vn_config_module",
      "value": {
        "Bool": true
      }
    },
    {
      "key": "met_kiki",
      "value": {
        "Bool": true
      }
    }
  ],
  "entities": [
    {
      "id": "vn-config-module",
      "kind": "Project",
      "label": "VN Config Module",
      "position": {
        "x": 2,
        "y": 2
      },
      "sprite": "project_core",
      "metadata": [
        [
          "ownership",
          "config"
        ]
      ]
    },
    {
      "id": "kiki",
      "kind": "Agent",
      "label": "Kiki",
      "position": {
        "x": 7,
        "y": 4
      },
      "sprite": "vn.character.kiki.neutral",
      "metadata": [
        [
          "sprite_family",
          "vn.character.kiki"
        ]
      ]
    }
  ],
  "dialogue_speaker": "Codex",
  "dialogue": "",
  "dialogue_lines": [
    {
      "speaker": "Codex",
      "text": "Scene Mode is using the config-owned Kiki VN module.",
      "metadata": [
        [
          "source",
          "vn-config-module"
        ]
      ]
    }
  ],
  "choices": [
    {
      "label": "Inspect config module.",
      "kind": "Inspect",
      "policy": {
        "origin": "authored",
        "risk": "inspect",
        "scope": "scene",
        "summary": "Inspect the installed VN config module."
      }
    }
  ],
  "selected_entity": "kiki"
}
EOF
}

write_script_attribution() {
  local target="$1"
  mkdir -p "$(dirname "${target}")"
  cat >"${target}" <<'EOF'
{
  "generated_by": "gameterm-scene-vn-demo.sh",
  "source": "native Scene Mode JSON",
  "ownership": "user config module",
  "notes": [
    "Kiki sprites, school backgrounds, sprite manifest, and VN scene JSON are config-owned content.",
    "The app bundle owns the runtime and renderer, not this local VN module."
  ]
}
EOF
}

run_asset_intake() {
  local target_dir="$1"
  require_asset_source_root
  cargo run -q -p gameterm-visual --example scene_vn_asset_intake -- \
    --catalog "${asset_catalog}" \
    --source-root "${asset_source_root}" \
    --output-root "${target_dir}/assets/vn-demo" \
    --sprite-manifest "${target_dir}/sprites.json" \
    --attribution "${target_dir}/vn-demo-asset-attribution.json" \
    --bindings "${target_dir}/vn-demo-bindings.json" \
    --base-manifest "${fixture_root}/sprites.json" \
    --force
}

generate_module() {
  if [[ -z "${output_dir}" ]]; then
    echo "--output-dir is required for generate" >&2
    exit 2
  fi
  check_overwrite_allowed "${output_dir}/default.json"
  check_overwrite_allowed "${output_dir}/sprites.json"
  mkdir -p "${output_dir}"
  run_asset_intake "${output_dir}"
  write_native_vn_scene "${output_dir}/default.json"
  write_script_attribution "${output_dir}/vn-demo-script-attribution.json"
  doctor_module_for_paths "${output_dir}/default.json" "${output_dir}/sprites.json"
  echo "Generated VN config module: ${output_dir}"
}

backup_module() {
  local root
  local backup_dir
  root="$(backup_root)"
  backup_dir="${root}/vn-demo-$(timestamp)"
  mkdir -p "${backup_dir}"

  local copied=0
  local path
  for path in \
    "$(scene_dir)/default.json" \
    "$(scene_dir)/sprites.json" \
    "$(scene_dir)/vn-overlay-layout.json" \
    "$(scene_dir)/vn-demo-bindings.json" \
    "$(scene_dir)/vn-demo-asset-attribution.json" \
    "$(scene_dir)/vn-demo-script-attribution.json" \
    "$(compose_path)"
  do
    if [[ -e "${path}" ]]; then
      cp -R "${path}" "${backup_dir}/"
      copied=1
    fi
  done
  if [[ -d "$(scene_dir)/assets/vn-demo" ]]; then
    mkdir -p "${backup_dir}/assets"
    cp -R "$(scene_dir)/assets/vn-demo" "${backup_dir}/assets/"
    copied=1
  fi

  if [[ "${copied}" -eq 0 ]]; then
    echo "No VN module files found to back up under $(scene_dir)"
  else
    echo "Backed up VN config module: ${backup_dir}"
  fi
}

install_module() {
  require_asset_source_root
  local tmp_dir
  local target_dir
  tmp_dir="$(mktemp -d /tmp/gameterm-vn-module-install.XXXXXX)"
  target_dir="$(scene_dir)"
  output_dir="${tmp_dir}"
  generate_module >/tmp/gameterm-vn-module-generate.out

  mkdir -p "${target_dir}"
  local install_files=(
    default.json
    sprites.json
    vn-demo-bindings.json
    vn-demo-asset-attribution.json
    vn-demo-script-attribution.json
  )
  local file
  for file in "${install_files[@]}"; do
    check_overwrite_allowed "${target_dir}/${file}"
  done
  if [[ -d "${target_dir}/assets/vn-demo" && "${force}" -ne 1 ]]; then
    check_overwrite_allowed "${target_dir}/assets/vn-demo"
  fi

  if [[ "${force}" -eq 1 ]]; then
    backup_module
  fi

  for file in "${install_files[@]}"; do
    cp "${tmp_dir}/${file}" "${target_dir}/${file}"
  done
  mkdir -p "${target_dir}/assets"
  rm -rf "${target_dir}/assets/vn-demo"
  cp -R "${tmp_dir}/assets/vn-demo" "${target_dir}/assets/vn-demo"

  output_dir=""
  doctor_module
  echo "Installed VN config module: ${target_dir}"
}

stale_origin_count() {
  local scene="$1"
  if [[ ! -f "${scene}" ]]; then
    echo 0
    return
  fi
  jq -r '[.choices[]? | select(.policy.origin? == "vn_script_import")] | length' "${scene}"
}

migrate_stale_origins() {
  local scene="$1"
  local count
  count="$(stale_origin_count "${scene}")"
  if [[ "${count}" == "0" ]]; then
    echo "No stale VN policy origins found."
    return
  fi

  echo "Found ${count} stale policy origin(s): vn_script_import -> authored"
  if [[ "${dry_run}" -eq 1 ]]; then
    echo "Dry run: would migrate ${scene}"
    return
  fi

  backup_module
  local tmp
  tmp="$(mktemp /tmp/gameterm-vn-origin-migrate.XXXXXX)"
  jq '(.choices[]?.policy.origin | select(. == "vn_script_import")) = "authored"' \
    "${scene}" >"${tmp}"
  mv "${tmp}" "${scene}"
  echo "Migrated stale policy origins in ${scene}"
}

doctor_module_for_paths() {
  local scene="$1"
  local sprites="$2"
  local doctor_args=(--scene "${scene}" --sprites "${sprites}")
  if [[ "${strict_images}" -eq 1 ]]; then
    doctor_args+=(--strict-images)
  fi
  "${repo_root}/ci/gameterm-scene-doctor.sh" "${doctor_args[@]}"
}

check_optional_json() {
  local label="$1"
  local path="$2"
  if [[ -f "${path}" ]]; then
    if jq empty "${path}" >/dev/null; then
      echo "OK: ${label} JSON parses: ${path}"
    else
      echo "ERROR: ${label} JSON does not parse: ${path}" >&2
      return 1
    fi
  else
    echo "OK: ${label} config absent: ${path}"
  fi
}

doctor_module() {
  local scene
  local sprites
  scene="$(scene_path)"
  sprites="$(sprites_path)"
  echo "VN config module doctor"
  echo "Config home: ${config_home}"
  echo "Scene: ${scene}"
  echo "Sprites: ${sprites}"
  echo

  local stale_count
  stale_count="$(stale_origin_count "${scene}")"
  if [[ "${stale_count}" != "0" ]]; then
    echo "WARN: ${stale_count} stale policy origin(s): vn_script_import"
    echo "SUGGEST: run ci/gameterm-scene-vn-demo.sh update --config-home ${config_home}"
    echo
  fi

  doctor_module_for_paths "${scene}" "${sprites}"
  check_optional_json "VN overlay layout" "$(layout_path)"
  check_optional_json "Scene compose" "$(compose_path)"
}

update_module() {
  migrate_stale_origins "$(scene_path)"
  if [[ "${dry_run}" -eq 1 ]]; then
    return
  fi
  doctor_module
}

smoke_module() {
  doctor_module
  local gui_bin="${app_path}/Contents/MacOS/gameterm-gui"
  if [[ ! -x "${gui_bin}" ]]; then
    echo "installed GameTerm GUI not found: ${gui_bin}" >&2
    exit 1
  fi
  if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "installed app smoke is only supported on macOS" >&2
    exit 2
  fi
  if ! command -v osascript >/dev/null 2>&1 || ! command -v screencapture >/dev/null 2>&1; then
    echo "smoke requires osascript and screencapture" >&2
    exit 2
  fi

  pkill -f "${gui_bin}" >/dev/null 2>&1 || true
  open "${app_path}"
  sleep 2
  osascript <<'APPLESCRIPT'
tell application "GameTerm" to activate
delay 0.5
tell application "System Events"
  keystroke "1"
  delay 0.2
  key code 36
end tell
APPLESCRIPT
  sleep 3
  screencapture -x "${smoke_output}"
  if [[ ! -s "${smoke_output}" ]]; then
    echo "smoke screenshot was not written: ${smoke_output}" >&2
    exit 1
  fi
  echo "Wrote VN config module smoke screenshot: ${smoke_output}"
  echo "Manual check: screenshot should show school background, Kiki, dialogue panel, composer dock, and no load-error frame."
}

case "${command}" in
  generate)
    require_command jq
    generate_module
    ;;
  install)
    require_command jq
    install_module
    ;;
  update)
    require_command jq
    update_module
    ;;
  doctor)
    require_command jq
    doctor_module
    ;;
  backup)
    backup_module
    ;;
  smoke)
    require_command jq
    smoke_module
    ;;
  -h|--help|"")
    usage
    if [[ -z "${command}" ]]; then
      exit 2
    fi
    ;;
  *)
    echo "unknown command: ${command}" >&2
    usage >&2
    exit 2
    ;;
esac
