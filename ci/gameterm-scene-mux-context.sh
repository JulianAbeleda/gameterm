#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-mux-context.sh COMMAND [OPTIONS]

Collect active GameTerm mux pane context and forward it to Scene workspace
discovery.

Commands:
  collect                       Print normalized mux context.
  discover                      Generate a workspace scene using mux context.
  patch                         Generate a workspace patch using mux context.
  doctor                        Validate mux context without writing output.

Common options:
  --pane-id ID                  Override collected pane id.
  --mux-window-id ID            Override collected mux window id.
  --cwd PATH                    Explicit workspace cwd override.
  --scene-output PATH           Forwarded to workspace discover.
  --patch-output PATH           Forwarded to workspace patch.
  --config-home PATH            Forwarded for workspace install.
  --fixture-context PATH        Use fixture JSON instead of live collection.
  --allow-missing               Succeed when mux context is unavailable.
  --format json|args            collect output format. Default: json.
  --install                     Forwarded only to workspace discover.
  --force                       Forwarded where downstream command supports it.

Live collection currently accepts context from either:
  GAMETERM_SCENE_MUX_CONTEXT    Path to normalized context JSON.
  GAMETERM_SCENE_PANE_ID
  GAMETERM_SCENE_MUX_WINDOW_ID
  GAMETERM_SCENE_PANE_CWD
  GAMETERM_SCENE_FOREGROUND_PROCESS_NAME
  GAMETERM_SCENE_FOREGROUND_PROCESS_PATH
  GAMETERM_SCENE_PANE_PROGRESS
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
command="${1:-}"

if [[ -z "${command}" ]]; then
  usage >&2
  exit 2
fi
shift

fixture_context=""
cwd=""
scene_output=""
patch_output=""
config_home=""
format="json"
allow_missing=0
install=0
force=0
override_pane_id=""
override_mux_window_id=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --fixture-context)
      fixture_context="$2"
      shift 2
      ;;
    --pane-id)
      override_pane_id="$2"
      shift 2
      ;;
    --mux-window-id)
      override_mux_window_id="$2"
      shift 2
      ;;
    --cwd)
      cwd="$2"
      shift 2
      ;;
    --scene-output)
      scene_output="$2"
      shift 2
      ;;
    --patch-output)
      patch_output="$2"
      shift 2
      ;;
    --config-home)
      config_home="$2"
      shift 2
      ;;
    --format)
      format="$2"
      shift 2
      ;;
    --allow-missing)
      allow_missing=1
      shift
      ;;
    --install)
      install=1
      shift
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

if [[ "${format}" != "json" && "${format}" != "args" ]]; then
  echo "--format must be json or args" >&2
  exit 2
fi
if [[ -n "${override_pane_id}" && ! "${override_pane_id}" =~ ^[0-9]+$ ]]; then
  echo "--pane-id must be a non-negative integer" >&2
  exit 2
fi
if [[ -n "${override_mux_window_id}" && ! "${override_mux_window_id}" =~ ^[0-9]+$ ]]; then
  echo "--mux-window-id must be a non-negative integer" >&2
  exit 2
fi

tmp_paths=()
cleanup() {
  set +u
  for path in "${tmp_paths[@]}"; do
    if [[ -n "${path}" && -f "${path}" ]]; then
      rm -f "${path}"
    fi
  done
  set -u
}
trap cleanup EXIT

new_tmp() {
  local path
  path="$(mktemp /tmp/gameterm-scene-mux-context.XXXXXX)"
  tmp_paths+=("${path}")
  printf '%s\n' "${path}"
}

resolve_existing_dir() {
  local path="$1"
  case "${path}" in
    /*) ;;
    *) path="${repo_root}/${path}" ;;
  esac
  if [[ ! -d "${path}" ]]; then
    echo "pane cwd does not exist or is not a directory: ${path}" >&2
    exit 2
  fi
  (cd "${path}" && pwd -P)
}

normalize_json() {
  local source_file="$1"
  local target_file="$2"
  jq '
    {
      source: (.source // "mux"),
      available: (.available // false),
      pane_id: (.pane_id // null),
      mux_window_id: (.mux_window_id // null),
      pane_cwd: (.pane_cwd // null),
      foreground_process_name: (.foreground_process_name // null),
      foreground_process_path: (.foreground_process_path // null),
      pane_progress: (.pane_progress // null),
      warnings: (if (.warnings? | type) == "array" then .warnings else [] end)
    }
  ' "${source_file}" >"${target_file}"
}

build_env_context() {
  local target_file="$1"
  local available=false
  if [[ -n "${GAMETERM_SCENE_PANE_ID:-}${GAMETERM_SCENE_MUX_WINDOW_ID:-}${GAMETERM_SCENE_PANE_CWD:-}${GAMETERM_SCENE_FOREGROUND_PROCESS_NAME:-}${GAMETERM_SCENE_FOREGROUND_PROCESS_PATH:-}${GAMETERM_SCENE_PANE_PROGRESS:-}" ]]; then
    available=true
  fi
  jq -n \
    --argjson available "${available}" \
    --arg pane_id "${GAMETERM_SCENE_PANE_ID:-}" \
    --arg mux_window_id "${GAMETERM_SCENE_MUX_WINDOW_ID:-}" \
    --arg pane_cwd "${GAMETERM_SCENE_PANE_CWD:-}" \
    --arg foreground_process_name "${GAMETERM_SCENE_FOREGROUND_PROCESS_NAME:-}" \
    --arg foreground_process_path "${GAMETERM_SCENE_FOREGROUND_PROCESS_PATH:-}" \
    --arg pane_progress "${GAMETERM_SCENE_PANE_PROGRESS:-}" \
    '{
      source: "env",
      available: $available,
      pane_id: (if $pane_id == "" then null else $pane_id end),
      mux_window_id: (if $mux_window_id == "" then null else $mux_window_id end),
      pane_cwd: (if $pane_cwd == "" then null else $pane_cwd end),
      foreground_process_name: (if $foreground_process_name == "" then null else $foreground_process_name end),
      foreground_process_path: (if $foreground_process_path == "" then null else $foreground_process_path end),
      pane_progress: (if $pane_progress == "" then null else $pane_progress end),
      warnings: (if $available then [] else ["live mux context is not available from environment"] end)
    }' >"${target_file}"
}

load_context() {
  local raw_file normalized_file
  raw_file="$(new_tmp)"
  normalized_file="$(new_tmp)"

  if [[ -n "${fixture_context}" ]]; then
    if [[ ! -f "${fixture_context}" ]]; then
      echo "fixture context does not exist: ${fixture_context}" >&2
      exit 2
    fi
    cp "${fixture_context}" "${raw_file}"
  elif [[ -n "${GAMETERM_SCENE_MUX_CONTEXT:-}" ]]; then
    if [[ ! -f "${GAMETERM_SCENE_MUX_CONTEXT}" ]]; then
      echo "GAMETERM_SCENE_MUX_CONTEXT does not exist: ${GAMETERM_SCENE_MUX_CONTEXT}" >&2
      exit 2
    fi
    cp "${GAMETERM_SCENE_MUX_CONTEXT}" "${raw_file}"
  else
    build_env_context "${raw_file}"
  fi

  normalize_json "${raw_file}" "${normalized_file}"
  printf '%s\n' "${normalized_file}"
}

validate_context() {
  local context_file="$1"
  local available pane_id mux_window_id pane_cwd resolved_cwd
  available="$(jq -r '.available == true' "${context_file}")"
  pane_id="$(jq -r '.pane_id // empty' "${context_file}")"
  mux_window_id="$(jq -r '.mux_window_id // empty' "${context_file}")"
  pane_cwd="$(jq -r '.pane_cwd // empty' "${context_file}")"

  if [[ "${available}" != "true" ]]; then
    if [[ "${allow_missing}" -eq 1 ]]; then
      return 0
    fi
    jq -r '.warnings[]?' "${context_file}" >&2
    echo "mux context is unavailable; pass --allow-missing to fall back" >&2
    exit 1
  fi

  if [[ -n "${override_pane_id}" ]]; then
    pane_id="${override_pane_id}"
  fi
  if [[ -n "${override_mux_window_id}" ]]; then
    mux_window_id="${override_mux_window_id}"
  fi
  if [[ -n "${pane_id}" && ! "${pane_id}" =~ ^[0-9]+$ ]]; then
    echo "pane_id must be a non-negative integer" >&2
    exit 2
  fi
  if [[ -n "${mux_window_id}" && ! "${mux_window_id}" =~ ^[0-9]+$ ]]; then
    echo "mux_window_id must be a non-negative integer" >&2
    exit 2
  fi
  if [[ -n "${pane_cwd}" ]]; then
    resolved_cwd="$(resolve_existing_dir "${pane_cwd}")"
    jq \
      --arg pane_cwd "${resolved_cwd}" \
      --arg pane_id "${pane_id}" \
      --arg mux_window_id "${mux_window_id}" \
      '.pane_cwd = $pane_cwd
        | .pane_id = (if $pane_id == "" then null else ($pane_id | tonumber) end)
        | .mux_window_id = (if $mux_window_id == "" then null else ($mux_window_id | tonumber) end)' \
      "${context_file}" >"${context_file}.next"
    mv "${context_file}.next" "${context_file}"
  else
    jq \
      --arg pane_id "${pane_id}" \
      --arg mux_window_id "${mux_window_id}" \
      '.pane_id = (if $pane_id == "" then null else ($pane_id | tonumber) end)
        | .mux_window_id = (if $mux_window_id == "" then null else ($mux_window_id | tonumber) end)' \
      "${context_file}" >"${context_file}.next"
    mv "${context_file}.next" "${context_file}"
  fi
}

build_workspace_args() {
  local context_file="$1"
  workspace_args=()
  context_args=()

  if [[ -n "${cwd}" ]]; then
    workspace_args+=(--cwd "${cwd}")
  fi

  if [[ "$(jq -r '.available == true' "${context_file}")" == "true" ]]; then
    local value
    value="$(jq -r '.pane_id // empty' "${context_file}")"
    if [[ -n "${value}" ]]; then
      context_args+=(--pane-id "${value}")
    fi
    value="$(jq -r '.mux_window_id // empty' "${context_file}")"
    if [[ -n "${value}" ]]; then
      context_args+=(--mux-window-id "${value}")
    fi
    value="$(jq -r '.pane_cwd // empty' "${context_file}")"
    if [[ -n "${value}" ]]; then
      context_args+=(--pane-cwd "${value}")
    fi
    value="$(jq -r '.foreground_process_name // empty' "${context_file}")"
    if [[ -n "${value}" ]]; then
      context_args+=(--foreground-process-name "${value}")
    fi
    value="$(jq -r '.foreground_process_path // empty' "${context_file}")"
    if [[ -n "${value}" ]]; then
      context_args+=(--foreground-process-path "${value}")
    fi
    value="$(jq -r '.pane_progress // empty' "${context_file}")"
    if [[ -n "${value}" ]]; then
      context_args+=(--pane-progress "${value}")
    fi
  fi
  workspace_args+=("${context_args[@]}")
}

run_collect() {
  local context_file
  context_file="$(load_context)"
  validate_context "${context_file}"
  build_workspace_args "${context_file}"
  case "${format}" in
    json)
      cat "${context_file}"
      ;;
    args)
      printf '%q ' "${context_args[@]}"
      printf '\n'
      ;;
  esac
}

run_discover() {
  local context_file
  context_file="$(load_context)"
  validate_context "${context_file}"
  build_workspace_args "${context_file}"
  if [[ -n "${scene_output}" ]]; then
    workspace_args+=(--scene-output "${scene_output}")
  fi
  if [[ -n "${config_home}" ]]; then
    workspace_args+=(--config-home "${config_home}")
  fi
  if [[ "${install}" -eq 1 ]]; then
    workspace_args+=(--install)
  fi
  if [[ "${force}" -eq 1 ]]; then
    workspace_args+=(--force)
  fi
  "${repo_root}/ci/gameterm-scene-workspace.sh" discover "${workspace_args[@]}"
}

run_patch() {
  local context_file
  context_file="$(load_context)"
  validate_context "${context_file}"
  build_workspace_args "${context_file}"
  if [[ -n "${patch_output}" ]]; then
    workspace_args+=(--patch-output "${patch_output}")
  fi
  if [[ "${force}" -eq 1 ]]; then
    workspace_args+=(--force)
  fi
  "${repo_root}/ci/gameterm-scene-workspace.sh" patch "${workspace_args[@]}"
}

run_doctor() {
  local context_file
  context_file="$(load_context)"
  validate_context "${context_file}"
  jq -r '
    "source=\(.source)",
    "available=\(.available)",
    "pane_id=\(.pane_id // "")",
    "mux_window_id=\(.mux_window_id // "")",
    "pane_cwd=\(.pane_cwd // "")",
    "foreground_process_name=\(.foreground_process_name // "")",
    "pane_progress=\(.pane_progress // "")",
    (.warnings[]? | "warning=\(.)")
  ' "${context_file}"
}

case "${command}" in
  collect)
    run_collect
    ;;
  discover)
    run_discover
    ;;
  patch)
    run_patch
    ;;
  doctor)
    run_doctor
    ;;
  *)
    echo "unknown command: ${command}" >&2
    usage >&2
    exit 2
    ;;
esac
