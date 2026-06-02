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
                           vn-demo, or renderer-rows. Default:
                           renderer-rows.
  --vn-asset-source-root PATH
                           Local VN asset source root for the vn-demo scenario.
                           Defaults to the repo-safe fixture asset source.
  --allow-ai-assisted-vn-assets
                           Allow VN demo intake to include catalog sources
                           marked as AI-assisted.
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
  --open-active-pane-scene  When auto-opening Scene Mode, send the native
                           active-pane shortcut Ctrl+Alt+Shift+G instead of
                           the configured-scene shortcut Ctrl+Shift+G.
  --wait-before-capture N  Seconds to wait after launch before capture.
                           Use this time to press the relevant Scene shortcut.
                           Default: 10.
  --no-auto-open-scene     Do not use macOS automation to foreground GameTerm
                           and press the Scene shortcut after launch.
  --no-fullscreen-window   Do not resize the launched GameTerm window to fill
                           the visible desktop before capture.
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
fullscreen_window=1
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
scene_open_shortcut="configured"
vn_asset_source_root=""
allow_ai_assisted_vn_assets=0
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
    --vn-asset-source-root)
      vn_asset_source_root="$2"
      shift 2
      ;;
    --allow-ai-assisted-vn-assets)
      allow_ai_assisted_vn_assets=1
      shift
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
    --open-active-pane-scene)
      scene_open_shortcut="active-pane"
      shift
      ;;
    --wait-before-capture)
      wait_before_capture="$2"
      shift 2
      ;;
    --no-auto-open-scene)
      auto_open_scene=0
      shift
      ;;
    --no-fullscreen-window)
      fullscreen_window=0
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
active-pane-gui
vn-demo
vn-compose
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
    active-pane-gui)
      cat <<'EOF'
Scenario: active-pane-gui
Fixture: renderer-rows
Setup: launch GameTerm from the current build and use macOS automation to send Ctrl+Alt+Shift+G.
Checks: native active-pane Scene action opens a transient generated workspace scene without writing the configured scene file.
Expected status: generated active pane scene is visible; Esc/q closes it cleanly.
EOF
      ;;
    vn-demo)
      cat <<'EOF'
Scenario: vn-demo
Fixture: generated VN demo
Setup: generate the VN demo into a temporary Scene config, validate sprite files as real PNG images, then launch Scene Mode.
Checks: imported VN dialogue, choices, bindings, and copied VN sprite assets render through the normal Scene Mode path.
Expected status: VN Script Demo Import is visible.
EOF
      ;;
    vn-compose)
      cat <<'EOF'
Scenario: vn-compose
Fixture: generated VN demo
Setup: generate the VN demo, launch Scene Mode, type a prompt into the compose dock, submit it, and wait for the deterministic local compose backend.
Checks: compose input remains docked, backend output renders as Codex dialogue, and VN background/sprites remain visible.
Expected status: Compose succeeded.
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
    active-pane-gui)
      fixture="renderer-rows"
      scene_open_shortcut="active-pane"
      ;;
    vn-demo)
      fixture="vn-demo"
      if [[ -z "${key_sequence}" ]]; then
        key_sequence="enter,j,enter"
      fi
      ;;
    vn-compose)
      fixture="vn-demo"
      if [[ -z "${key_sequence}" ]]; then
        key_sequence="text:look at roadmap,enter,delay:1"
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

run_mux_context_helper() {
  local subcommand="$1"
  shift
  local args=("${subcommand}")
  if [[ -x "${repo_root}/target/debug/gameterm" ]]; then
    args+=(--gameterm-bin "${repo_root}/target/debug/gameterm")
  fi
  "${repo_root}/ci/gameterm-scene-mux-context.sh" "${args[@]}" "$@"
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
      run_mux_context_helper \
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
    vn-demo)
      if [[ -z "${vn_asset_source_root}" ]]; then
        vn_asset_source_root="${fixture_root}/vn-asset-source"
      fi
      vn_demo_args=(
        --output-dir "${scene_dir}"
        --asset-source-root "${vn_asset_source_root}"
        --strict-images
        --force
      )
      if [[ "${allow_ai_assisted_vn_assets}" -eq 1 ]]; then
        vn_demo_args+=(--allow-ai-assisted-assets)
      fi
      "${repo_root}/ci/gameterm-scene-vn-demo.sh" generate \
        "${vn_demo_args[@]}" >/dev/null
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
  local shortcut="$3"
  local resize_window="$4"
  local shortcut_label="Ctrl+Shift+G"

  if [[ "${shortcut}" == "active-pane" ]]; then
    shortcut_label="Ctrl+Alt+Shift+G"
  fi

  if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "Automatic Scene Mode opening is only supported on macOS; open Scene Mode manually." >&2
    return 0
  fi
  if ! command -v osascript >/dev/null 2>&1; then
    echo "osascript is unavailable; open Scene Mode manually." >&2
    return 0
  fi

  echo "Foregrounding GameTerm pid ${pid} and opening Scene Mode with ${shortcut_label}..."
  if ! osascript - "${pid}" "${timeout}" "${shortcut}" "${resize_window}" <<'EOF'
on visibleDesktopFrame()
  tell application "Finder"
    set desktopBounds to bounds of window of desktop
  end tell
  set leftEdge to item 1 of desktopBounds
  set topEdge to item 2 of desktopBounds
  set rightEdge to item 3 of desktopBounds
  set bottomEdge to item 4 of desktopBounds
  return {leftEdge, topEdge + 24, rightEdge - leftEdge, bottomEdge - topEdge - 24}
end visibleDesktopFrame

on run argv
  set targetPid to (item 1 of argv) as integer
  set timeoutSeconds to (item 2 of argv) as integer
  set shortcutName to item 3 of argv
  set resizeWindow to item 4 of argv
  set deadline to (current date) + timeoutSeconds

  tell application "System Events"
    repeat while (current date) is less than deadline
      if exists (first application process whose unix id is targetPid) then
        set targetProcess to first application process whose unix id is targetPid
        set frontmost of targetProcess to true
        delay 0.5
        if resizeWindow is "1" then
          set frame to my visibleDesktopFrame()
          set windowPosition to {item 1 of frame, item 2 of frame}
          set windowSize to {item 3 of frame, item 4 of frame}
          if exists window 1 of targetProcess then
            set position of window 1 of targetProcess to windowPosition
            set size of window 1 of targetProcess to windowSize
            delay 0.5
          end if
        end if
        if shortcutName is "active-pane" then
          keystroke "g" using {control down, option down, shift down}
        else
          keystroke "g" using {control down, shift down}
        end if
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
    cat >&2 <<EOF
Failed to foreground GameTerm or send ${shortcut_label} through macOS automation.

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

on sendText(valueText)
  tell application "System Events"
    keystroke valueText
  end tell
end sendText

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
    else if keyName starts with "text:" then
      set textValue to text 6 thru -1 of keyName
      sendText(textValue)
      delay 0.35
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
    foreground_gui_and_open_scene \
      "${gui_pid}" \
      "${focus_timeout}" \
      "${scene_open_shortcut}" \
      "${fullscreen_window}"
  else
    if [[ "${scene_open_shortcut}" == "active-pane" ]]; then
      echo "Press Ctrl+Alt+Shift+G in the GameTerm window to open active-pane Scene Mode."
    else
      echo "Press Ctrl+Shift+G in the GameTerm window to open Scene Mode."
    fi
    if [[ "${fullscreen_window}" -eq 1 ]]; then
      echo "Resize the GameTerm window to fill the screen before capture."
    fi
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
    run_mux_context_helper collect --allow-missing || true
  fi
  if [[ -n "${patch_inbox}" ]]; then
    echo "Patch audit: inbox transport is enabled at ${patch_inbox}"
  fi
  if [[ "${scenario}" == "process-state" ]]; then
    echo "Process-state audit: the script will run a true command through ci/gameterm-scene-process.sh before capture."
  fi
  if [[ "${scenario}" == "active-pane-gui" ]]; then
    echo "Active-pane GUI audit: open a transient generated workspace scene through the native shortcut."
  fi
  if [[ "${scenario}" == "agent-lifecycle" ]]; then
    echo "Agent lifecycle audit: the script will emit planning, blocked, and complete patches before capture."
  fi
  if [[ "${scenario}" == "authoring-loop" ]]; then
    echo "Authoring loop audit: save story state, mutate draft state, then reload the saved state."
  fi
  if [[ "${scenario}" == "vn-demo" || "${scenario}" == "vn-compose" ]]; then
    echo "VN demo audit: launch imported VN script with strict-validated local PNG assets."
  fi
  if [[ "${scenario}" == "vn-compose" ]]; then
    echo "VN compose audit: type a prompt and render the deterministic compose backend reply."
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
