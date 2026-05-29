#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-smoke.sh [OPTIONS]

Checks whether macOS ffmpeg screen capture works for GameTerm Scene Mode smoke
testing. By default it lists AVFoundation devices and attempts one still-frame
capture from "0:none".

Options:
  --launch                 Launch target/debug/gameterm-gui with the renderer
                           row fixture in a temporary XDG_CONFIG_HOME.
  --check-assets           Check bundled Scene Mode PNG assets and fixture
                           sprite manifests without launching or capturing.
  --fixture NAME           Fixture to install when --launch is used: basic,
                           navigate, invalid, sprites, missing-sprite,
                           run-command-targets, or renderer-rows. Default:
                           renderer-rows.
  --patch-inbox PATH       Set GAMETERM_SCENE_PATCH_FILE to PATH when
                           launching. Use "auto" to create a temporary inbox.
  --submit-mux-patch PATH  After --launch wait, submit PATCH through
                           gameterm cli scene-patch before capture.
  --submit-target-pane-id ID
                           Target pane id for --submit-mux-patch. Optional.
  --wait-before-capture N  Seconds to wait after launch before capture.
                           Use this time to press Ctrl+Shift+G. Default: 10.
  --device DEVICE          AVFoundation device string. Default: 0:none.
  --output PATH            Capture output path. Default:
                           /tmp/gameterm-scene-smoke.png.
  --min-bytes N            Minimum acceptable capture size. Default: 1000.
  --capture-timeout N      Seconds to wait for one frame before diagnosing a
                           hang. Default: 12.
  --ffmpeg PATH            ffmpeg binary. Defaults to PATH lookup, then common
                           Homebrew locations.
  -h, --help               Show this help.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="${repo_root}/ci/fixtures/gameterm-scene"
device="0:none"
output="/tmp/gameterm-scene-smoke.png"
min_bytes=1000
capture_timeout=12
launch=0
check_assets=0
fixture="renderer-rows"
wait_before_capture=10
ffmpeg_bin="${FFMPEG:-}"
gui_pid=""
tmp_home=""
patch_inbox=""
submit_mux_patch=""
submit_target_pane_id=""
log_file="/tmp/gameterm-scene-smoke-ffmpeg.log"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --launch)
      launch=1
      shift
      ;;
    --check-assets)
      check_assets=1
      shift
      ;;
    --fixture)
      fixture="$2"
      shift 2
      ;;
    --patch-inbox)
      patch_inbox="$2"
      shift 2
      ;;
    --submit-mux-patch)
      submit_mux_patch="$2"
      shift 2
      ;;
    --submit-target-pane-id)
      submit_target_pane_id="$2"
      shift 2
      ;;
    --wait-before-capture)
      wait_before_capture="$2"
      shift 2
      ;;
    --device)
      device="$2"
      shift 2
      ;;
    --output)
      output="$2"
      shift 2
      ;;
    --min-bytes)
      min_bytes="$2"
      shift 2
      ;;
    --capture-timeout)
      capture_timeout="$2"
      shift 2
      ;;
    --ffmpeg)
      ffmpeg_bin="$2"
      shift 2
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

cleanup() {
  if [[ -n "${gui_pid}" ]] && kill -0 "${gui_pid}" 2>/dev/null; then
    kill "${gui_pid}" 2>/dev/null || true
  fi
  if [[ -n "${tmp_home}" && -d "${tmp_home}" ]]; then
    rm -rf "${tmp_home}"
  fi
}
trap cleanup EXIT

resolve_ffmpeg() {
  if [[ -n "${ffmpeg_bin}" ]]; then
    printf '%s\n' "${ffmpeg_bin}"
    return
  fi
  if command -v ffmpeg >/dev/null 2>&1; then
    command -v ffmpeg
    return
  fi
  for candidate in \
    /opt/homebrew/bin/ffmpeg \
    /usr/local/bin/ffmpeg \
    /opt/homebrew/opt/ffmpeg/bin/ffmpeg \
    /usr/local/opt/ffmpeg/bin/ffmpeg
  do
    if [[ -x "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return
    fi
  done
  return 1
}

install_scene_fixture() {
  local scene_dir="$1"
  mkdir -p "${scene_dir}"

  case "${fixture}" in
    renderer-rows)
      cp "${repo_root}/docs/examples/gameterm-scene-renderer-rows.json" \
        "${scene_dir}/default.json"
      ;;
    basic)
      cp "${fixture_root}/default.json" "${scene_dir}/default.json"
      ;;
    navigate)
      cp "${fixture_root}/default.json" "${scene_dir}/default.json"
      cp "${fixture_root}/memory.json" "${scene_dir}/memory.json"
      ;;
    invalid)
      cp "${fixture_root}/invalid.json" "${scene_dir}/default.json"
      ;;
    sprites)
      cp "${fixture_root}/default.json" "${scene_dir}/default.json"
      cp "${fixture_root}/sprites.json" "${scene_dir}/sprites.json"
      ;;
    missing-sprite)
      cp "${fixture_root}/default.json" "${scene_dir}/default.json"
      cp "${fixture_root}/sprites-missing.json" "${scene_dir}/sprites.json"
      ;;
    run-command-targets)
      cp "${fixture_root}/run-command-targets.json" "${scene_dir}/default.json"
      cp "${fixture_root}/sprites.json" "${scene_dir}/sprites.json"
      ;;
    *)
      echo "unknown fixture: ${fixture}" >&2
      usage >&2
      exit 2
      ;;
  esac
}

check_bundled_assets() {
  local expected=(
    workspace-map.png
    project-core.png
    task-tile.png
    agent-idle.png
    memory-note.png
  )
  local asset
  for asset in "${expected[@]}"; do
    local path="${repo_root}/assets/gameterm-scene/${asset}"
    if [[ ! -f "${path}" ]]; then
      echo "missing bundled Scene Mode asset: ${path}" >&2
      exit 6
    fi
    if ! file "${path}" | grep -q 'PNG image data, 32 x 32'; then
      file "${path}" >&2 || true
      echo "bundled Scene Mode asset is not a 32x32 PNG: ${path}" >&2
      exit 6
    fi
  done

  "${repo_root}/ci/gameterm-scene-doctor.sh" \
    --scene "${fixture_root}/default.json" \
    --sprites "${fixture_root}/sprites.json" \
    --strict >/tmp/gameterm-scene-smoke-doctor.out

  echo "Bundled Scene Mode asset check succeeded."
}

if [[ "${check_assets}" -eq 1 ]]; then
  check_bundled_assets
  if [[ "${launch}" -eq 0 ]]; then
    exit 0
  fi
fi

if [[ -n "${submit_mux_patch}" && "${launch}" -eq 0 ]]; then
  echo "--submit-mux-patch requires --launch" >&2
  exit 2
fi

if ! ffmpeg_bin="$(resolve_ffmpeg)"; then
  cat >&2 <<'EOF'
ffmpeg was not found.

Install it with:
  brew install ffmpeg

Then rerun this script.
EOF
  exit 1
fi

echo "Using ffmpeg: ${ffmpeg_bin}"
"${ffmpeg_bin}" -version | sed -n '1,3p'
echo

echo "AVFoundation devices:"
set +e
"${ffmpeg_bin}" -hide_banner -f avfoundation -list_devices true -i "" 2>&1
list_rc=$?
set -e
echo
if [[ ${list_rc} -ne 0 ]]; then
  echo "Device listing returned ${list_rc}; this can still happen after listing devices." >&2
fi

if [[ "${launch}" -eq 1 ]]; then
  gui_bin="${repo_root}/target/debug/gameterm-gui"
  if [[ ! -x "${gui_bin}" ]]; then
    cat >&2 <<EOF
${gui_bin} does not exist.

Build it first:
  cargo build -p gameterm-gui
EOF
    exit 1
  fi

  tmp_home="$(mktemp -d /tmp/gameterm-scene-smoke.XXXXXX)"
  install_scene_fixture "${tmp_home}/gameterm/scenes"
  if [[ "${patch_inbox}" == "auto" ]]; then
    patch_inbox="${tmp_home}/gameterm/scenes/patch-inbox.json"
  fi

  echo "Launching GameTerm with fixture ${fixture}"
  echo "Temporary XDG_CONFIG_HOME=${tmp_home}"
  if [[ -n "${patch_inbox}" ]]; then
    echo "Patch inbox: ${patch_inbox}"
    XDG_CONFIG_HOME="${tmp_home}" \
      GAMETERM_SCENE_PATCH_FILE="${patch_inbox}" \
      "${gui_bin}" start --always-new-process --cwd "${repo_root}" &
  else
    XDG_CONFIG_HOME="${tmp_home}" "${gui_bin}" start --always-new-process \
      --cwd "${repo_root}" &
  fi
  gui_pid=$!
  echo "GameTerm pid: ${gui_pid}"
  echo "Press Ctrl+Shift+G in the GameTerm window to open Scene Mode."
  if [[ "${fixture}" == "run-command-targets" ]]; then
    echo "RunCommand audit: press Enter for tab, Next then Enter for split_right, Next then Enter for split_down."
  fi
  if [[ -n "${patch_inbox}" ]]; then
    echo "Patch audit: after opening Scene Mode, run:"
    echo "  ci/gameterm-scene-patch.sh write-inbox --inbox '${patch_inbox}' --patch ci/fixtures/gameterm-scene/patch-status.json"
  fi
  if [[ -n "${submit_mux_patch}" ]]; then
    echo "Mux patch audit: open Scene Mode before the wait expires; the script will submit:"
    echo "  ${submit_mux_patch}"
  fi
  echo "Waiting ${wait_before_capture}s before capture..."
  sleep "${wait_before_capture}"

  if [[ -n "${submit_mux_patch}" ]]; then
    submit_args=(submit-mux --patch "${submit_mux_patch}")
    if [[ -n "${submit_target_pane_id}" ]]; then
      submit_args+=(--target-pane-id "${submit_target_pane_id}")
    fi
    "${repo_root}/ci/gameterm-scene-patch.sh" "${submit_args[@]}"
    sleep 1
  fi
fi

rm -f "${output}" "${log_file}"
echo "Capturing one frame from avfoundation device ${device}"
echo "Output: ${output}"

"${ffmpeg_bin}" -hide_banner -y \
  -f avfoundation \
  -pixel_format bgr0 \
  -framerate 1 \
  -i "${device}" \
  -frames:v 1 \
  "${output}" >"${log_file}" 2>&1 &
capture_pid=$!

sleep "${capture_timeout}"
if kill -0 "${capture_pid}" 2>/dev/null; then
  kill "${capture_pid}" 2>/dev/null || true
  sleep 1
  cat "${log_file}" >&2 || true
  cat >&2 <<'EOF'

ffmpeg did not produce a frame before the timeout.

Most likely causes on macOS:
  - Screen Recording permission is not granted to the terminal/host app that
    runs this script.
  - The process is running outside a fully interactive GUI login session.

Fix:
  1. Open System Settings -> Privacy & Security -> Screen Recording.
  2. Enable the terminal/host app that runs this script.
  3. Fully quit and reopen that app.
  4. Rerun this script.
EOF
  exit 3
fi

wait "${capture_pid}" || {
  rc=$?
  cat "${log_file}" >&2 || true
  echo "ffmpeg capture failed with exit code ${rc}" >&2
  exit "${rc}"
}

if [[ ! -s "${output}" ]]; then
  cat "${log_file}" >&2 || true
  echo "ffmpeg exited but did not produce a non-empty output file." >&2
  exit 4
fi

actual_bytes="$(wc -c <"${output}" | tr -d ' ')"
if [[ "${actual_bytes}" -lt "${min_bytes}" ]]; then
  cat "${log_file}" >&2 || true
  echo "capture output is too small: ${actual_bytes} bytes, expected at least ${min_bytes}" >&2
  exit 5
fi

file "${output}"
ls -lh "${output}"
echo "Smoke capture succeeded."
