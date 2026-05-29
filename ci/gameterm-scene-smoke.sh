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
  --wait-before-capture N  Seconds to wait after launch before capture.
                           Use this time to press Ctrl+Shift+G. Default: 10.
  --device DEVICE          AVFoundation device string. Default: 0:none.
  --output PATH            Capture output path. Default:
                           /tmp/gameterm-scene-smoke.png.
  --capture-timeout N      Seconds to wait for one frame before diagnosing a
                           hang. Default: 12.
  --ffmpeg PATH            ffmpeg binary. Defaults to PATH lookup, then common
                           Homebrew locations.
  -h, --help               Show this help.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
device="0:none"
output="/tmp/gameterm-scene-smoke.png"
capture_timeout=12
launch=0
wait_before_capture=10
ffmpeg_bin="${FFMPEG:-}"
gui_pid=""
tmp_home=""
log_file="/tmp/gameterm-scene-smoke-ffmpeg.log"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --launch)
      launch=1
      shift
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
  mkdir -p "${tmp_home}/gameterm/scenes"
  cp "${repo_root}/docs/examples/gameterm-scene-renderer-rows.json" \
    "${tmp_home}/gameterm/scenes/default.json"

  echo "Launching GameTerm with temporary XDG_CONFIG_HOME=${tmp_home}"
  XDG_CONFIG_HOME="${tmp_home}" "${gui_bin}" start --always-new-process \
    --cwd "${repo_root}" &
  gui_pid=$!
  echo "GameTerm pid: ${gui_pid}"
  echo "Press Ctrl+Shift+G in the GameTerm window to open Scene Mode."
  echo "Waiting ${wait_before_capture}s before capture..."
  sleep "${wait_before_capture}"
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

file "${output}"
ls -lh "${output}"
echo "Smoke capture succeeded."
