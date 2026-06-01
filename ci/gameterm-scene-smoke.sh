#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-smoke.sh [OPTIONS]

Checks whether macOS ffmpeg screen capture works for GameTerm Scene Mode smoke
testing. By default it attempts one still-frame capture from "0:none".

Options:
  --list-scenarios        List named smoke scenarios and exit.
  --describe-scenario NAME
                           Print one scenario's fixture, setup, and manual
                           checks, then exit.
  --scenario NAME         Apply defaults for a named smoke scenario.
  --launch                 Launch target/debug/gameterm-gui with the renderer
                           row fixture in a temporary XDG_CONFIG_HOME.
  --check-assets           Check bundled Scene Mode PNG assets and fixture
                           sprite manifests without launching or capturing.
  --fixture NAME           Fixture to install when --launch is used: basic,
                           navigate, invalid, sprites, missing-sprite,
                           run-command-targets, layered-mode, vertical-slice,
                           workspace-agent, workspace-discovery, authoring-loop,
                           or renderer-rows. Default:
                           renderer-rows.
  --patch-inbox PATH       Set GAMETERM_SCENE_PATCH_FILE to PATH when
                           launching. Use "auto" to create a temporary inbox.
  --submit-mux-patch PATH  After --launch wait, submit PATCH through
                           gameterm cli scene-patch before capture.
  --submit-target-pane-id ID
                           Target pane id for --submit-mux-patch. If omitted,
                           the smoke harness discovers the GameTerm Scene pane.
  --key-sequence LIST       Comma-separated keys to send before capture after
                           launch setup. Supported keys: enter, tab, escape,
                           space, up, down, left, right, h, j, k, l, q, r,
                           and delay:N.
  --wait-before-capture N  Seconds to wait after launch before capture.
                           Use this time to press Ctrl+Shift+G. Default: 10.
  --no-auto-open-scene     Do not use macOS automation to foreground GameTerm
                           and press Ctrl+Shift+G after launch.
  --allow-background-capture
                           Warn instead of failing when the launched GameTerm
                           process is not frontmost before capture.
  --post-action-wait N     Seconds to wait after scripted patches or key
                           sequences before capture. Default: 1.
  --focus-timeout N        Seconds to wait for the launched GUI process to
                           become visible to macOS automation. Default: 10.
  --device DEVICE          AVFoundation device string. Default: 0:none.
  --list-devices           Print AVFoundation devices before capture.
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
scenario=""
describe_scenario=""
list_scenarios=0
wait_before_capture=10
auto_open_scene=1
require_frontmost=1
post_action_wait=1
focus_timeout=10
ffmpeg_bin="${FFMPEG:-}"
list_devices=0
gui_pid=""
tmp_home=""
gui_class=""
patch_inbox=""
submit_mux_patch=""
submit_target_pane_id=""
key_sequence=""
log_file="/tmp/gameterm-scene-smoke-ffmpeg.log"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --list-scenarios)
      list_scenarios=1
      shift
      ;;
    --describe-scenario)
      describe_scenario="$2"
      shift 2
      ;;
    --scenario)
      scenario="$2"
      shift 2
      ;;
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
    --key-sequence)
      key_sequence="$2"
      shift 2
      ;;
    --wait-before-capture)
      wait_before_capture="$2"
      shift 2
      ;;
    --no-auto-open-scene)
      auto_open_scene=0
      shift
      ;;
    --allow-background-capture)
      require_frontmost=0
      shift
      ;;
    --post-action-wait)
      post_action_wait="$2"
      shift 2
      ;;
    --focus-timeout)
      focus_timeout="$2"
      shift 2
      ;;
    --device)
      device="$2"
      shift 2
      ;;
    --list-devices)
      list_devices=1
      shift
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

list_smoke_scenarios() {
  cat <<'EOF'
renderer-rows
guarded-input
run-command-targets
overlay-cleanup
vertical-slice
workspace-agent
workspace-discovery
live-mux-discovery
agent-lifecycle
authoring-loop
patch-inbox
mux-patch
process-state
EOF
}

describe_smoke_scenario() {
  case "$1" in
    renderer-rows)
      cat <<'EOF'
Scenario: renderer-rows
Fixture: renderer-rows
Setup: launch Scene Mode renderer row fixture.
Checks: rows remain visually aligned; sprite/entity layers do not shift.
Expected status: Ready.
EOF
      ;;
    guarded-input)
      cat <<'EOF'
Scenario: guarded-input
Fixture: layered-mode
Setup: launch layered state fixture.
Checks: automated input exercises a layer-owned update and guarded transition without closing Scene Mode.
Expected status: Layer story transitioned: dialogue -> choice.
EOF
      ;;
    run-command-targets)
      cat <<'EOF'
Scenario: run-command-targets
Fixture: run-command-targets
Setup: launch RunCommand target fixture.
Checks: automated input activates tab, split_right, and split_down RunCommand choices.
Expected status: RunCommand opened split_down pane.
EOF
      ;;
    overlay-cleanup)
      cat <<'EOF'
Scenario: overlay-cleanup
Fixture: basic
Setup: launch Scene Mode and send Escape before capture.
Checks: overlay cleanup returns to the underlying terminal without crashing the GUI.
Expected status: Scene Mode overlay closed.
EOF
      ;;
    vertical-slice)
      cat <<'EOF'
Scenario: vertical-slice
Fixture: vertical-slice
Setup: launch playable vertical slice fixture.
Checks: automated input accepts the brief, prepares the launch kit, completes the scene loop, and keeps Scene Mode open.
Expected status: Dialogue advanced: Guide.
EOF
      ;;
    workspace-agent)
      cat <<'EOF'
Scenario: workspace-agent
Fixture: workspace-agent
Setup: launch Agent/Workspace fixture with auto patch inbox, then emit real process and agent lifecycle patches.
Checks: Scene Mode renders workspace, project, task, agent, process, and file entities while external helpers drive lifecycle state.
Expected status: Agent complete: Workspace slice ready.
EOF
      ;;
    workspace-discovery)
      cat <<'EOF'
Scenario: workspace-discovery
Fixture: generated workspace scene
Setup: generate a Scene Mode scene from the current repository with ci/gameterm-scene-workspace.sh, then launch it.
Checks: Scene Mode renders generated workspace, project, task, process, and file entities from cwd/git state.
Expected status: Discovered workspace scene is visible.
EOF
      ;;
    live-mux-discovery)
      cat <<'EOF'
Scenario: live-mux-discovery
Fixture: generated live mux workspace scene
Setup: generate a Scene Mode scene from the active mux pane with ci/gameterm-scene-mux-context.sh, then launch it.
Checks: Scene Mode renders generated workspace, pane, and process entities from active mux context, or falls back to cwd discovery when mux context is unavailable.
Expected status: Discovered live mux workspace scene is visible.
EOF
      ;;
    agent-lifecycle)
      cat <<'EOF'
Scenario: agent-lifecycle
Fixture: basic
Setup: launch with auto patch inbox and emit planning, blocked, and complete agent phases.
Checks: Scene Mode receives structured agent lifecycle patches and renders the final completed agent state.
Expected status: Agent complete: Finished visual slice.
EOF
      ;;
    authoring-loop)
      cat <<'EOF'
Scenario: authoring-loop
Fixture: authoring-loop
Setup: launch story-state authoring fixture.
Checks: automated input saves story state, mutates draft state, reloads the saved state, and keeps Scene Mode open.
Expected status: Story state imported: /tmp/gameterm-scene-authoring-loop.story.json.
EOF
      ;;
    patch-inbox)
      cat <<'EOF'
Scenario: patch-inbox
Fixture: basic
Setup: launch with GAMETERM_SCENE_PATCH_FILE auto-inbox.
Checks: write-inbox patch updates visible entity state and keeps Scene Mode open.
Expected status: Fixture patch applied.
EOF
      ;;
    mux-patch)
      cat <<'EOF'
Scenario: mux-patch
Fixture: basic
Setup: launch, open Scene Mode, submit ci/fixtures/gameterm-scene/patch-status.json through mux.
Checks: mux patch updates visible entity state and reports patch source.
Expected status: Fixture patch applied.
EOF
      ;;
    process-state)
      cat <<'EOF'
Scenario: process-state
Fixture: basic
Setup: launch with auto patch inbox, then run the process helper before capture.
Checks: entity transitions running -> succeeded/failed and Tile Debugger shows typed process state.
Expected status: Process succeeded: true.
EOF
      ;;
    *)
      echo "unknown smoke scenario: $1" >&2
      list_smoke_scenarios >&2
      exit 2
      ;;
  esac
}

apply_smoke_scenario_defaults() {
  case "${scenario}" in
    "")
      ;;
    renderer-rows)
      fixture="renderer-rows"
      ;;
    guarded-input)
      fixture="layered-mode"
      if [[ -z "${key_sequence}" ]]; then
        key_sequence="space,enter"
      fi
      ;;
    run-command-targets)
      fixture="run-command-targets"
      if [[ -z "${key_sequence}" ]]; then
        key_sequence="enter,j,enter,j,enter"
      fi
      ;;
    overlay-cleanup)
      fixture="basic"
      if [[ -z "${key_sequence}" ]]; then
        key_sequence="escape"
      fi
      ;;
    vertical-slice)
      fixture="vertical-slice"
      if [[ -z "${key_sequence}" ]]; then
        key_sequence="enter,j,enter,j,enter,j,enter"
      fi
      ;;
    workspace-agent)
      fixture="workspace-agent"
      if [[ -z "${patch_inbox}" ]]; then
        patch_inbox="auto"
      fi
      ;;
    workspace-discovery)
      fixture="workspace-discovery"
      ;;
    live-mux-discovery)
      fixture="live-mux-discovery"
      ;;
    agent-lifecycle)
      fixture="basic"
      if [[ -z "${patch_inbox}" ]]; then
        patch_inbox="auto"
      fi
      ;;
    authoring-loop)
      fixture="authoring-loop"
      if [[ -z "${key_sequence}" ]]; then
        key_sequence="enter,j,enter,j,enter"
      fi
      ;;
    patch-inbox)
      fixture="basic"
      if [[ -z "${patch_inbox}" ]]; then
        patch_inbox="auto"
      fi
      ;;
    mux-patch)
      fixture="basic"
      if [[ -z "${submit_mux_patch}" ]]; then
        submit_mux_patch="${fixture_root}/patch-status.json"
      fi
      ;;
    process-state)
      fixture="basic"
      if [[ -z "${patch_inbox}" ]]; then
        patch_inbox="auto"
      fi
      ;;
    *)
      echo "unknown smoke scenario: ${scenario}" >&2
      list_smoke_scenarios >&2
      exit 2
      ;;
  esac
}

if [[ "${list_scenarios}" -eq 1 ]]; then
  list_smoke_scenarios
  exit 0
fi

if [[ -n "${describe_scenario}" ]]; then
  describe_smoke_scenario "${describe_scenario}"
  exit 0
fi

apply_smoke_scenario_defaults

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
      install_sprite_manifest "${scene_dir}/sprites.json"
      ;;
    missing-sprite)
      cp "${fixture_root}/default.json" "${scene_dir}/default.json"
      cp "${fixture_root}/sprites-missing.json" "${scene_dir}/sprites.json"
      ;;
    run-command-targets)
      cp "${fixture_root}/run-command-targets.json" "${scene_dir}/default.json"
      install_sprite_manifest "${scene_dir}/sprites.json"
      ;;
    layered-mode)
      cp "${fixture_root}/layered-mode.json" "${scene_dir}/default.json"
      install_sprite_manifest "${scene_dir}/sprites.json"
      ;;
    vertical-slice)
      cp "${fixture_root}/vertical-slice.json" "${scene_dir}/default.json"
      install_sprite_manifest "${scene_dir}/sprites.json"
      ;;
    workspace-agent)
      cp "${fixture_root}/workspace-agent.json" "${scene_dir}/default.json"
      install_sprite_manifest "${scene_dir}/sprites.json"
      ;;
    workspace-discovery)
      "${repo_root}/ci/gameterm-scene-workspace.sh" \
        discover \
        --cwd "${repo_root}" \
        --scene-output "${scene_dir}/default.json" \
        --force >/dev/null
      install_sprite_manifest "${scene_dir}/sprites.json"
      ;;
    live-mux-discovery)
      "${repo_root}/ci/gameterm-scene-mux-context.sh" \
        discover \
        --allow-missing \
        --scene-output "${scene_dir}/default.json" \
        --force >/dev/null
      install_sprite_manifest "${scene_dir}/sprites.json"
      ;;
    authoring-loop)
      cp "${fixture_root}/authoring-loop.json" "${scene_dir}/default.json"
      install_sprite_manifest "${scene_dir}/sprites.json"
      ;;
    *)
      echo "unknown fixture: ${fixture}" >&2
      usage >&2
      exit 2
      ;;
  esac
}

assert_gui_binary_current() {
  local gui_bin="$1"
  local stale_source

  stale_source="$(
    find \
      "${repo_root}/gameterm-gui" \
      "${repo_root}/gameterm-visual" \
      -type f \( -name '*.rs' -o -name Cargo.toml \) \
      -newer "${gui_bin}" \
      -print \
      -quit
  )"
  if [[ -n "${stale_source}" ]]; then
    cat >&2 <<EOF
${gui_bin} is older than Scene Mode source:
  ${stale_source}

Build the GUI before live smoke:
  cargo build -p gameterm-gui
EOF
    exit 1
  fi
}

install_sprite_manifest() {
  local target="$1"
  jq --arg asset_root "${repo_root}/assets/gameterm-scene" '
    .sprites |= map(.path = ($asset_root + "/" + (.path | split("/") | last)))
  ' "${fixture_root}/sprites.json" >"${target}"
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

frontmost_process() {
  osascript <<'EOF' 2>/dev/null
tell application "System Events"
  set frontProcess to first application process whose frontmost is true
  set frontName to name of frontProcess
  set frontPid to unix id of frontProcess
  return frontName & " " & frontPid
end tell
EOF
}

foreground_gui_and_open_scene() {
  local pid="$1"
  local timeout="$2"

  if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "Automatic Scene Mode opening is only supported on macOS; open Scene Mode manually." >&2
    return 0
  fi
  if ! command -v osascript >/dev/null 2>&1; then
    echo "osascript is unavailable; open Scene Mode manually." >&2
    return 0
  fi

  echo "Foregrounding GameTerm pid ${pid} and opening Scene Mode..."
  if ! osascript - "${pid}" "${timeout}" <<'EOF'
on run argv
  set targetPid to (item 1 of argv) as integer
  set timeoutSeconds to (item 2 of argv) as integer
  set deadline to (current date) + timeoutSeconds

  tell application "System Events"
    repeat while (current date) is less than deadline
      if exists (first application process whose unix id is targetPid) then
        set targetProcess to first application process whose unix id is targetPid
        set frontmost of targetProcess to true
        delay 0.5
        keystroke "g" using {control down, shift down}
        delay 0.5
        return
      end if
      delay 0.2
    end repeat
  end tell

  error "GameTerm GUI process was not visible to System Events before timeout"
end run
EOF
  then
    cat >&2 <<'EOF'
Failed to foreground GameTerm or send Ctrl+Shift+G through macOS automation.

Most likely causes:
  - Accessibility permission is not granted to the terminal/host app running
    this script.
  - GameTerm did not create a GUI application process before the focus timeout.

Fix:
  1. Open System Settings -> Privacy & Security -> Accessibility.
  2. Enable the terminal/host app that runs this script.
  3. Fully quit and reopen that app.
  4. Rerun this script, or use --no-auto-open-scene and open Scene Mode manually.
EOF
    exit 7
  fi
}

send_key_sequence() {
  local pid="$1"
  local sequence="$2"

  if [[ -z "${sequence}" ]]; then
    return 0
  fi
  if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "Automatic key sequences are only supported on macOS; skipping ${sequence}." >&2
    return 0
  fi
  if ! command -v osascript >/dev/null 2>&1; then
    echo "osascript is unavailable; skipping key sequence ${sequence}." >&2
    return 0
  fi

  echo "Sending Scene Mode key sequence: ${sequence}"
  if ! osascript - "${pid}" "${sequence}" <<'EOF'
on trimText(valueText)
  set oldDelimiters to AppleScript's text item delimiters
  set AppleScript's text item delimiters to ""
  set charactersList to characters of valueText
  repeat while (count of charactersList) > 0 and item 1 of charactersList is in {" ", tab, return, linefeed}
    if (count of charactersList) is 1 then
      set charactersList to {}
    else
      set charactersList to items 2 thru -1 of charactersList
    end if
  end repeat
  repeat while (count of charactersList) > 0 and item -1 of charactersList is in {" ", tab, return, linefeed}
    if (count of charactersList) is 1 then
      set charactersList to {}
    else
      set charactersList to items 1 thru -2 of charactersList
    end if
  end repeat
  set trimmedText to charactersList as text
  set AppleScript's text item delimiters to oldDelimiters
  return trimmedText
end trimText

on sendNamedKey(keyName)
  tell application "System Events"
    if keyName is "enter" or keyName is "return" then
      key code 36
    else if keyName is "tab" then
      key code 48
    else if keyName is "escape" or keyName is "esc" then
      key code 53
    else if keyName is "space" then
      key code 49
    else if keyName is "up" then
      key code 126
    else if keyName is "down" then
      key code 125
    else if keyName is "left" then
      key code 123
    else if keyName is "right" then
      key code 124
    else if keyName is "h" or keyName is "j" or keyName is "k" or keyName is "l" or keyName is "q" or keyName is "r" then
      keystroke keyName
    else
      error "unsupported Scene Mode smoke key: " & keyName
    end if
  end tell
end sendNamedKey

on run argv
  set targetPid to (item 1 of argv) as integer
  set keySequence to item 2 of argv
  set oldDelimiters to AppleScript's text item delimiters
  set AppleScript's text item delimiters to ","
  set keyItems to text items of keySequence
  set AppleScript's text item delimiters to oldDelimiters

  tell application "System Events"
    if not (exists (first application process whose unix id is targetPid)) then
      error "GameTerm GUI process is not visible to System Events"
    end if
    set targetProcess to first application process whose unix id is targetPid
    set frontmost of targetProcess to true
  end tell
  delay 0.3

  repeat with rawKey in keyItems
    set keyName to trimText(rawKey as text)
    if keyName starts with "delay:" then
      set delaySeconds to text 7 thru -1 of keyName as real
      delay delaySeconds
    else if keyName is not "" then
      sendNamedKey(keyName)
      delay 0.35
    end if
  end repeat
end run
EOF
  then
    cat >&2 <<'EOF'
Failed to send the Scene Mode key sequence through macOS automation.

Check Accessibility permission for the terminal/host app running this script,
or rerun without --key-sequence and drive the interaction manually.
EOF
    exit 8
  fi
}

assert_gui_foreground() {
  local pid="$1"
  local front

  front="$(frontmost_process || true)"
  if [[ -z "${front}" ]]; then
    if [[ "${require_frontmost}" -eq 1 ]]; then
      echo "Could not determine frontmost macOS process before capture." >&2
      exit 9
    fi
    echo "Could not determine frontmost macOS process before capture." >&2
    return
  fi
  echo "Frontmost process before capture: ${front}"
  if [[ "${front}" != *" ${pid}" ]]; then
    if [[ "${require_frontmost}" -eq 1 ]]; then
      cat >&2 <<EOF
GameTerm pid ${pid} is not the frontmost macOS process before capture.
Frontmost process: ${front}

This would likely capture another app instead of Scene Mode. Rerun after
closing/defocusing the conflicting app, or pass --allow-background-capture if
you intentionally want a best-effort capture.
EOF
      exit 9
    fi
    cat >&2 <<EOF
Warning: GameTerm pid ${pid} is not the frontmost macOS process before capture.
The capture may show another app instead of Scene Mode.
EOF
  fi
}

submit_mux_patch_with_retry() {
  local patch="$1"
  local timeout="$2"
  local deadline=$((SECONDS + timeout))
  local rc=1
  local submit_args
  submit_args=(submit-mux --patch "${patch}")
  if [[ -n "${gui_class}" ]]; then
    submit_args+=(--class "${gui_class}")
  fi
  if [[ -n "${submit_target_pane_id}" ]]; then
    submit_args+=(--target-pane-id "${submit_target_pane_id}")
  fi

  while ((SECONDS <= deadline)); do
    set +e
    if [[ -n "${tmp_home}" ]]; then
      env -u GAMETERM_UNIX_SOCKET XDG_CONFIG_HOME="${tmp_home}" \
        "${repo_root}/ci/gameterm-scene-patch.sh" "${submit_args[@]}" \
        >/tmp/gameterm-scene-smoke-submit.out \
        2>/tmp/gameterm-scene-smoke-submit.err
    else
      env -u GAMETERM_UNIX_SOCKET \
        "${repo_root}/ci/gameterm-scene-patch.sh" "${submit_args[@]}" \
        >/tmp/gameterm-scene-smoke-submit.out \
        2>/tmp/gameterm-scene-smoke-submit.err
    fi
    rc=$?
    set -e
    if [[ "${rc}" -eq 0 ]]; then
      cat /tmp/gameterm-scene-smoke-submit.out
      return 0
    fi
    if ! grep -q "no active GameTerm Scene Mode overlay" \
      /tmp/gameterm-scene-smoke-submit.err; then
      cat /tmp/gameterm-scene-smoke-submit.out >&2 || true
      cat /tmp/gameterm-scene-smoke-submit.err >&2 || true
      return "${rc}"
    fi
    sleep 1
  done

  cat /tmp/gameterm-scene-smoke-submit.out >&2 || true
  cat /tmp/gameterm-scene-smoke-submit.err >&2 || true
  echo "Timed out waiting for active GameTerm Scene Mode overlay before mux patch submission." >&2
  return "${rc}"
}

discover_scene_overlay_pane_id() {
  local timeout="$1"
  local deadline=$((SECONDS + timeout))
  local list_out="/tmp/gameterm-scene-smoke-list.out"
  local list_err="/tmp/gameterm-scene-smoke-list.err"
  local pane_id=""
  local rc=1

  while ((SECONDS <= deadline)); do
    set +e
    if [[ -n "${tmp_home}" ]]; then
      env -u GAMETERM_UNIX_SOCKET XDG_CONFIG_HOME="${tmp_home}" \
        cargo run -q -p gameterm -- cli --class "${gui_class}" list --format json \
        >"${list_out}" \
        2>"${list_err}"
    else
      env -u GAMETERM_UNIX_SOCKET \
        cargo run -q -p gameterm -- cli list --format json \
        >"${list_out}" \
        2>"${list_err}"
    fi
    rc=$?
    set -e

    if [[ "${rc}" -eq 0 ]]; then
      pane_id="$(
        jq -r '
          (map(select(.title == "GameTerm Scene"))
            | sort_by(if .is_active then 0 else 1 end)
            | .[0].pane_id)
          // (map(select(.is_active == true)) | .[0].pane_id)
          // empty
        ' "${list_out}"
      )"
      if [[ -n "${pane_id}" ]]; then
        printf '%s\n' "${pane_id}"
        return 0
      fi
    fi

    sleep 1
  done

  cat "${list_err}" >&2 || true
  cat >&2 <<'EOF'
Timed out waiting for a GameTerm Scene target pane in `gameterm cli list`.

The mux smoke path needs an explicit target pane because the active overlay
fallback is process-local. The smoke harness launches a uniquely classed GUI
and connects the CLI to that published class.
EOF
  return 1
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

if [[ "${list_devices}" -eq 1 ]]; then
  echo "AVFoundation devices:"
  set +e
  "${ffmpeg_bin}" -hide_banner -f avfoundation -list_devices true -i "" 2>&1
  list_rc=$?
  set -e
  echo
  if [[ ${list_rc} -ne 0 ]]; then
    echo "Device listing returned ${list_rc}; this can still happen after listing devices." >&2
  fi
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
  assert_gui_binary_current "${gui_bin}"

  tmp_home="$(mktemp -d /tmp/gameterm-scene-smoke.XXXXXX)"
  gui_class="org.gameterm.scene-smoke.$$"
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
      "${gui_bin}" start --class "${gui_class}" --cwd "${repo_root}" &
  else
    XDG_CONFIG_HOME="${tmp_home}" "${gui_bin}" start --class "${gui_class}" \
      --cwd "${repo_root}" &
  fi
  gui_pid=$!
  echo "GameTerm pid: ${gui_pid}"
  echo "GameTerm class: ${gui_class}"
  if [[ "${auto_open_scene}" -eq 1 ]]; then
    foreground_gui_and_open_scene "${gui_pid}" "${focus_timeout}"
  else
    echo "Press Ctrl+Shift+G in the GameTerm window to open Scene Mode."
  fi
  if [[ "${fixture}" == "run-command-targets" ]]; then
    echo "RunCommand audit: activate tab, split_right, and split_down choices."
  fi
  if [[ "${scenario}" == "guarded-input" ]]; then
    echo "Guarded input audit: exercise layer-owned update and guarded transition inputs."
  fi
  if [[ "${scenario}" == "overlay-cleanup" ]]; then
    echo "Overlay cleanup audit: close Scene Mode before capture."
  fi
  if [[ "${scenario}" == "vertical-slice" ]]; then
    echo "Vertical slice audit: complete the playable brief, launch kit, loop, and ending path."
  fi
  if [[ "${scenario}" == "workspace-agent" ]]; then
    echo "Agent/Workspace audit: emit process and agent lifecycle patches into the workspace fixture."
  fi
  if [[ "${scenario}" == "workspace-discovery" ]]; then
    echo "Workspace discovery audit: launch a scene generated from ${repo_root}."
  fi
  if [[ "${scenario}" == "live-mux-discovery" ]]; then
    echo "Live mux discovery audit: launch a scene generated from active mux context."
    "${repo_root}/ci/gameterm-scene-mux-context.sh" collect --allow-missing || true
  fi
  if [[ -n "${patch_inbox}" ]]; then
    echo "Patch audit: inbox transport is enabled at ${patch_inbox}"
  fi
  if [[ "${scenario}" == "process-state" ]]; then
    echo "Process-state audit: the script will run a true command through ci/gameterm-scene-process.sh before capture."
  fi
  if [[ "${scenario}" == "agent-lifecycle" ]]; then
    echo "Agent lifecycle audit: the script will emit planning, blocked, and complete patches before capture."
  fi
  if [[ "${scenario}" == "authoring-loop" ]]; then
    echo "Authoring loop audit: save story state, mutate draft state, then reload the saved state."
  fi
  if [[ -n "${submit_mux_patch}" ]]; then
    echo "Mux patch audit: open Scene Mode before the wait expires; the script will submit:"
    echo "  ${submit_mux_patch}"
  fi
  echo "Waiting ${wait_before_capture}s before capture..."
  sleep "${wait_before_capture}"

  if [[ -n "${submit_mux_patch}" ]]; then
    if [[ -z "${submit_target_pane_id}" ]]; then
      submit_target_pane_id="$(discover_scene_overlay_pane_id "${focus_timeout}")"
      echo "Discovered Scene Mode target pane: ${submit_target_pane_id}"
    fi
    submit_mux_patch_with_retry "${submit_mux_patch}" "${focus_timeout}"
    sleep "${post_action_wait}"
  fi
  if [[ "${scenario}" == "patch-inbox" ]]; then
    "${repo_root}/ci/gameterm-scene-patch.sh" \
      write-inbox \
      --inbox "${patch_inbox}" \
      --patch "${fixture_root}/patch-status.json" >/dev/null
    echo "Wrote patch-inbox smoke patch: ${fixture_root}/patch-status.json"
    sleep "${post_action_wait}"
  fi
  if [[ "${scenario}" == "process-state" ]]; then
    process_patch="${tmp_home}/gameterm/scenes/process-state.json"
    "${repo_root}/ci/gameterm-scene-process.sh" \
      --entity-id project-harness \
      --patch "${process_patch}" \
      --inbox "${patch_inbox}" \
      --select \
      -- \
      true >/dev/null
    echo "Wrote process-state smoke patch: ${process_patch}"
    sleep "${post_action_wait}"
  fi
  if [[ "${scenario}" == "workspace-agent" ]]; then
    workspace_patch_dir="${tmp_home}/gameterm/scenes/workspace-agent"
    mkdir -p "${workspace_patch_dir}"
    "${repo_root}/ci/gameterm-scene-process.sh" \
      --entity-id scene-verify-process \
      --patch "${workspace_patch_dir}/process.json" \
      --inbox "${patch_inbox}" \
      --select \
      -- \
      true >/dev/null
    sleep "${post_action_wait}"
    "${repo_root}/ci/gameterm-scene-agent.sh" \
      status \
      --entity-id scene-agent \
      --phase planning \
      --command "build workspace slice" \
      --message "Planning workspace slice" \
      --patch "${workspace_patch_dir}/planning.json" \
      --inbox "${patch_inbox}" \
      --select >/dev/null
    sleep "${post_action_wait}"
    "${repo_root}/ci/gameterm-scene-agent.sh" \
      status \
      --entity-id scene-agent \
      --phase running \
      --command "build workspace slice" \
      --message "Running workspace slice" \
      --patch "${workspace_patch_dir}/running.json" \
      --inbox "${patch_inbox}" \
      --select >/dev/null
    sleep "${post_action_wait}"
    "${repo_root}/ci/gameterm-scene-agent.sh" \
      status \
      --entity-id scene-agent \
      --phase blocked \
      --command "build workspace slice" \
      --message "Review needed before verification" \
      --patch "${workspace_patch_dir}/blocked.json" \
      --inbox "${patch_inbox}" \
      --select >/dev/null
    sleep "${post_action_wait}"
    "${repo_root}/ci/gameterm-scene-agent.sh" \
      status \
      --entity-id scene-agent \
      --phase complete \
      --command "build workspace slice" \
      --message "Workspace slice ready" \
      --patch "${workspace_patch_dir}/complete.json" \
      --inbox "${patch_inbox}" \
      --select >/dev/null
    echo "Wrote workspace-agent smoke patches: ${workspace_patch_dir}"
    sleep "${post_action_wait}"
  fi
  if [[ "${scenario}" == "agent-lifecycle" ]]; then
    agent_patch_dir="${tmp_home}/gameterm/scenes/agent-lifecycle"
    mkdir -p "${agent_patch_dir}"
    "${repo_root}/ci/gameterm-scene-agent.sh" \
      status \
      --entity-id project-harness \
      --phase planning \
      --command "ship visual slice" \
      --message "Planning visual slice" \
      --patch "${agent_patch_dir}/planning.json" \
      --inbox "${patch_inbox}" \
      --select >/dev/null
    sleep "${post_action_wait}"
    "${repo_root}/ci/gameterm-scene-agent.sh" \
      status \
      --entity-id project-harness \
      --phase blocked \
      --command "ship visual slice" \
      --message "Waiting on approval" \
      --patch "${agent_patch_dir}/blocked.json" \
      --inbox "${patch_inbox}" \
      --select >/dev/null
    sleep "${post_action_wait}"
    "${repo_root}/ci/gameterm-scene-agent.sh" \
      status \
      --entity-id project-harness \
      --phase complete \
      --command "ship visual slice" \
      --message "Finished visual slice" \
      --patch "${agent_patch_dir}/complete.json" \
      --inbox "${patch_inbox}" \
      --select >/dev/null
    echo "Wrote agent-lifecycle smoke patches: ${agent_patch_dir}"
    sleep "${post_action_wait}"
  fi
  if [[ -n "${key_sequence}" ]]; then
    send_key_sequence "${gui_pid}" "${key_sequence}"
    sleep "${post_action_wait}"
  fi
  assert_gui_foreground "${gui_pid}"
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
