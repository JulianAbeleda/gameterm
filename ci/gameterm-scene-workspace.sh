#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-workspace.sh COMMAND [OPTIONS]

Discover local workspace state and write GameTerm Scene Mode output.

Commands:
  inspect                       Print discovered workspace summary.
  discover                      Generate a complete Scene Mode scene.
  dogfood                       Generate/install the daily dogfood workspace scene.
  patch                         Generate a patch for the workspace-agent scene.
  brief                         Generate a local task brief JSON file.

Common options:
  --cwd PATH                    Workspace directory. Default: current directory.
  --title TEXT                  Override generated scene title.
  --task TEXT                   Active task label/objective.
  --verify-argv JSON_ARRAY      Explicit verification argv.
  --open PATH                   Add an important file. May be repeated.
  --pane-id ID                  Active GameTerm pane id, when known.
  --mux-window-id ID            Active GameTerm mux window id, when known.
  --pane-cwd PATH               Active pane working directory, when known.
  --foreground-process-name TEXT
                                Active pane foreground process name.
  --foreground-process-path PATH
                                Active pane foreground process executable path.
  --pane-progress TEXT          Active pane progress state, when known.
  --max-files N                 Maximum file entities. Default: 5.
  --strict                      Fail when a user-provided --open file is missing.
  --brief-output PATH           Task brief path to write or link from a scene.

Options for discover:
  --scene-output PATH           Write generated scene to PATH.
  --install                     Install as ~/.config/gameterm/scenes/default.json.
  --config-home PATH            Use PATH instead of XDG_CONFIG_HOME or ~/.config.
  --force                       Overwrite output/install target.

Options for patch:
  --base PATH                   Base scene for patch validation.
                                Default: ci/fixtures/gameterm-scene/workspace-agent.json.
  --patch-output PATH           Write generated patch to PATH.
  --inbox PATH                  Atomically write generated patch to Scene inbox.
  --force                       Overwrite patch output.

Examples:
  ci/gameterm-scene-workspace.sh inspect --cwd .

  ci/gameterm-scene-workspace.sh discover \
    --cwd . \
    --scene-output /tmp/gameterm-workspace.json

  ci/gameterm-scene-workspace.sh discover --cwd . --install --force

  ci/gameterm-scene-workspace.sh dogfood --cwd . --install --force

  ci/gameterm-scene-workspace.sh patch \
    --cwd . \
    --patch-output /tmp/gameterm-workspace.patch.json

  ci/gameterm-scene-workspace.sh brief \
    --cwd . \
    --brief-output /tmp/gameterm-task-brief.json
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="${repo_root}/ci/fixtures/gameterm-scene"
command="${1:-}"

if [[ -z "${command}" ]]; then
  usage >&2
  exit 2
fi
shift

cwd="."
cwd_provided=0
title_override=""
task_text=""
verify_argv=""
max_files=5
max_files_provided=0
strict=0
scene_output=""
patch_output=""
brief_output=""
base_scene="${fixture_root}/workspace-agent.json"
inbox_path=""
install=0
force=0
config_home="${XDG_CONFIG_HOME:-${HOME}/.config}"
open_paths=()
pane_id=""
mux_window_id=""
pane_cwd=""
foreground_process_name=""
foreground_process_path=""
pane_progress=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --cwd)
      cwd="$2"
      cwd_provided=1
      shift 2
      ;;
    --title)
      title_override="$2"
      shift 2
      ;;
    --task)
      task_text="$2"
      shift 2
      ;;
    --verify-argv)
      verify_argv="$2"
      shift 2
      ;;
    --open)
      open_paths+=("$2")
      shift 2
      ;;
    --pane-id)
      pane_id="$2"
      shift 2
      ;;
    --mux-window-id)
      mux_window_id="$2"
      shift 2
      ;;
    --pane-cwd)
      pane_cwd="$2"
      shift 2
      ;;
    --foreground-process-name)
      foreground_process_name="$2"
      shift 2
      ;;
    --foreground-process-path)
      foreground_process_path="$2"
      shift 2
      ;;
    --pane-progress)
      pane_progress="$2"
      shift 2
      ;;
    --max-files)
      max_files="$2"
      max_files_provided=1
      shift 2
      ;;
    --strict)
      strict=1
      shift
      ;;
    --scene-output)
      scene_output="$2"
      shift 2
      ;;
    --patch-output)
      patch_output="$2"
      shift 2
      ;;
    --brief-output|--output)
      brief_output="$2"
      shift 2
      ;;
    --base)
      base_scene="$2"
      shift 2
      ;;
    --inbox)
      inbox_path="$2"
      shift 2
      ;;
    --install)
      install=1
      shift
      ;;
    --config-home)
      config_home="$2"
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

if [[ ! "${max_files}" =~ ^[0-9]+$ || "${max_files}" -eq 0 ]]; then
  echo "--max-files must be a positive integer" >&2
  exit 2
fi
if [[ -n "${pane_id}" && ! "${pane_id}" =~ ^[0-9]+$ ]]; then
  echo "--pane-id must be a non-negative integer" >&2
  exit 2
fi
if [[ -n "${mux_window_id}" && ! "${mux_window_id}" =~ ^[0-9]+$ ]]; then
  echo "--mux-window-id must be a non-negative integer" >&2
  exit 2
fi
if [[ -n "${verify_argv}" ]]; then
  if ! jq -e 'type == "array" and length > 0 and all(.[]; type == "string" and length > 0)' \
    <<<"${verify_argv}" >/dev/null; then
    echo "--verify-argv must be a non-empty JSON string array" >&2
    exit 2
  fi
fi

resolve_dir() {
  local path="$1"
  if [[ ! -d "${path}" ]]; then
    echo "workspace cwd does not exist or is not a directory: ${path}" >&2
    exit 2
  fi
  (cd "${path}" && pwd -P)
}

if [[ "${cwd_provided}" -eq 0 && -n "${pane_cwd}" ]]; then
  cwd="${pane_cwd}"
fi
workspace_dir="$(resolve_dir "${cwd}")"

git_value() {
  local fallback="$1"
  shift
  if git -C "${workspace_dir}" "$@" >/tmp/gameterm-scene-workspace-git.out 2>/dev/null; then
    cat /tmp/gameterm-scene-workspace-git.out
  else
    printf '%s\n' "${fallback}"
  fi
}

git_root="$(git_value "" rev-parse --show-toplevel)"
if [[ -n "${git_root}" ]]; then
  root_dir="${git_root}"
  repo_status="clean"
  changed_files="$(git -C "${workspace_dir}" status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
  if [[ "${changed_files}" != "0" ]]; then
    repo_status="dirty"
  fi
  repo_branch="$(git_value unknown branch --show-current)"
  if [[ -z "${repo_branch}" ]]; then
    repo_branch="detached"
  fi
  repo_revision="$(git_value unknown rev-parse --short HEAD)"
else
  root_dir="${workspace_dir}"
  repo_status="not_git"
  changed_files="0"
  repo_branch="unknown"
  repo_revision="unknown"
fi

project_label="$(basename "${root_dir}")"
dogfood_profile=0
if [[ "${command}" == "dogfood" ]]; then
  dogfood_profile=1
  scene_title="${title_override:-GameTerm Dogfood Workspace}"
  task_label="${task_text:-Dogfood Scene Mode daily workspace}"
  if [[ "${max_files_provided}" -eq 0 ]]; then
    max_files=10
  fi
  if [[ "${install}" -eq 1 && -z "${brief_output}" ]]; then
    brief_output="${config_home}/gameterm/scenes/dogfood-task-brief.json"
  fi
else
  scene_title="${title_override:-${project_label} Workspace}"
  task_label="${task_text:-Review ${project_label} workspace}"
fi

language="unknown"
manifest=""
if [[ -f "${root_dir}/Cargo.toml" ]]; then
  language="rust"
  manifest="Cargo.toml"
elif [[ -f "${root_dir}/package.json" ]]; then
  language="node"
  manifest="package.json"
elif [[ -f "${root_dir}/pyproject.toml" ]]; then
  language="python"
  manifest="pyproject.toml"
fi

if [[ -z "${verify_argv}" ]]; then
  if [[ "${dogfood_profile}" -eq 1 && -x "${root_dir}/ci/gameterm-scene-verify.sh" ]]; then
    verify_argv='["ci/gameterm-scene-verify.sh","--all"]'
  elif [[ -x "${root_dir}/ci/gameterm-scene-verify.sh" ]]; then
    verify_argv='["ci/gameterm-scene-verify.sh","--fixture","workspace-agent"]'
  elif [[ "${language}" == "rust" ]]; then
    verify_argv='["cargo","test"]'
  elif [[ "${language}" == "node" ]] \
    && jq -e '.scripts.test? and (.scripts.test | type == "string")' \
      "${root_dir}/package.json" >/dev/null 2>&1; then
    verify_argv='["npm","test"]'
  fi
fi

pane_context="absent"
if [[ -n "${pane_id}${mux_window_id}${pane_cwd}${foreground_process_name}${foreground_process_path}${pane_progress}" ]]; then
  pane_context="provided"
fi
discovery_source="cwd"
if [[ "${cwd_provided}" -eq 0 && -n "${pane_cwd}" ]]; then
  discovery_source="pane_cwd"
elif [[ "${pane_context}" == "provided" ]]; then
  discovery_source="cwd_with_pane_metadata"
fi
active_process_label="No foreground process"
active_process_phase="none"
active_process_command=""
active_process_message="No active pane process metadata"
if [[ -n "${foreground_process_name}" ]]; then
  active_process_label="$(basename "${foreground_process_name}")"
  active_process_command="${foreground_process_name}"
  active_process_phase="running"
  active_process_message="Foreground process detected"
elif [[ -n "${foreground_process_path}" ]]; then
  active_process_label="$(basename "${foreground_process_path}")"
  active_process_command="${foreground_process_path}"
  active_process_phase="running"
  active_process_message="Foreground process detected"
fi
if [[ -n "${pane_progress}" && "${active_process_phase}" == "none" ]]; then
  active_process_label="Pane progress"
  active_process_phase="running"
  active_process_message="Pane progress reported"
fi

relative_path() {
  local path="$1"
  case "${path}" in
    "${root_dir}") printf '.\n' ;;
    "${root_dir}"/*) printf '%s\n' "${path#"${root_dir}/"}" ;;
    *) printf '%s\n' "${path}" ;;
  esac
}

file_role() {
  local path="$1"
  case "$(basename "${path}")" in
    README.md) printf 'readme\n' ;;
    AGENTS.md|CODING_PRINCIPLES.md) printf 'principles\n' ;;
    Cargo.toml|package.json|pyproject.toml) printf 'manifest\n' ;;
    .gitignore) printf 'config\n' ;;
    *) printf 'doc\n' ;;
  esac
}

file_kind() {
  local role="$1"
  case "${role}" in
    principles|config) printf 'Principle\n' ;;
    *) printf 'Memory\n' ;;
  esac
}

add_file_entry() {
  local raw="$1"
  local user_provided="$2"
  local abs rel role kind label

  if [[ "${raw}" = /* ]]; then
    abs="${raw}"
  else
    abs="${root_dir}/${raw}"
  fi
  if [[ ! -f "${abs}" ]]; then
    if [[ "${user_provided}" -eq 1 && "${strict}" -eq 1 ]]; then
      echo "important file does not exist: ${raw}" >&2
      exit 2
    fi
    if [[ "${user_provided}" -eq 1 ]]; then
      echo "WARN: important file missing: ${raw}" >&2
    fi
    return 0
  fi

  rel="$(relative_path "${abs}")"
  for existing in "${file_entries[@]:-}"; do
    if [[ "${existing}" == "${rel}"$'\t'* ]]; then
      return 0
    fi
  done
  role="$(file_role "${rel}")"
  kind="$(file_kind "${role}")"
  label="$(basename "${rel}")"
  file_entries+=("${rel}"$'\t'"${role}"$'\t'"${kind}"$'\t'"${label}")
}

if [[ "${dogfood_profile}" -eq 1 ]]; then
  dogfood_files=(
    docs/gameterm-scene-roadmap.md
    docs/gameterm-scene-dogfood-workspace-scope.md
    docs/gameterm-scene-onboarding.md
    docs/gameterm-scene-smoke-report.md
    docs/gameterm-scene-refactor-plan.md
  )
  for path in "${dogfood_files[@]}"; do
    open_paths+=("${path}")
  done
fi

file_entries=()
if [[ "${#open_paths[@]}" -gt 0 ]]; then
  for path in "${open_paths[@]}"; do
    add_file_entry "${path}" 1
  done
fi

default_files=(
  README.md
  AGENTS.md
  CODING_PRINCIPLES.md
  docs/gameterm-scene-workspace-discovery-scope.md
  docs/gameterm-scene-agent-workspace-scope.md
  docs/gameterm-scene-runtime-roadmap.md
  Cargo.toml
  package.json
  pyproject.toml
  .gitignore
)
for path in "${default_files[@]}"; do
  if [[ "${#file_entries[@]}" -ge "${max_files}" ]]; then
    break
  fi
  add_file_entry "${path}" 0
done

files_json="[]"
if [[ "${#file_entries[@]}" -gt 0 ]]; then
  files_json="$(
    printf '%s\n' "${file_entries[@]}" | jq -R -s '
      split("\n")[:-1]
      | map(split("\t") | {
          path: .[0],
          role: .[1],
          kind: .[2],
          label: .[3]
        })
    '
  )"
fi

verify_json="${verify_argv:-null}"
if [[ "${verify_json}" != "null" ]]; then
  verify_label="$(jq -r 'join(" ")' <<<"${verify_json}")"
else
  verify_label=""
fi

scene_json() {
  jq -n \
    --arg title "${scene_title}" \
    --arg project_label "${project_label}" \
    --arg root "${root_dir}" \
    --arg workspace "${workspace_dir}" \
    --arg branch "${repo_branch}" \
    --arg revision "${repo_revision}" \
    --arg status "${repo_status}" \
    --arg changed "${changed_files}" \
    --arg language "${language}" \
    --arg manifest "${manifest}" \
    --arg task "${task_label}" \
    --arg verify_label "${verify_label}" \
    --arg pane_context "${pane_context}" \
    --arg pane_id "${pane_id}" \
    --arg mux_window_id "${mux_window_id}" \
    --arg pane_cwd "${pane_cwd}" \
    --arg foreground_process_name "${foreground_process_name}" \
    --arg foreground_process_path "${foreground_process_path}" \
    --arg pane_progress "${pane_progress}" \
    --arg discovery_source "${discovery_source}" \
    --arg active_process_label "${active_process_label}" \
    --arg active_process_phase "${active_process_phase}" \
    --arg active_process_command "${active_process_command}" \
    --arg active_process_message "${active_process_message}" \
    --arg brief_output "${brief_output}" \
    --arg dogfood_profile "${dogfood_profile}" \
    --argjson files "${files_json}" \
    --argjson verify "${verify_json}" \
    '{
      title: $title,
      background: "workspace-map",
      width: 16,
      height: 9,
      mode: {
        mode_id: "workspace",
        label: "Workspace",
        description: "Discovered workspace state",
        scene_profile: "scene",
        allowed_actions: ["Inspect", "OpenFile", "RunCommand"]
      },
      layers: [
        { layer_id: "workspace", state: "overview", label: "Workspace" },
        { layer_id: "agent", state: "idle", label: "Agent" },
        { layer_id: "process", state: "none", label: "Process" },
        {
          layer_id: "ui",
          state: "scene",
          label: "UI",
          input_map: [{ input: "other", action: "toggle_debug" }]
        }
      ],
      variables: ([
        { key: "workspace_mode", value: { Text: "overview" } },
        { key: "workspace_root", value: { Text: $root } },
        { key: "repo_branch", value: { Text: $branch } },
        { key: "repo_status", value: { Text: $status } },
        { key: "active_task_id", value: { Text: "discovered-task" } },
        { key: "process_phase", value: { Text: $active_process_phase } },
        { key: "verification_status", value: { Text: "not_run" } },
        { key: "discovery_source", value: { Text: $discovery_source } },
        { key: "pane_context", value: { Text: $pane_context } },
        { key: "discovered_file_count", value: { Number: ($files | length) } },
        { key: "dogfood_profile", value: { Text: (if $dogfood_profile == "1" then "true" else "false" end) } }
      ] + (if $pane_id == "" then [] else [{ key: "active_pane_id", value: { Number: ($pane_id | tonumber) } }] end)
        + (if $mux_window_id == "" then [] else [{ key: "active_mux_window_id", value: { Number: ($mux_window_id | tonumber) } }] end)),
      rpg: {
        relationships: ([
          {
            source_id: "discovered-workspace",
            target_id: "discovered-project",
            kind: "contains",
            value: 1,
            metadata: [["source", "workspace-discovery"], ["reason", "project root belongs to workspace"]]
          },
          {
            source_id: "discovered-task",
            target_id: "discovered-project",
            kind: "targets",
            value: 1,
            metadata: [["source", "workspace-discovery"], ["reason", "task was generated for discovered project"]]
          },
          {
            source_id: "discovered-process",
            target_id: "discovered-project",
            kind: "verifies",
            value: 1,
            metadata: [["source", "workspace-discovery"], ["reason", $verify_label]]
          }
        ] + (if $pane_context == "provided" then [{
          source_id: "discovered-pane",
          target_id: "discovered-process",
          kind: "observes",
          value: 1,
          metadata: [["source", "pane-metadata"], ["reason", "pane metadata supplied active process context"]]
        }] else [] end)
          + ($files | to_entries | map({
            source_id: "discovered-project",
            target_id: ("file-" + (.key | tostring)),
            kind: "includes",
            value: 1,
            metadata: [["source", "workspace-discovery"], ["role", .value.role], ["path", .value.path]]
          }))
          + ($files | to_entries | map({
            source_id: "discovered-task",
            target_id: ("file-" + (.key | tostring)),
            kind: "references",
            value: 1,
            metadata: [["source", "workspace-discovery"], ["reason", "task related_files metadata"], ["path", .value.path]]
          }))
          + (if $brief_output == "" then [] else [{
            source_id: "discovered-task",
            target_id: "task-brief",
            kind: "described_by",
            value: 1,
            metadata: [["source", "task-bootstrap"], ["path", $brief_output]]
          }] end))
      },
      entities: ([
        {
          id: "discovered-workspace",
          kind: "Project",
          label: ($project_label + " Workspace"),
          position: { x: 2, y: 2 },
          sprite: "workspace-map",
          state_flags: (["workspace", "active"] + if $status == "clean" then ["git_clean"] elif $status == "dirty" then ["git_dirty"] else [] end),
          metadata: ([
            ["entity_type", "workspace"],
            ["root", $root],
            ["cwd", $workspace],
            ["repo_branch", $branch],
            ["repo_revision", $revision],
            ["repo_status", $status],
            ["changed_files", $changed],
            ["discovery_source", $discovery_source],
            ["dogfood_profile", (if $dogfood_profile == "1" then "true" else "false" end)]
          ] + (if $pane_id == "" then [] else [["active_pane_id", $pane_id]] end)
            + (if $mux_window_id == "" then [] else [["active_mux_window_id", $mux_window_id]] end)
            + (if $pane_cwd == "" then [] else [["pane_cwd", $pane_cwd]] end))
        },
        {
          id: "discovered-project",
          kind: "Project",
          label: $project_label,
          position: { x: 5, y: 2 },
          sprite: "project_core",
          state_flags: ["project", "active"],
          metadata: [
            ["entity_type", "project"],
            ["root", $root],
            ["language", $language],
            ["manifest", $manifest],
            ["verification", $verify_label]
          ]
        },
        {
          id: "discovered-pane",
          kind: "Task",
          label: (if $pane_id == "" then "Active Pane" else ("Pane " + $pane_id) end),
          position: { x: 11, y: 2 },
          sprite: "task_tile",
          state_flags: (["pane"] + (if $pane_context == "provided" then ["active"] else ["unknown"] end)),
          metadata: ([
            ["entity_type", "pane"],
            ["context", $pane_context]
          ] + (if $pane_id == "" then [] else [["pane_id", $pane_id]] end)
            + (if $mux_window_id == "" then [] else [["mux_window_id", $mux_window_id]] end)
            + (if $pane_cwd == "" then [] else [["cwd", $pane_cwd]] end)
            + (if $pane_progress == "" then [] else [["progress", $pane_progress]] end))
        },
        {
          id: "discovered-task",
          kind: "Task",
          label: $task,
          position: { x: 8, y: 4 },
          sprite: "task_tile",
          state_flags: ["task", "discovered"],
          metadata: [
            ["entity_type", "task"],
            ["objective", $task],
            ["owner", "user"],
            ["phase", "discovered"],
            ["related_files", ($files | map(.path) | join(","))]
          ]
        },
        {
          id: "discovered-process",
          kind: "Task",
          label: $active_process_label,
          position: { x: 12, y: 5 },
          sprite: "task_tile",
          state_flags: ["process", $active_process_phase],
          metadata: ([
            ["entity_type", "process"],
            ["command", $active_process_command],
            ["cwd", $root],
            ["phase", $active_process_phase],
            ["message", $active_process_message]
          ] + (if $foreground_process_name == "" then [] else [["foreground_process_name", $foreground_process_name]] end)
            + (if $foreground_process_path == "" then [] else [["foreground_process_path", $foreground_process_path]] end)
            + (if $pane_progress == "" then [] else [["pane_progress", $pane_progress]] end))
        }
      ] + ($files | to_entries | map({
        id: ("file-" + (.key | tostring)),
        kind: .value.kind,
        label: .value.label,
        position: { x: ((.key % 5) + 2), y: ((.key / 5 | floor) + 6) },
        sprite: "memory_note",
        state_flags: ["file", .value.role],
        metadata: [
          ["entity_type", "file"],
          ["path", .value.path],
          ["role", .value.role],
          ["status", "present"]
        ]
      }))
        + (if $brief_output == "" then [] else [{
          id: "task-brief",
          kind: "Task",
          label: "Task Brief",
          position: { x: 14, y: 7 },
          sprite: "memory_note",
          state_flags: ["brief", "inspectable"],
          metadata: [
            ["entity_type", "task_brief"],
            ["path", $brief_output],
            ["source", "workspace-discovery"],
            ["status", "generated"]
          ]
        }] end)),
      dialogue_speaker: "Workspace",
      dialogue: ("Discovered " + $project_label + " from " + $root + "."),
      dialogue_lines: [
        {
          speaker: "Workspace",
          text: ("Discovered " + $project_label + " from " + $root + "."),
          portrait: "project_core"
        }
      ],
      choices: ([
        {
          label: "Inspect workspace",
          kind: "Inspect",
          policy: {
            origin: "workspace_discovery",
            risk: "inspect",
            scope: "workspace",
            summary: "Inspect the generated workspace state"
          }
        }
      ] + (if $brief_output == "" then [] else [{
        label: "Open task brief",
        kind: { OpenFile: { path: $brief_output } },
        policy: {
          origin: "workspace_discovery",
          risk: "open_file",
          scope: "workspace",
          summary: "Open the generated local task brief"
        }
      }] end)
      + ($files | map({
        label: ("Open " + .label),
        kind: { OpenFile: { path: .path } },
        policy: {
          origin: "workspace_discovery",
          risk: "open_file",
          scope: "workspace",
          summary: ("Open discovered file " + .label)
        }
      })) + (if $verify == null then [] else [{
        label: "Run verification",
        kind: {
          RunCommand: {
            argv: $verify,
            cwd: $root,
            target: "split_down"
          }
        },
        policy: {
          origin: "workspace_discovery",
          risk: "command",
          scope: "workspace",
          requires_confirmation: true,
          summary: "Run the explicit verification command in the discovered workspace"
        }
      }] end)
      + (if $dogfood_profile == "1" then [
        {
          label: "Run git status",
          kind: {
            RunCommand: {
              argv: ["git", "status", "--short"],
              cwd: $root,
              target: "split_down"
            }
          },
          policy: {
            origin: "workspace_discovery",
            risk: "command",
            scope: "workspace",
            requires_confirmation: true,
            summary: "Inspect the working tree before dogfood changes"
          }
        },
        {
          label: "Run dogfood smoke",
          kind: {
            RunCommand: {
              argv: ["ci/gameterm-scene-smoke.sh", "--scenario", "dogfood", "--check-assets"],
              cwd: $root,
              target: "split_down"
            }
          },
          policy: {
            origin: "workspace_discovery",
            risk: "command",
            scope: "workspace",
            requires_confirmation: true,
            summary: "Run the focused dogfood Scene Mode smoke check"
          }
        }
      ] else [] end))
    }'
}

patch_json() {
  jq -n \
    --arg project_label "${project_label}" \
    --arg root "${root_dir}" \
    --arg workspace "${workspace_dir}" \
    --arg branch "${repo_branch}" \
    --arg revision "${repo_revision}" \
    --arg status "${repo_status}" \
    --arg changed "${changed_files}" \
    --arg language "${language}" \
    --arg manifest "${manifest}" \
    --arg task "${task_label}" \
    --arg verify_label "${verify_label}" \
    --arg pane_context "${pane_context}" \
    --arg pane_id "${pane_id}" \
    --arg mux_window_id "${mux_window_id}" \
    --arg pane_cwd "${pane_cwd}" \
    --arg foreground_process_name "${foreground_process_name}" \
    --arg foreground_process_path "${foreground_process_path}" \
    --arg pane_progress "${pane_progress}" \
    --arg discovery_source "${discovery_source}" \
    --arg active_process_label "${active_process_label}" \
    --arg active_process_phase "${active_process_phase}" \
    --arg active_process_command "${active_process_command}" \
    --arg active_process_message "${active_process_message}" \
    --argjson files "${files_json}" \
    '{
      scene_patch_version: 1,
      status: ("Workspace discovered: " + $project_label + " (" + $status + ")"),
      selected_entity_id: "workspace-gameterm",
      variables: ([
        { key: "workspace_mode", value: { Text: "overview" } },
        { key: "workspace_root", value: { Text: $root } },
        { key: "repo_branch", value: { Text: $branch } },
        { key: "repo_status", value: { Text: $status } },
        { key: "discovery_source", value: { Text: $discovery_source } },
        { key: "pane_context", value: { Text: $pane_context } },
        { key: "process_phase", value: { Text: $active_process_phase } },
        { key: "discovered_file_count", value: { Number: ($files | length) } }
      ] + (if $pane_id == "" then [] else [{ key: "active_pane_id", value: { Number: ($pane_id | tonumber) } }] end)
        + (if $mux_window_id == "" then [] else [{ key: "active_mux_window_id", value: { Number: ($mux_window_id | tonumber) } }] end)),
      updates: [
        {
          entity_id: "workspace-gameterm",
          label: ($project_label + " Workspace"),
          state_flags: (["workspace", "active"] + if $status == "clean" then ["git_clean"] elif $status == "dirty" then ["git_dirty"] else [] end),
          metadata: ([
            ["entity_type", "workspace"],
            ["root", $root],
            ["cwd", $workspace],
            ["repo_branch", $branch],
            ["repo_revision", $revision],
            ["repo_status", $status],
            ["changed_files", $changed],
            ["discovery_source", $discovery_source]
          ] + (if $pane_id == "" then [] else [["active_pane_id", $pane_id]] end)
            + (if $mux_window_id == "" then [] else [["active_mux_window_id", $mux_window_id]] end)
            + (if $pane_cwd == "" then [] else [["pane_cwd", $pane_cwd]] end))
        },
        {
          entity_id: "project-scene-mode",
          label: $project_label,
          metadata: [
            ["entity_type", "project"],
            ["root", $root],
            ["language", $language],
            ["manifest", $manifest],
            ["verification", $verify_label]
          ]
        },
        {
          entity_id: "scene-agent-workspace-task",
          label: $task,
          state_flags: ["task", "discovered"],
          metadata: [
            ["entity_type", "task"],
            ["objective", $task],
            ["owner", "user"],
            ["phase", "discovered"],
            ["related_files", ($files | map(.path) | join(","))]
          ]
        },
        {
          entity_id: "scene-verify-process",
          label: $active_process_label,
          state_flags: ["process", $active_process_phase],
          metadata: ([
            ["entity_type", "process"],
            ["command", $active_process_command],
            ["cwd", $root],
            ["phase", $active_process_phase],
            ["message", $active_process_message]
          ] + (if $foreground_process_name == "" then [] else [["foreground_process_name", $foreground_process_name]] end)
            + (if $foreground_process_path == "" then [] else [["foreground_process_path", $foreground_process_path]] end)
            + (if $pane_progress == "" then [] else [["pane_progress", $pane_progress]] end))
        },
        {
          entity_id: "scope-doc",
          label: (if ($files | length) > 0 then $files[0].label else "Workspace File" end),
          metadata: [
            ["entity_type", "file"],
            ["path", (if ($files | length) > 0 then $files[0].path else "" end)],
            ["role", (if ($files | length) > 0 then $files[0].role else "file" end)]
          ]
        },
        {
          entity_id: "fixture-file",
          label: (if ($files | length) > 1 then $files[1].label else "Workspace Fixture" end),
          metadata: [
            ["entity_type", "file"],
            ["path", (if ($files | length) > 1 then $files[1].path else "" end)],
            ["role", (if ($files | length) > 1 then $files[1].role else "file" end)]
          ]
        }
      ],
      process_state: (if $active_process_phase == "none" then null else {
        entity_id: "scene-verify-process",
        phase: $active_process_phase,
        command: (if $active_process_command == "" then null else $active_process_command end),
        message: $active_process_message
      } end)
    }'
}

brief_json() {
  jq -n \
    --arg project_label "${project_label}" \
    --arg root "${root_dir}" \
    --arg workspace "${workspace_dir}" \
    --arg branch "${repo_branch}" \
    --arg revision "${repo_revision}" \
    --arg status "${repo_status}" \
    --arg changed "${changed_files}" \
    --arg language "${language}" \
    --arg manifest "${manifest}" \
    --arg objective "${task_label}" \
    --arg verify_label "${verify_label}" \
    --arg pane_context "${pane_context}" \
    --arg pane_id "${pane_id}" \
    --arg pane_cwd "${pane_cwd}" \
    --arg foreground_process_name "${foreground_process_name}" \
    --arg foreground_process_path "${foreground_process_path}" \
    --arg pane_progress "${pane_progress}" \
    --arg active_process_phase "${active_process_phase}" \
    --arg active_process_command "${active_process_command}" \
    --arg generated_at "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
    --argjson files "${files_json}" \
    --argjson verify "${verify_json}" \
    '{
      brief_version: 1,
      generated_at: $generated_at,
      workspace_root: $root,
      workspace_cwd: $workspace,
      project_label: $project_label,
      objective: $objective,
      repo: {
        branch: $branch,
        revision: $revision,
        status: $status,
        changed_files: ($changed | tonumber)
      },
      project: {
        language: $language,
        manifest: $manifest
      },
      context_files: ($files | map(.path)),
      verification: $verify,
      verification_label: $verify_label,
      pane_context: {
        status: $pane_context,
        pane_id: (if $pane_id == "" then null else ($pane_id | tonumber) end),
        cwd: (if $pane_cwd == "" then null else $pane_cwd end),
        foreground_process_name: (if $foreground_process_name == "" then null else $foreground_process_name end),
        foreground_process_path: (if $foreground_process_path == "" then null else $foreground_process_path end),
        progress: (if $pane_progress == "" then null else $pane_progress end)
      },
      active_process: {
        phase: $active_process_phase,
        command: $active_process_command
      },
      constraints: [
        "do not run commands automatically",
        "do not start agents automatically",
        "inspect this brief before handoff"
      ]
    }'
}

validate_scene() {
  cargo run -q -p gameterm-visual --example scene_validate -- "$1" >/dev/null
}

validate_generated_layout() {
  local scene="$1"
  jq -e '
    .width as $width
    | .height as $height
    |
    (.entities | map(select(.visible // true)) | length) as $visible_count
    | (.entities
      | map(select(.visible // true) | "\(.position.x),\(.position.y)")
      | unique
      | length) == $visible_count
    and all(.entities[]; .position.x >= 0 and .position.x < $width and .position.y >= 0 and .position.y < $height)
    and any(.entities[]; .id == "discovered-workspace" and .position.x <= 3 and .position.y <= 3)
    and any(.entities[]; .id == "discovered-project" and .position.x <= 6 and .position.y <= 3)
    and any(.entities[]; .id == "discovered-pane" and .position.x >= 10 and .position.y <= 3)
    and any(.entities[]; .id == "discovered-process" and .position.x >= 10 and .position.y >= 4)
    and all(.entities[] | select((.metadata // []) | any(.[0] == "entity_type" and .[1] == "file")); .position.y >= 6)
  ' "${scene}" >/dev/null
}

write_output_file() {
  local source="$1"
  local target="$2"
  if [[ -e "${target}" && "${force}" -ne 1 ]]; then
    cat >&2 <<EOF
${target} already exists.

Rerun with --force to overwrite it.
EOF
    exit 1
  fi
  mkdir -p "$(dirname "${target}")"
  cp "${source}" "${target}"
  echo "Wrote ${target}"
}

run_inspect() {
  cat <<EOF
workspace_dir=${workspace_dir}
root_dir=${root_dir}
project_label=${project_label}
repo_status=${repo_status}
repo_branch=${repo_branch}
repo_revision=${repo_revision}
changed_files=${changed_files}
language=${language}
manifest=${manifest}
file_count=$(jq 'length' <<<"${files_json}")
verify_argv=${verify_argv:-}
discovery_source=${discovery_source}
pane_context=${pane_context}
pane_id=${pane_id}
mux_window_id=${mux_window_id}
pane_cwd=${pane_cwd}
foreground_process_name=${foreground_process_name}
foreground_process_path=${foreground_process_path}
pane_progress=${pane_progress}
active_process_phase=${active_process_phase}
EOF
}

run_discover() {
  local tmp install_target
  tmp="$(mktemp /tmp/gameterm-scene-workspace.XXXXXX)"
  if [[ -n "${brief_output}" ]]; then
    run_brief >/dev/null
  fi
  scene_json >"${tmp}"
  validate_scene "${tmp}"
  validate_generated_layout "${tmp}"

  if [[ -n "${scene_output}" ]]; then
    write_output_file "${tmp}" "${scene_output}"
  fi
  if [[ "${install}" -eq 1 ]]; then
    install_target="${config_home}/gameterm/scenes/default.json"
    write_output_file "${tmp}" "${install_target}"
  fi
  if [[ -z "${scene_output}" && "${install}" -eq 0 ]]; then
    cat "${tmp}"
  fi
  rm -f "${tmp}"
}

run_patch() {
  local tmp
  tmp="$(mktemp /tmp/gameterm-scene-workspace-patch.XXXXXX)"
  patch_json >"${tmp}"
  "${repo_root}/ci/gameterm-scene-patch.sh" validate \
    --scene "${base_scene}" \
    --patch "${tmp}" >/dev/null

  if [[ -n "${patch_output}" ]]; then
    write_output_file "${tmp}" "${patch_output}"
  fi
  if [[ -n "${inbox_path}" ]]; then
    "${repo_root}/ci/gameterm-scene-patch.sh" write-inbox \
      --inbox "${inbox_path}" \
      --patch "${tmp}" >/dev/null
    echo "Wrote inbox ${inbox_path}"
  fi
  if [[ -z "${patch_output}" && -z "${inbox_path}" ]]; then
    cat "${tmp}"
  fi
  rm -f "${tmp}"
}

run_brief() {
  if [[ -z "${brief_output}" ]]; then
    echo "missing required --brief-output" >&2
    usage >&2
    exit 2
  fi
  local tmp
  tmp="$(mktemp /tmp/gameterm-scene-task-brief.XXXXXX)"
  brief_json >"${tmp}"
  jq -e '
    .brief_version == 1
    and (.workspace_root | type == "string" and length > 0)
    and (.objective | type == "string" and length > 0)
    and (.context_files | type == "array")
    and (.constraints | index("do not start agents automatically"))
  ' "${tmp}" >/dev/null
  write_output_file "${tmp}" "${brief_output}"
  rm -f "${tmp}"
}

case "${command}" in
  inspect)
    run_inspect
    ;;
  discover|dogfood)
    run_discover
    ;;
  patch)
    run_patch
    ;;
  brief)
    run_brief
    ;;
  -h|--help)
    usage
    ;;
  *)
    echo "unknown command: ${command}" >&2
    usage >&2
    exit 2
    ;;
esac
