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
  set-variable PATH             Set or add a typed variable.
  clear-variable PATH           Remove a variable by key.
  add-layer PATH                Add a Scene Mode layer.
  set-layer PATH                Update a layer state.
  add-layer-transition PATH     Add a guarded layer transition.
  add-mode-input PATH           Add a guarded mode input binding.
  set-lifecycle PATH            Set Scene Mode lifecycle status hooks.
  add-inventory PATH            Add or replace an RPG inventory item.
  remove-inventory PATH         Remove an RPG inventory item.
  set-stat PATH                 Add or replace an RPG stat.
  adjust-stat PATH              Adjust a numeric RPG stat.
  add-quest PATH                Add or replace an RPG quest.
  advance-quest PATH            Set an RPG quest stage.
  complete-quest PATH           Mark an RPG quest complete.
  append-quest-journal PATH     Append RPG quest journal text.
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

Options for set-variable:
  --key KEY
  --value-bool true|false | --value-number N | --value-text TEXT

Options for clear-variable:
  --key KEY

Options for add-layer:
  --layer-id ID --state STATE
  --label TEXT                  Optional layer label.

Options for set-layer:
  --layer-id ID --state STATE

Options for add-layer-transition:
  --layer-id ID --input INPUT --target-state STATE
  --condition-source SOURCE     Optional guard source. Default: variable.
  --condition-variable KEY      Optional guard variable.
  --condition-bool true|false | --condition-number N | --condition-text TEXT

Options for add-mode-input:
  --input INPUT --action ACTION
  --condition-source SOURCE     Optional guard source. Default: variable.
  --condition-variable KEY      Optional guard variable.
  --condition-bool true|false | --condition-number N | --condition-text TEXT

Options for set-lifecycle:
  --enter-status TEXT           Optional mode enter status.
  --update-status TEXT          Optional mode update status.
  --exit-status TEXT            Optional mode exit status.

Options for add-inventory:
  --item-id ID --label TEXT --count N

Options for remove-inventory:
  --item-id ID

Options for set-stat:
  --key KEY
  --owner-id ID                 Optional stat owner.
  --value-bool true|false | --value-number N | --value-text TEXT

Options for adjust-stat:
  --key KEY --amount N
  --owner-id ID                 Optional stat owner.

Options for add-quest:
  --quest-id ID --label TEXT --stage N
  --journal TEXT                Optional quest journal.

Options for advance-quest:
  --quest-id ID --stage N

Options for complete-quest:
  --quest-id ID

Options for append-quest-journal:
  --quest-id ID --journal TEXT

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
EOF
  print_catalog_csv "${AUTHOR_FIXTURES[@]}"
  cat <<'EOF'

Templates:
EOF
  print_catalog_csv "${AUTHOR_TEMPLATES[@]}"
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="${repo_root}/ci/fixtures/gameterm-scene"
AUTHOR_FIXTURES=(
  basic
  navigate
  invalid
  sprites
  missing-sprite
  vertical-slice
  authoring-loop
  game-states
  chained-transitions
  workspace-agent
  multi-agent-coordination
)
AUTHOR_TEMPLATES=(
  agent-workflow
  project-dashboard
  visual-novel
  layered-mode
  rpg-quest
  vertical-slice
  workspace-agent
  multi-agent-coordination
)

print_catalog_csv() {
  local line="  "
  local item
  local separator=""

  for item in "$@"; do
    if [[ $(( ${#line} + ${#separator} + ${#item} )) -gt 78 ]]; then
      printf '%s\n' "${line},"
      line="  "
      separator=""
    fi
    line+="${separator}${item}"
    separator=", "
  done
  printf '%s\n' "${line}"
}

print_catalog_lines() {
  printf '%s\n' "$@"
}

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
state_key=""
value_bool=""
value_number=""
value_text=""
layer_id=""
layer_state=""
layer_label=""
transition_input=""
transition_target_state=""
mode_action=""
condition_source=""
condition_variable=""
condition_bool=""
condition_number=""
condition_text=""
lifecycle_enter_status=""
lifecycle_update_status=""
lifecycle_exit_status=""
rpg_label=""
item_id=""
item_count=""
owner_id=""
amount=""
quest_id=""
quest_stage=""
quest_journal=""
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
      elif [[ "${command}" == "add-layer" ]]; then
        layer_label="$2"
      elif [[ "${command}" == "add-inventory" || "${command}" == "add-quest" ]]; then
        rpg_label="$2"
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
      if [[ "${command}" == "set-variable" || "${command}" == "add-layer-transition" ]]; then
        value_text="$2"
      else
        dialogue_text="$2"
      fi
      shift 2
      ;;
    --key)
      state_key="$2"
      shift 2
      ;;
    --value-bool)
      value_bool="$2"
      shift 2
      ;;
    --value-number)
      value_number="$2"
      shift 2
      ;;
    --value-text)
      value_text="$2"
      shift 2
      ;;
    --layer-id)
      layer_id="$2"
      shift 2
      ;;
    --state)
      layer_state="$2"
      shift 2
      ;;
    --input)
      transition_input="$2"
      shift 2
      ;;
    --action)
      mode_action="$2"
      shift 2
      ;;
    --target-state)
      transition_target_state="$2"
      shift 2
      ;;
    --condition-source)
      condition_source="$2"
      shift 2
      ;;
    --condition-variable)
      condition_variable="$2"
      shift 2
      ;;
    --condition-bool)
      condition_bool="$2"
      shift 2
      ;;
    --condition-number)
      condition_number="$2"
      shift 2
      ;;
    --condition-text)
      condition_text="$2"
      shift 2
      ;;
    --item-id)
      item_id="$2"
      shift 2
      ;;
    --count)
      item_count="$2"
      shift 2
      ;;
    --owner-id)
      owner_id="$2"
      shift 2
      ;;
    --amount)
      amount="$2"
      shift 2
      ;;
    --quest-id)
      quest_id="$2"
      shift 2
      ;;
    --stage)
      quest_stage="$2"
      shift 2
      ;;
    --journal)
      quest_journal="$2"
      shift 2
      ;;
    --enter-status)
      lifecycle_enter_status="$2"
      shift 2
      ;;
    --update-status)
      lifecycle_update_status="$2"
      shift 2
      ;;
    --exit-status)
      lifecycle_exit_status="$2"
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

  require_overwrite_allowed "${target}" || return 1
  cp "${source}" "${target}"
  echo "Wrote ${target}"
}

require_overwrite_allowed() {
  local target="$1"
  if [[ -e "${target}" && "${force}" -ne 1 ]]; then
    cat >&2 <<EOF
${target} already exists.

Rerun with --force to overwrite it.
EOF
    return 1
  fi
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

typed_value_json() {
  local bool_value="$1"
  local number_value="$2"
  local text_value="$3"
  local one_of_message="$4"
  local bool_error="$5"
  local number_error="$6"
  local count=0

  [[ -n "${bool_value}" ]] && count=$((count + 1))
  [[ -n "${number_value}" ]] && count=$((count + 1))
  [[ -n "${text_value}" ]] && count=$((count + 1))
  if [[ "${count}" -ne 1 ]]; then
    echo "${one_of_message}" >&2
    exit 2
  fi
  if [[ -n "${bool_value}" ]]; then
    case "${bool_value}" in
      true|false) jq -n --argjson value "${bool_value}" '{Bool: $value}' ;;
      *)
        echo "${bool_error}" >&2
        exit 2
        ;;
    esac
  elif [[ -n "${number_value}" ]]; then
    if [[ ! "${number_value}" =~ ^-?[0-9]+$ ]]; then
      echo "${number_error}" >&2
      exit 2
    fi
    jq -n --argjson value "${number_value}" '{Number: $value}'
  else
    jq -n --arg value "${text_value}" '{Text: $value}'
  fi
}

state_value_json() {
  typed_value_json \
    "$1" \
    "$2" \
    "$3" \
    "provide exactly one of --value-bool, --value-number, or --value-text" \
    "--value-bool must be true or false" \
    "--value-number must be an integer"
}

condition_value_json() {
  typed_value_json \
    "${condition_bool}" \
    "${condition_number}" \
    "${condition_text}" \
    "condition requires exactly one of --condition-bool, --condition-number, or --condition-text" \
    "--condition-bool must be true or false" \
    "--condition-number must be an integer"
}

condition_json() {
  local condition_value
  condition_value="$(condition_value_json)"
  jq -n \
    --arg source "${condition_source}" \
    --arg variable "${condition_variable}" \
    --argjson equals "${condition_value}" \
    '[{
      source: (if $source == "" or $source == "variable" then null else $source end),
      variable: $variable,
      equals: $equals
    } | with_entries(select(.value != null))]'
}

optional_conditions_json() {
  if [[ -n "${condition_variable}" ]]; then
    condition_json
  else
    printf '[]\n'
  fi
}

mode_default_filter() {
  cat <<'EOF'
(.mode // {
  mode_id: "workspace",
  label: "Workspace",
  description: "",
  scene_profile: "scene",
  allowed_actions: []
})
EOF
}

validate_scene_file() {
  cargo run -q -p gameterm-visual --example scene_validate -- "$1"
}

write_validated_json() {
  local target="$1"
  local tmp
  tmp="$(mktemp /tmp/gameterm-scene-author.XXXXXX)"
  cat >"${tmp}"
  validate_scene_file "${tmp}" >/dev/null
  mv "${tmp}" "${target}"
}

create_scene() {
  local target="$1"
  require_overwrite_allowed "${target}" || return 1

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
    }' | write_validated_json "${target}"
  echo "Wrote ${target}"
}

create_template() {
  local target="$1"
  require_overwrite_allowed "${target}" || return 1

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
      }' | write_validated_json "${target}"
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
      }' | write_validated_json "${target}"
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
      }' | write_validated_json "${target}"
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
      }' | write_validated_json "${target}"
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
            id: "player",
            kind: "Agent",
            label: "Player",
            position: { x: 3, y: 3 },
            sprite: "agent_idle",
            state_flags: ["player"],
            metadata: [["relationship_role", "source"]]
          },
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
      }' | write_validated_json "${target}"
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
            id: "player",
            kind: "Agent",
            label: "Player",
            position: { x: 5, y: 4 },
            sprite: "agent_idle",
            state_flags: ["player"],
            metadata: [["relationship_role", "source"]]
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
      }' | write_validated_json "${target}"
      ;;
    workspace-agent)
      jq '.' "${fixture_root}/workspace-agent.json" | write_validated_json "${target}"
      ;;
    multi-agent-coordination)
      jq '.' "${fixture_root}/multi-agent-coordination.json" | write_validated_json "${target}"
      ;;
    *)
      echo "unknown template: ${template_name}" >&2
      usage >&2
      exit 2
      ;;
  esac

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
    }]' "${target}" | write_validated_json "${target}"
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
        "${target}" | write_validated_json "${target}"
      ;;
    OpenFile)
      require_value "--open-file" "${choice_payload}"
      jq --arg label "${choice_label}" --arg path "${choice_payload}" \
        '.choices += [{ label: $label, kind: { OpenFile: { path: $path } } }]' \
        "${target}" | write_validated_json "${target}"
      ;;
    Navigate)
      require_value "--navigate" "${choice_payload}"
      jq --arg label "${choice_label}" --arg target_path "${choice_payload}" \
        '.choices += [{ label: $label, kind: { Navigate: { target: $target_path } } }]' \
        "${target}" | write_validated_json "${target}"
      ;;
    RunCommand)
      require_value "--run-argv" "${choice_payload}"
      if [[ -n "${choice_cwd}" ]]; then
        jq --arg label "${choice_label}" \
          --argjson argv "${choice_payload}" \
          --arg cwd "${choice_cwd}" \
          --arg target "${choice_target}" \
          '.choices += [{ label: $label, kind: { RunCommand: { argv: $argv, cwd: $cwd, target: $target } } }]' \
          "${target}" | write_validated_json "${target}"
      else
        jq --arg label "${choice_label}" \
          --argjson argv "${choice_payload}" \
          --arg target "${choice_target}" \
          '.choices += [{ label: $label, kind: { RunCommand: { argv: $argv, target: $target } } }]' \
          "${target}" | write_validated_json "${target}"
      fi
      ;;
  esac

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
  ' "${target}" | write_validated_json "${target}"
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
        "${target}" | write_validated_json "${target}"
    else
      jq --argjson index "${choice_index}" \
        --arg label "${choice_label}" \
        --argjson argv "${choice_payload}" \
        --arg target "${choice_target}" \
        --arg payload "${choice_payload}" \
        "if (.choices | has(\$index)) then .choices[\$index] = ${filter} else error(\"choice index not found: \" + (\$index | tostring)) end" \
        "${target}" | write_validated_json "${target}"
    fi
  else
    jq --argjson index "${choice_index}" \
      --arg label "${choice_label}" \
      --arg payload "${choice_payload}" \
      "if (.choices | has(\$index)) then .choices[\$index] = ${filter} else error(\"choice index not found: \" + (\$index | tostring)) end" \
      "${target}" | write_validated_json "${target}"
  fi

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
  ' "${target}" | write_validated_json "${target}"
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
  ' "${target}" | write_validated_json "${target}"
  echo "Moved entity ${entity_id} in ${target}"
}

set_dialogue() {
  local target="$1"
  require_value "--speaker" "${dialogue_speaker}"
  require_value "--text" "${dialogue_text}"

  jq --arg speaker "${dialogue_speaker}" --arg text "${dialogue_text}" '
    .dialogue_speaker = $speaker | .dialogue = $text
  ' "${target}" | write_validated_json "${target}"
  echo "Updated dialogue in ${target}"
}

set_variable() {
  local target="$1"
  local value_json
  require_value "--key" "${state_key}"
  value_json="$(state_value_json "${value_bool}" "${value_number}" "${value_text}")"

  jq --arg key "${state_key}" --argjson value "${value_json}" '
    .variables = (((.variables // []) | map(select(.key != $key)))
      + [{ key: $key, value: $value }])
  ' "${target}" | write_validated_json "${target}"
  echo "Set variable ${state_key} in ${target}"
}

clear_variable() {
  local target="$1"
  require_value "--key" "${state_key}"

  jq --arg key "${state_key}" '
    .variables = ((.variables // []) | map(select(.key != $key)))
  ' "${target}" | write_validated_json "${target}"
  echo "Cleared variable ${state_key} in ${target}"
}

add_layer() {
  local target="$1"
  require_value "--layer-id" "${layer_id}"
  require_value "--state" "${layer_state}"

  jq \
    --arg layer_id "${layer_id}" \
    --arg state "${layer_state}" \
    --arg label "${layer_label}" '
    if any((.layers // [])[]; .layer_id == $layer_id) then
      error("layer id already exists: " + $layer_id)
    else
      .layers = ((.layers // []) + [{
        layer_id: $layer_id,
        state: $state,
        label: (if $label == "" then null else $label end),
        input_map: [],
        transitions: []
      } | with_entries(select(.value != null))])
    end
  ' "${target}" | write_validated_json "${target}"
  echo "Added layer ${layer_id} to ${target}"
}

set_layer() {
  local target="$1"
  require_value "--layer-id" "${layer_id}"
  require_value "--state" "${layer_state}"

  jq --arg layer_id "${layer_id}" --arg state "${layer_state}" '
    if any((.layers // [])[]; .layer_id == $layer_id) then
      .layers |= map(if .layer_id == $layer_id then .state = $state else . end)
    else
      error("layer id not found: " + $layer_id)
    end
  ' "${target}" | write_validated_json "${target}"
  echo "Set layer ${layer_id} state in ${target}"
}

add_layer_transition() {
  local target="$1"
  local conditions_json
  require_value "--layer-id" "${layer_id}"
  require_value "--input" "${transition_input}"
  require_value "--target-state" "${transition_target_state}"

  conditions_json="$(optional_conditions_json)"

  jq \
    --arg layer_id "${layer_id}" \
    --arg input "${transition_input}" \
    --arg target_state "${transition_target_state}" \
    --argjson conditions "${conditions_json}" '
    if any((.layers // [])[]; .layer_id == $layer_id) then
      .layers |= map(if .layer_id == $layer_id then
        .transitions = ((.transitions // []) + [{
          input: $input,
          target_state: $target_state,
          conditions: $conditions
        }])
      else . end)
    else
      error("layer id not found: " + $layer_id)
    end
  ' "${target}" | write_validated_json "${target}"
  echo "Added layer transition ${layer_id}:${transition_input} to ${target}"
}

add_mode_input() {
  local target="$1"
  local conditions_json
  local mode_default
  require_value "--input" "${transition_input}"
  require_value "--action" "${mode_action}"

  conditions_json="$(optional_conditions_json)"
  mode_default="$(mode_default_filter)"

  jq \
    --arg input "${transition_input}" \
    --arg action "${mode_action}" \
    --argjson conditions "${conditions_json}" "
    .mode = ${mode_default}
    | .mode.input_map = ((.mode.input_map // []) + [{
      input: \$input,
      action: \$action,
      conditions: \$conditions
    }])
  " "${target}" | write_validated_json "${target}"
  echo "Added mode input ${transition_input}->${mode_action} to ${target}"
}

set_lifecycle() {
  local target="$1"
  local mode_default
  if [[ -z "${lifecycle_enter_status}" && -z "${lifecycle_update_status}" && -z "${lifecycle_exit_status}" ]]; then
    echo "provide at least one of --enter-status, --update-status, or --exit-status" >&2
    exit 2
  fi
  mode_default="$(mode_default_filter)"

  jq \
    --arg enter_status "${lifecycle_enter_status}" \
    --arg update_status "${lifecycle_update_status}" \
    --arg exit_status "${lifecycle_exit_status}" "
    .mode = ${mode_default}
    | .mode.lifecycle = (.mode.lifecycle // {})
    | if \$enter_status != \"\" then .mode.lifecycle.enter_status = \$enter_status else . end
    | if \$update_status != \"\" then .mode.lifecycle.update_status = \$update_status else . end
    | if \$exit_status != \"\" then .mode.lifecycle.exit_status = \$exit_status else . end
  " "${target}" | write_validated_json "${target}"
  echo "Updated mode lifecycle in ${target}"
}

add_inventory() {
  local target="$1"
  require_value "--item-id" "${item_id}"
  require_value "--label" "${rpg_label}"
  require_value "--count" "${item_count}"
  if [[ ! "${item_count}" =~ ^[0-9]+$ ]]; then
    echo "--count must be a non-negative integer" >&2
    exit 2
  fi

  jq \
    --arg item_id "${item_id}" \
    --arg label "${rpg_label}" \
    --argjson count "${item_count}" '
    .rpg = (.rpg // {})
    | .rpg.inventory = (((.rpg.inventory // []) | map(select(.item_id != $item_id)))
      + [{ item_id: $item_id, label: $label, count: $count }])
  ' "${target}" | write_validated_json "${target}"
  echo "Added inventory item ${item_id} to ${target}"
}

remove_inventory() {
  local target="$1"
  require_value "--item-id" "${item_id}"

  jq --arg item_id "${item_id}" '
    .rpg = (.rpg // {})
    | .rpg.inventory = ((.rpg.inventory // []) | map(select(.item_id != $item_id)))
  ' "${target}" | write_validated_json "${target}"
  echo "Removed inventory item ${item_id} from ${target}"
}

set_stat() {
  local target="$1"
  local value_json
  require_value "--key" "${state_key}"
  value_json="$(state_value_json "${value_bool}" "${value_number}" "${value_text}")"

  jq \
    --arg owner_id "${owner_id}" \
    --arg key "${state_key}" \
    --argjson value "${value_json}" '
    .rpg = (.rpg // {})
    | .rpg.stats = (((.rpg.stats // [])
      | map(select(.key != $key or ((.owner_id // "") != $owner_id))))
      + [{
        owner_id: (if $owner_id == "" then null else $owner_id end),
        key: $key,
        value: $value
      } | with_entries(select(.value != null))])
  ' "${target}" | write_validated_json "${target}"
  echo "Set stat ${state_key} in ${target}"
}

adjust_stat() {
  local target="$1"
  require_value "--key" "${state_key}"
  require_value "--amount" "${amount}"
  if [[ ! "${amount}" =~ ^-?[0-9]+$ ]]; then
    echo "--amount must be an integer" >&2
    exit 2
  fi

  jq \
    --arg owner_id "${owner_id}" \
    --arg key "${state_key}" \
    --argjson amount "${amount}" '
    if any((.rpg.stats // [])[]; .key == $key and ((.owner_id // "") == $owner_id)) then
      .rpg.stats |= map(if .key == $key and ((.owner_id // "") == $owner_id) then
        if (.value | has("Number")) then
          .value.Number += $amount
        else
          error("stat is not numeric: " + $key)
        end
      else . end)
    else
      error("stat key not found: " + $key)
    end
  ' "${target}" | write_validated_json "${target}"
  echo "Adjusted stat ${state_key} in ${target}"
}

add_quest() {
  local target="$1"
  require_value "--quest-id" "${quest_id}"
  require_value "--label" "${rpg_label}"
  require_value "--stage" "${quest_stage}"
  if [[ ! "${quest_stage}" =~ ^-?[0-9]+$ ]]; then
    echo "--stage must be an integer" >&2
    exit 2
  fi

  jq \
    --arg quest_id "${quest_id}" \
    --arg label "${rpg_label}" \
    --argjson stage "${quest_stage}" \
    --arg journal "${quest_journal}" '
    .rpg = (.rpg // {})
    | .rpg.quests = (((.rpg.quests // []) | map(select(.quest_id != $quest_id)))
      + [{
        quest_id: $quest_id,
        label: $label,
        stage: $stage,
        completed: false,
        journal: $journal
      }])
  ' "${target}" | write_validated_json "${target}"
  echo "Added quest ${quest_id} to ${target}"
}

advance_quest() {
  local target="$1"
  require_value "--quest-id" "${quest_id}"
  require_value "--stage" "${quest_stage}"
  if [[ ! "${quest_stage}" =~ ^-?[0-9]+$ ]]; then
    echo "--stage must be an integer" >&2
    exit 2
  fi

  jq --arg quest_id "${quest_id}" --argjson stage "${quest_stage}" '
    if any((.rpg.quests // [])[]; .quest_id == $quest_id) then
      .rpg.quests |= map(if .quest_id == $quest_id then .stage = $stage else . end)
    else
      error("quest id not found: " + $quest_id)
    end
  ' "${target}" | write_validated_json "${target}"
  echo "Advanced quest ${quest_id} in ${target}"
}

complete_quest() {
  local target="$1"
  require_value "--quest-id" "${quest_id}"

  jq --arg quest_id "${quest_id}" '
    if any((.rpg.quests // [])[]; .quest_id == $quest_id) then
      .rpg.quests |= map(if .quest_id == $quest_id then .completed = true else . end)
    else
      error("quest id not found: " + $quest_id)
    end
  ' "${target}" | write_validated_json "${target}"
  echo "Completed quest ${quest_id} in ${target}"
}

append_quest_journal() {
  local target="$1"
  require_value "--quest-id" "${quest_id}"
  require_value "--journal" "${quest_journal}"

  jq --arg quest_id "${quest_id}" --arg journal "${quest_journal}" '
    if any((.rpg.quests // [])[]; .quest_id == $quest_id) then
      .rpg.quests |= map(if .quest_id == $quest_id then
        .journal = (if (.journal // "") == "" then $journal else (.journal + "\n" + $journal) end)
      else . end)
    else
      error("quest id not found: " + $quest_id)
    end
  ' "${target}" | write_validated_json "${target}"
  echo "Appended quest journal ${quest_id} in ${target}"
}

format_scene() {
  local target="$1"
  jq '.' "${target}" | write_validated_json "${target}"
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
    chained-transitions)
      copy_file "${fixture_root}/chained-transitions.json" "${scene_dir}/default.json"
      copy_file "${fixture_root}/sprites.json" "${scene_dir}/sprites.json"
      ;;
    workspace-agent)
      copy_file "${fixture_root}/workspace-agent.json" "${scene_dir}/default.json"
      copy_file "${fixture_root}/sprites.json" "${scene_dir}/sprites.json"
      ;;
    multi-agent-coordination)
      copy_file "${fixture_root}/multi-agent-coordination.json" "${scene_dir}/default.json"
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
  set-variable)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    set_variable "${positionals[0]}"
    ;;
  clear-variable)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    clear_variable "${positionals[0]}"
    ;;
  add-layer)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    add_layer "${positionals[0]}"
    ;;
  set-layer)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    set_layer "${positionals[0]}"
    ;;
  add-layer-transition)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    add_layer_transition "${positionals[0]}"
    ;;
  add-mode-input)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    add_mode_input "${positionals[0]}"
    ;;
  set-lifecycle)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    set_lifecycle "${positionals[0]}"
    ;;
  add-inventory)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    add_inventory "${positionals[0]}"
    ;;
  remove-inventory)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    remove_inventory "${positionals[0]}"
    ;;
  set-stat)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    set_stat "${positionals[0]}"
    ;;
  adjust-stat)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    adjust_stat "${positionals[0]}"
    ;;
  add-quest)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    add_quest "${positionals[0]}"
    ;;
  advance-quest)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    advance_quest "${positionals[0]}"
    ;;
  complete-quest)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    complete_quest "${positionals[0]}"
    ;;
  append-quest-journal)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    append_quest_journal "${positionals[0]}"
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
    print_catalog_lines "${AUTHOR_FIXTURES[@]}"
    ;;
  list-templates)
    if [[ "${#positionals[@]}" -ne 0 ]]; then
      usage >&2
      exit 2
    fi
    print_catalog_lines "${AUTHOR_TEMPLATES[@]}"
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
