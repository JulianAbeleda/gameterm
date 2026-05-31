#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-author.sh COMMAND [OPTIONS]

Authoring helper for GameTerm Scene Mode JSON files.

Commands:
  validate PATH                 Validate a scene file with the Rust scene parser.
  new-scene PATH                Create a minimal editable scene file.
  new-template PATH             Create a guided template scene.
  add-entity PATH               Add an entity to a scene file.
  add-choice PATH               Add a choice to a scene file.
  remove-choice PATH            Remove a choice by zero-based index.
  update-choice PATH            Replace a choice by zero-based index.
  remove-entity PATH            Remove an entity by id.
  move-entity PATH              Move an entity by id.
  set-dialogue PATH             Set dialogue speaker and text.
  format PATH                   Rewrite a scene file with stable JSON format.
  install-fixture NAME          Install a fixture into the Scene Mode config dir.
  list-fixtures                 List available authoring fixtures.
  list-templates                List available guided templates.

Common options:
  --config-home PATH            Use PATH instead of XDG_CONFIG_HOME or ~/.config.
  --force                       Overwrite existing scene config files.

Options for new-scene:
  --title TEXT                  Scene title. Default: New GameTerm Scene.
  --width N                     Scene width. Default: 12.
  --height N                    Scene height. Default: 7.

Options for new-template:
  --template NAME               Template name. Default: agent-workflow.

Options for add-entity:
  --id ID --kind KIND --label TEXT --x N --y N --sprite ID
  --flag FLAG                   Add one state flag. May be repeated.
  --metadata KEY=VALUE          Add one metadata pair. May be repeated.

Options for remove-entity:
  --id ID

Options for move-entity:
  --id ID --x N --y N

Options for set-dialogue:
  --speaker TEXT --text TEXT

Options for add-choice:
  --label TEXT
  --inspect
  --open-file PATH
  --navigate TARGET
  --run-argv JSON_ARRAY         Explicit argv array, for example:
                                '["cargo","check","-p","gameterm-visual"]'
  --cwd PATH                    Optional cwd for --run-argv.
  --target TARGET               Optional RunCommand target: tab, split_right,
                                or split_down. Default: tab.

Options for remove-choice:
  --choice-index N

Options for update-choice:
  --choice-index N
  --label TEXT
  --inspect
  --open-file PATH
  --navigate TARGET
  --run-argv JSON_ARRAY
  --cwd PATH
  --target TARGET

Fixtures:
  basic, navigate, invalid, sprites, missing-sprite, vertical-slice,
  authoring-loop, game-states

Templates:
  agent-workflow, project-dashboard, visual-novel, layered-mode, rpg-quest,
  vertical-slice
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

config_home="${XDG_CONFIG_HOME:-${HOME}/.config}"
force=0
positionals=()
title="New GameTerm Scene"
width=12
height=7
entity_id=""
entity_kind=""
entity_label=""
entity_x=""
entity_y=""
entity_sprite=""
dialogue_speaker=""
dialogue_text=""
choice_label=""
choice_kind=""
choice_payload=""
choice_cwd=""
choice_target="tab"
choice_index=""
template_name="agent-workflow"
flags=()
metadata=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config-home)
      config_home="$2"
      shift 2
      ;;
    --force)
      force=1
      shift
      ;;
    --title)
      title="$2"
      shift 2
      ;;
    --width)
      width="$2"
      shift 2
      ;;
    --height)
      height="$2"
      shift 2
      ;;
    --id)
      entity_id="$2"
      shift 2
      ;;
    --kind)
      entity_kind="$2"
      shift 2
      ;;
    --label)
      if [[ "${command}" == "add-entity" ]]; then
        entity_label="$2"
      else
        choice_label="$2"
      fi
      shift 2
      ;;
    --x)
      entity_x="$2"
      shift 2
      ;;
    --y)
      entity_y="$2"
      shift 2
      ;;
    --sprite)
      entity_sprite="$2"
      shift 2
      ;;
    --speaker)
      dialogue_speaker="$2"
      shift 2
      ;;
    --text)
      dialogue_text="$2"
      shift 2
      ;;
    --flag)
      flags+=("$2")
      shift 2
      ;;
    --metadata)
      metadata+=("$2")
      shift 2
      ;;
    --inspect)
      choice_kind="Inspect"
      shift
      ;;
    --open-file)
      choice_kind="OpenFile"
      choice_payload="$2"
      shift 2
      ;;
    --navigate)
      choice_kind="Navigate"
      choice_payload="$2"
      shift 2
      ;;
    --run-argv)
      choice_kind="RunCommand"
      choice_payload="$2"
      shift 2
      ;;
    --cwd)
      choice_cwd="$2"
      shift 2
      ;;
    --target)
      choice_target="$2"
      shift 2
      ;;
    --template)
      template_name="$2"
      shift 2
      ;;
    --choice-index)
      choice_index="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --*)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      positionals+=("$1")
      shift
      ;;
  esac
done

copy_file() {
  local source="$1"
  local target="$2"

  if [[ -e "${target}" && "${force}" -ne 1 ]]; then
    cat >&2 <<EOF
${target} already exists.

Rerun with --force to overwrite it.
EOF
    return 1
  fi

  cp "${source}" "${target}"
  echo "Wrote ${target}"
}

require_value() {
  local name="$1"
  local value="$2"
  if [[ -z "${value}" ]]; then
    echo "missing required ${name}" >&2
    usage >&2
    exit 2
  fi
}

validate_scene_file() {
  cargo run -q -p gameterm-visual --example scene_validate -- "$1"
}

write_json() {
  local target="$1"
  local tmp
  tmp="$(mktemp /tmp/gameterm-scene-author.XXXXXX)"
  cat >"${tmp}"
  mv "${tmp}" "${target}"
}

create_scene() {
  local target="$1"
  if [[ -e "${target}" && "${force}" -ne 1 ]]; then
    cat >&2 <<EOF
${target} already exists.

Rerun with --force to overwrite it.
EOF
    return 1
  fi

  mkdir -p "$(dirname "${target}")"
  jq -n \
    --arg title "${title}" \
    --argjson width "${width}" \
    --argjson height "${height}" \
    '{
      title: $title,
      background: "workspace-map",
      width: $width,
      height: $height,
      entities: [],
      dialogue_speaker: "GameTerm",
      dialogue: "Edit this scene, then press r in Scene Mode to reload.",
      choices: [{ label: "Inspect selected entity", kind: "Inspect" }]
    }' | write_json "${target}"
  validate_scene_file "${target}" >/dev/null
  echo "Wrote ${target}"
}

create_template() {
  local target="$1"
  if [[ -e "${target}" && "${force}" -ne 1 ]]; then
    cat >&2 <<EOF
${target} already exists.

Rerun with --force to overwrite it.
EOF
    return 1
  fi

  mkdir -p "$(dirname "${target}")"

  case "${template_name}" in
    agent-workflow)
      jq -n '{
        title: "Agent Workflow",
        background: "workspace-map",
        width: 14,
        height: 8,
        entities: [
          {
            id: "project",
            kind: "Project",
            label: "Project",
            position: { x: 2, y: 2 },
            sprite: "project_core",
            state_flags: ["active"],
            metadata: [["role", "workspace root"]]
          },
          {
            id: "agent",
            kind: "Agent",
            label: "Agent",
            position: { x: 6, y: 3 },
            sprite: "agent_idle",
            state_flags: ["ready"],
            metadata: [["status", "waiting"]]
          },
          {
            id: "task",
            kind: "Task",
            label: "Task",
            position: { x: 10, y: 3 },
            sprite: "task_tile",
            state_flags: ["queued"],
            metadata: [["next", "run verification"]]
          }
        ],
        dialogue_speaker: "GameTerm",
        dialogue: "Use Scene Mode patches to move work from queued to running to verified.",
        choices: [
          { label: "Inspect selected entity", kind: "Inspect" },
          {
            label: "Run visual tests",
            kind: {
              RunCommand: {
                argv: ["cargo", "test", "-p", "gameterm-visual", "scene_patch"],
                target: "split_down"
              }
            }
          }
        ]
      }' | write_json "${target}"
      ;;
    project-dashboard)
      jq -n '{
        title: "Project Dashboard",
        background: "workspace-map",
        width: 16,
        height: 9,
        entities: [
          {
            id: "scope",
            kind: "Principle",
            label: "Scope",
            position: { x: 2, y: 2 },
            sprite: "memory_note",
            state_flags: ["defined"],
            metadata: [["meaning", "current milestone boundary"]]
          },
          {
            id: "implementation",
            kind: "Task",
            label: "Implementation",
            position: { x: 7, y: 4 },
            sprite: "task_tile",
            state_flags: ["running"],
            metadata: [["owner", "development"]]
          },
          {
            id: "audit",
            kind: "Agent",
            label: "Audit",
            position: { x: 12, y: 2 },
            sprite: "agent_idle",
            state_flags: ["watching"],
            metadata: [["owner", "review"]]
          }
        ],
        dialogue_speaker: "Dashboard",
        dialogue: "Track scope, implementation, and audit state as symbolic entities.",
        choices: [
          { label: "Inspect selected entity", kind: "Inspect" },
          {
            label: "Open roadmap",
            kind: { OpenFile: { path: "docs/gameterm-scene-runtime-roadmap.md" } }
          }
        ]
      }' | write_json "${target}"
      ;;
    visual-novel)
      jq -n '{
        title: "Visual Novel Branch",
        background: "workspace-map",
        width: 14,
        height: 8,
        mode: {
          mode_id: "conversation",
          label: "Conversation",
          description: "Dialogue-first Scene Mode template",
          scene_profile: "dialogue",
          allowed_actions: ["Inspect", "AdvanceDialogue", "Resolve"]
        },
        variables: [
          { key: "met_guide", value: { Bool: false } }
        ],
        entities: [
          {
            id: "guide",
            kind: "Agent",
            label: "Guide",
            position: { x: 6, y: 3 },
            sprite: "agent_idle",
            state_flags: ["speaking"],
            metadata: [["mode", "conversation"]]
          }
        ],
        dialogue_speaker: "Guide",
        dialogue: "Choose a branch.",
        dialogue_lines: [
          { speaker: "Guide", text: "Welcome to Scene Mode.", portrait: "agent_idle" },
          { speaker: "Guide", text: "The workspace path is open.", portrait: "agent_idle" },
          { speaker: "Guide", text: "The memory path is still locked.", portrait: "agent_idle" }
        ],
        choices: [
          {
            label: "Mark introduction complete",
            kind: {
              Resolve: {
                operations: [
                  { SetVariable: { key: "met_guide", value: { Bool: true } } }
                ]
              }
            }
          },
          {
            label: "Advance workspace branch",
            kind: { AdvanceDialogue: { target: 1 } },
            conditions: [
              { variable: "met_guide", equals: { Bool: true } }
            ]
          }
        ]
      }' | write_json "${target}"
      ;;
    layered-mode)
      jq -n '{
        title: "Layered Mode Template",
        background: "workspace-map",
        width: 14,
        height: 8,
        mode: {
          mode_id: "workspace",
          label: "Workspace",
          description: "Layered state machine authoring template",
          scene_profile: "scene",
          allowed_actions: ["Inspect", "Resolve"]
        },
        layers: [
          {
            layer_id: "ui",
            state: "scene",
            label: "UI",
            input_map: [
              { input: "other", action: "toggle_debug" }
            ]
          },
          {
            layer_id: "story",
            state: "dialogue",
            label: "Story",
            transitions: [
              {
                input: "activate",
                target_state: "choice",
                conditions: [
                  { variable: "story_ready", equals: { Bool: true } }
                ]
              }
            ]
          }
        ],
        variables: [
          { key: "story_ready", value: { Bool: true } }
        ],
        entities: [
          {
            id: "layered-entity",
            kind: "Project",
            label: "Layered Entity",
            position: { x: 4, y: 3 },
            sprite: "project_core",
            state_flags: ["layered"],
            metadata: [["mode", "workspace"]]
          }
        ],
        dialogue_speaker: "System",
        dialogue: "Layered input and transition state are active.",
        choices: [
          { label: "Inspect selected entity", kind: "Inspect" }
        ]
      }' | write_json "${target}"
      ;;
    rpg-quest)
      jq -n '{
        title: "RPG Quest Template",
        background: "workspace-map",
        width: 14,
        height: 8,
        variables: [
          { key: "quest_reward_claimed", value: { Bool: false } }
        ],
        rpg: {
          inventory: [
            { item_id: "scene-token", label: "Scene Token", count: 1 }
          ],
          stats: [
            { owner_id: "player", key: "focus", value: { Number: 1 } }
          ],
          quests: [
            {
              quest_id: "first-quest",
              label: "First Quest",
              stage: 1,
              completed: false,
              journal: "Find the memory key."
            }
          ],
          relationships: [
            {
              source_id: "player",
              target_id: "guide",
              kind: "trust",
              value: 1
            }
          ]
        },
        entities: [
          {
            id: "guide",
            kind: "Agent",
            label: "Guide",
            position: { x: 5, y: 3 },
            sprite: "agent_idle",
            state_flags: ["quest-giver"],
            metadata: [["mode", "conversation"]]
          }
        ],
        dialogue_speaker: "Guide",
        dialogue: "Claim the quest reward to update inventory, stats, quest, and relationship state.",
        choices: [
          {
            label: "Claim quest reward",
            kind: {
              Resolve: {
                operations: [
                  { SetVariable: { key: "quest_reward_claimed", value: { Bool: true } } },
                  { AddInventory: { item: { item_id: "memory-key", label: "Memory Key", count: 1 } } },
                  { AdjustStat: { owner_id: "player", key: "focus", amount: 1 } },
                  { AdvanceQuest: { quest_id: "first-quest", stage: 2 } },
                  { AdjustRelationship: { source_id: "player", target_id: "guide", kind: "trust", amount: 1 } }
                ]
              }
            },
            conditions: [
              { variable: "quest_reward_claimed", equals: { Bool: false } }
            ]
          }
        ]
      }' | write_json "${target}"
      ;;
    vertical-slice)
      jq -n '{
        title: "Scene Vertical Slice",
        background: "workspace-map",
        width: 16,
        height: 9,
        mode: {
          mode_id: "workspace",
          label: "Workspace",
          description: "Playable Scene Mode vertical slice",
          scene_profile: "scene",
          allowed_actions: ["Inspect", "Resolve", "AdvanceDialogue"]
        },
        layers: [
          {
            layer_id: "ui",
            state: "scene",
            label: "UI",
            input_map: [
              { input: "other", action: "toggle_debug" }
            ]
          },
          {
            layer_id: "story",
            state: "dialogue",
            label: "Story"
          },
          {
            layer_id: "process",
            state: "idle",
            label: "Process"
          }
        ],
        variables: [
          { key: "brief_accepted", value: { Bool: false } },
          { key: "launch_ready", value: { Bool: false } },
          { key: "agent_phase", value: { Text: "idle" } }
        ],
        rpg: {
          inventory: [
            { item_id: "scene-token", label: "Scene Token", count: 1 }
          ],
          stats: [
            { owner_id: "player", key: "focus", value: { Number: 1 } }
          ],
          quests: [
            {
              quest_id: "ship-scene",
              label: "Ship the Scene",
              stage: 1,
              completed: false,
              journal: "Meet the guide and prepare the Scene Mode launch."
            }
          ],
          relationships: [
            {
              source_id: "player",
              target_id: "guide",
              kind: "trust",
              value: 1
            }
          ]
        },
        entities: [
          {
            id: "project-core",
            kind: "Project",
            label: "Scene Project",
            position: { x: 2, y: 2 },
            sprite: "project_core",
            state_flags: ["active"],
            metadata: [["goal", "ship vertical slice"]]
          },
          {
            id: "guide",
            kind: "Agent",
            label: "Guide",
            position: { x: 7, y: 3 },
            sprite: "agent_idle",
            state_flags: ["speaking"],
            metadata: [["relationship", "trust"]]
          },
          {
            id: "build-task",
            kind: "Task",
            label: "Launch Check",
            position: { x: 12, y: 4 },
            sprite: "task_tile",
            state_flags: ["queued"],
            metadata: [["process", "idle"]]
          }
        ],
        dialogue_speaker: "Guide",
        dialogue: "Accept the brief, prepare the launch kit, then run the task check.",
        dialogue_lines: [
          { speaker: "Guide", text: "Scene Mode needs one playable loop.", portrait: "agent_idle" },
          { speaker: "Guide", text: "The launch kit is ready.", portrait: "agent_idle" },
          { speaker: "Guide", text: "The scene is ready to ship.", portrait: "agent_idle" }
        ],
        choices: [
          {
            label: "Accept scene brief",
            kind: {
              Resolve: {
                operations: [
                  { SetVariable: { key: "brief_accepted", value: { Bool: true } } },
                  { AddInventory: { item: { item_id: "scene-brief", label: "Scene Brief", count: 1 } } },
                  { AdvanceQuest: { quest_id: "ship-scene", stage: 2 } },
                  { AdjustRelationship: { source_id: "player", target_id: "guide", kind: "trust", amount: 1 } }
                ]
              }
            },
            conditions: [
              { variable: "brief_accepted", equals: { Bool: false } }
            ]
          },
          {
            label: "Prepare launch kit",
            kind: {
              Resolve: {
                operations: [
                  { SetVariable: { key: "launch_ready", value: { Bool: true } } },
                  { AddInventory: { item: { item_id: "launch-kit", label: "Launch Kit", count: 1 } } },
                  { AdjustStat: { owner_id: "player", key: "focus", amount: 2 } },
                  { AppendQuestJournal: { quest_id: "ship-scene", text: "Prepared the launch kit." } }
                ]
              }
            },
            conditions: [
              { variable: "brief_accepted", equals: { Bool: true } }
            ]
          },
          {
            label: "Complete scene loop",
            kind: {
              Resolve: {
                operations: [
                  { CompleteQuest: { quest_id: "ship-scene" } },
                  { SetVariable: { key: "agent_phase", value: { Text: "complete" } } }
                ]
              }
            },
            conditions: [
              { variable: "launch_ready", equals: { Bool: true } }
            ]
          },
          {
            label: "Read ending",
            kind: { AdvanceDialogue: { target: 2 } },
            conditions: [
              { variable: "agent_phase", equals: { Text: "complete" } }
            ]
          }
        ]
      }' | write_json "${target}"
      ;;
    *)
      echo "unknown template: ${template_name}" >&2
      usage >&2
      exit 2
      ;;
  esac

  validate_scene_file "${target}" >/dev/null
  echo "Wrote ${template_name} template to ${target}"
}

add_entity() {
  local target="$1"
  require_value "--id" "${entity_id}"
  require_value "--kind" "${entity_kind}"
  require_value "--label" "${entity_label}"
  require_value "--x" "${entity_x}"
  require_value "--y" "${entity_y}"
  require_value "--sprite" "${entity_sprite}"

  local flags_json metadata_json
  flags_json="$(printf '%s\n' "${flags[@]}" | jq -R -s 'split("\n")[:-1]')"
  metadata_json="$(printf '%s\n' "${metadata[@]}" | jq -R -s '
    split("\n")[:-1]
    | map(capture("(?<key>[^=]+)=(?<value>.*)") | [.key, .value])
  ')"

  jq \
    --arg id "${entity_id}" \
    --arg kind "${entity_kind}" \
    --arg label "${entity_label}" \
    --argjson x "${entity_x}" \
    --argjson y "${entity_y}" \
    --arg sprite "${entity_sprite}" \
    --argjson flags "${flags_json}" \
    --argjson metadata "${metadata_json}" \
    '.entities += [{
      id: $id,
      kind: $kind,
      label: $label,
      position: { x: $x, y: $y },
      sprite: $sprite,
      state_flags: $flags,
      metadata: $metadata
    }]' "${target}" | write_json "${target}"
  validate_scene_file "${target}" >/dev/null
  echo "Added entity ${entity_id} to ${target}"
}

add_choice() {
  local target="$1"
  require_value "--label" "${choice_label}"
  require_value "choice action" "${choice_kind}"

  case "${choice_kind}" in
    Inspect)
      jq --arg label "${choice_label}" \
        '.choices += [{ label: $label, kind: "Inspect" }]' \
        "${target}" | write_json "${target}"
      ;;
    OpenFile)
      require_value "--open-file" "${choice_payload}"
      jq --arg label "${choice_label}" --arg path "${choice_payload}" \
        '.choices += [{ label: $label, kind: { OpenFile: { path: $path } } }]' \
        "${target}" | write_json "${target}"
      ;;
    Navigate)
      require_value "--navigate" "${choice_payload}"
      jq --arg label "${choice_label}" --arg target_path "${choice_payload}" \
        '.choices += [{ label: $label, kind: { Navigate: { target: $target_path } } }]' \
        "${target}" | write_json "${target}"
      ;;
    RunCommand)
      require_value "--run-argv" "${choice_payload}"
      if [[ -n "${choice_cwd}" ]]; then
        jq --arg label "${choice_label}" \
          --argjson argv "${choice_payload}" \
          --arg cwd "${choice_cwd}" \
          --arg target "${choice_target}" \
          '.choices += [{ label: $label, kind: { RunCommand: { argv: $argv, cwd: $cwd, target: $target } } }]' \
          "${target}" | write_json "${target}"
      else
        jq --arg label "${choice_label}" \
          --argjson argv "${choice_payload}" \
          --arg target "${choice_target}" \
          '.choices += [{ label: $label, kind: { RunCommand: { argv: $argv, target: $target } } }]' \
          "${target}" | write_json "${target}"
      fi
      ;;
  esac

  validate_scene_file "${target}" >/dev/null
  echo "Added choice ${choice_label} to ${target}"
}

choice_json_filter() {
  case "${choice_kind}" in
    Inspect)
      printf '{ label: $label, kind: "Inspect" }'
      ;;
    OpenFile)
      require_value "--open-file" "${choice_payload}"
      printf '{ label: $label, kind: { OpenFile: { path: $payload } } }'
      ;;
    Navigate)
      require_value "--navigate" "${choice_payload}"
      printf '{ label: $label, kind: { Navigate: { target: $payload } } }'
      ;;
    RunCommand)
      require_value "--run-argv" "${choice_payload}"
      if [[ -n "${choice_cwd}" ]]; then
        printf '{ label: $label, kind: { RunCommand: { argv: $argv, cwd: $cwd, target: $target } } }'
      else
        printf '{ label: $label, kind: { RunCommand: { argv: $argv, target: $target } } }'
      fi
      ;;
    *)
      require_value "choice action" "${choice_kind}"
      ;;
  esac
}

remove_choice() {
  local target="$1"
  require_value "--choice-index" "${choice_index}"

  jq --argjson index "${choice_index}" '
    if (.choices | has($index)) then
      .choices |= del(.[$index])
    else
      error("choice index not found: " + ($index | tostring))
    end
  ' "${target}" | write_json "${target}"
  validate_scene_file "${target}" >/dev/null
  echo "Removed choice ${choice_index} from ${target}"
}

update_choice() {
  local target="$1"
  require_value "--choice-index" "${choice_index}"
  require_value "--label" "${choice_label}"
  require_value "choice action" "${choice_kind}"

  local filter
  filter="$(choice_json_filter)"

  if [[ "${choice_kind}" == "RunCommand" ]]; then
    if [[ -n "${choice_cwd}" ]]; then
      jq --argjson index "${choice_index}" \
        --arg label "${choice_label}" \
        --argjson argv "${choice_payload}" \
        --arg cwd "${choice_cwd}" \
        --arg target "${choice_target}" \
        --arg payload "${choice_payload}" \
        "if (.choices | has(\$index)) then .choices[\$index] = ${filter} else error(\"choice index not found: \" + (\$index | tostring)) end" \
        "${target}" | write_json "${target}"
    else
      jq --argjson index "${choice_index}" \
        --arg label "${choice_label}" \
        --argjson argv "${choice_payload}" \
        --arg target "${choice_target}" \
        --arg payload "${choice_payload}" \
        "if (.choices | has(\$index)) then .choices[\$index] = ${filter} else error(\"choice index not found: \" + (\$index | tostring)) end" \
        "${target}" | write_json "${target}"
    fi
  else
    jq --argjson index "${choice_index}" \
      --arg label "${choice_label}" \
      --arg payload "${choice_payload}" \
      "if (.choices | has(\$index)) then .choices[\$index] = ${filter} else error(\"choice index not found: \" + (\$index | tostring)) end" \
      "${target}" | write_json "${target}"
  fi

  validate_scene_file "${target}" >/dev/null
  echo "Updated choice ${choice_index} in ${target}"
}

remove_entity() {
  local target="$1"
  require_value "--id" "${entity_id}"

  jq --arg id "${entity_id}" '
    if any(.entities[]?; .id == $id) then
      .entities |= map(select(.id != $id))
    else
      error("entity id not found: " + $id)
    end
  ' "${target}" | write_json "${target}"
  validate_scene_file "${target}" >/dev/null
  echo "Removed entity ${entity_id} from ${target}"
}

move_entity() {
  local target="$1"
  require_value "--id" "${entity_id}"
  require_value "--x" "${entity_x}"
  require_value "--y" "${entity_y}"

  jq --arg id "${entity_id}" --argjson x "${entity_x}" --argjson y "${entity_y}" '
    if any(.entities[]?; .id == $id) then
      .entities |= map(if .id == $id then .position = { x: $x, y: $y } else . end)
    else
      error("entity id not found: " + $id)
    end
  ' "${target}" | write_json "${target}"
  validate_scene_file "${target}" >/dev/null
  echo "Moved entity ${entity_id} in ${target}"
}

set_dialogue() {
  local target="$1"
  require_value "--speaker" "${dialogue_speaker}"
  require_value "--text" "${dialogue_text}"

  jq --arg speaker "${dialogue_speaker}" --arg text "${dialogue_text}" '
    .dialogue_speaker = $speaker | .dialogue = $text
  ' "${target}" | write_json "${target}"
  validate_scene_file "${target}" >/dev/null
  echo "Updated dialogue in ${target}"
}

format_scene() {
  local target="$1"
  jq '.' "${target}" | write_json "${target}"
  validate_scene_file "${target}" >/dev/null
  echo "Formatted ${target}"
}

install_fixture() {
  local fixture="$1"
  local scene_dir="${config_home}/gameterm/scenes"
  mkdir -p "${scene_dir}"

  case "${fixture}" in
    basic)
      copy_file "${fixture_root}/default.json" "${scene_dir}/default.json"
      ;;
    navigate)
      copy_file "${fixture_root}/default.json" "${scene_dir}/default.json"
      copy_file "${fixture_root}/memory.json" "${scene_dir}/memory.json"
      ;;
    invalid)
      copy_file "${fixture_root}/invalid.json" "${scene_dir}/default.json"
      ;;
    sprites)
      copy_file "${fixture_root}/default.json" "${scene_dir}/default.json"
      copy_file "${fixture_root}/sprites.json" "${scene_dir}/sprites.json"
      ;;
    missing-sprite)
      copy_file "${fixture_root}/default.json" "${scene_dir}/default.json"
      copy_file "${fixture_root}/sprites-missing.json" "${scene_dir}/sprites.json"
      ;;
    vertical-slice)
      copy_file "${fixture_root}/vertical-slice.json" "${scene_dir}/default.json"
      copy_file "${fixture_root}/sprites.json" "${scene_dir}/sprites.json"
      ;;
    authoring-loop)
      copy_file "${fixture_root}/authoring-loop.json" "${scene_dir}/default.json"
      copy_file "${fixture_root}/sprites.json" "${scene_dir}/sprites.json"
      ;;
    game-states)
      copy_file "${fixture_root}/game-states.json" "${scene_dir}/default.json"
      copy_file "${fixture_root}/sprites.json" "${scene_dir}/sprites.json"
      ;;
    *)
      echo "unknown fixture: ${fixture}" >&2
      usage >&2
      exit 2
      ;;
  esac

  echo
  echo "Installed ${fixture} fixture into ${scene_dir}"
}

case "${command}" in
  validate)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    validate_scene_file "${positionals[0]}"
    ;;
  new-scene)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    create_scene "${positionals[0]}"
    ;;
  new-template)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    create_template "${positionals[0]}"
    ;;
  add-entity)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    add_entity "${positionals[0]}"
    ;;
  add-choice)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    add_choice "${positionals[0]}"
    ;;
  remove-choice)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    remove_choice "${positionals[0]}"
    ;;
  update-choice)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    update_choice "${positionals[0]}"
    ;;
  remove-entity)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    remove_entity "${positionals[0]}"
    ;;
  move-entity)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    move_entity "${positionals[0]}"
    ;;
  set-dialogue)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    set_dialogue "${positionals[0]}"
    ;;
  format)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    format_scene "${positionals[0]}"
    ;;
  install-fixture)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    install_fixture "${positionals[0]}"
    ;;
  list-fixtures)
    if [[ "${#positionals[@]}" -ne 0 ]]; then
      usage >&2
      exit 2
    fi
    printf '%s\n' basic navigate invalid sprites missing-sprite vertical-slice authoring-loop game-states
    ;;
  list-templates)
    if [[ "${#positionals[@]}" -ne 0 ]]; then
      usage >&2
      exit 2
    fi
    printf '%s\n' agent-workflow project-dashboard visual-novel layered-mode rpg-quest vertical-slice
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
