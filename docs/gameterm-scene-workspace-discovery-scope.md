# GameTerm Scene Mode Workspace Discovery Scope

This document scopes the next Scene Mode product layer after the
Agent/Workspace first pass: Workspace Discovery.

The Agent/Workspace slice proved that Scene Mode can represent workspace,
project, task, agent, process, and file entities when they are authored in a
fixture. Workspace Discovery turns that authored model into a live GameTerm
workflow by deriving a useful scene or patch from the user's current workspace.

## Purpose

Workspace Discovery should make Scene Mode reflect the real work context the
user is already in.

The purpose is not to build a full IDE, crawler, project indexer, or autonomous
agent system. The purpose is to answer:

> What workspace am I in, what important local artifacts are visible, and what
> explicit actions can I take from Scene Mode?

The first pass should discover a conservative, bounded snapshot of workspace
state and express it through the existing Scene Mode contracts:

- scene entities
- entity metadata
- variables
- layers
- explicit `OpenFile` choices
- explicit `RunCommand` choices
- optional patch transport
- smoke-verifiable generated output

## End Goal

The end goal is a first-pass discovery path where a user can run one command
from a real repo and get a valid Scene Mode workspace view.

Target experience:

1. User is inside a project directory.
2. User runs a discovery helper.
3. The helper detects the workspace root, git branch/status when available,
   important files, and reasonable verification commands.
4. The helper writes either:
   - a complete Scene Mode scene file, or
   - a patch for an existing Agent/Workspace scene.
5. User opens Scene Mode.
6. Scene Mode shows a live-derived workspace/project/task/file map.
7. User can inspect entities, open important files, and run explicit commands.
8. Generated output validates before it is installed or submitted.
9. Failures are visible and do not mutate existing scene files unless the user
   explicitly requests an install/write path.

The end state is "a generated workspace view that is useful immediately," not
"perfect automatic understanding of the whole repo."

## Alignment With GameTerm Goals

Workspace Discovery aligns with GameTerm's direction in four ways.

1. It keeps Scene Mode terminal-native.
   Discovery starts from cwd, panes, files, git, and commands. The terminal
   remains the source of truth; Scene Mode becomes a structured visual surface.

2. It turns authored state into live state.
   The `workspace-agent` fixture proved the model. Discovery makes the same
   model reflect the user's actual work.

3. It preserves explicit action boundaries.
   Discovery may suggest commands, but commands only run through explicit
   `RunCommand` choices. Generated patches must not execute anything.

4. It creates a bridge to later agent and memory features.
   Agents need a workspace model to act on. Memory and relationship views need
   entities and references. Discovery provides that substrate without requiring
   a full productized memory graph.

## Product Boundaries

### In Scope

- cwd/root discovery
- git repo root, branch, and dirty/clean status
- selected or configured project files
- generated workspace/project/task/file entities
- optional process/task entity for verification
- generated variables and layer state
- explicit open-file actions
- explicit verification command actions
- dry-run output
- safe install output
- static validation of generated scenes/patches
- deterministic fixture coverage
- live smoke scenario when the helper is stable

### Out Of Scope

- recursive full-repo indexing
- semantic code understanding
- hidden command execution
- automatic file edits
- automatic agent execution
- background daemons
- network calls
- remote workspace discovery
- multi-repo graphing
- persistent session database
- replacing existing shell/editor workflows
- inferring task intent from terminal output

## Discovery Inputs

The first pass should use inputs that are cheap, local, and predictable.

### Required Input

- `--cwd PATH`
  - Defaults to current directory.
  - Must resolve to an existing directory.

### Optional Inputs

- `--scene-output PATH`
  - Write a complete generated scene.

- `--patch-output PATH`
  - Write a patch for an existing Agent/Workspace scene.

- `--install`
  - Install generated scene to `~/.config/gameterm/scenes/default.json`.
  - Must validate before replacing.
  - Should require `--force` if the target exists.

- `--inbox PATH`
  - Write generated patch to a Scene Mode patch inbox.

- `--title TEXT`
  - Override generated scene title.

- `--task TEXT`
  - Name an active task entity.

- `--verify-argv JSON_ARRAY`
  - Add an explicit verification `RunCommand` choice.

- `--open PATH`
  - Add an explicit file entity and open-file choice.
  - Can be repeated.

- `--max-files N`
  - Bound discovered file entities.

- `--config PATH`
  - Optional local config for discovery hints.

### Environment Inputs

The helper may read:

- current working directory
- git command output
- `XDG_CONFIG_HOME`
- known repo files
- local file metadata

The helper must not require:

- network access
- an active GUI
- a running mux server
- a running agent

## Discovery Sources

### CWD

Purpose: identify the active workspace.

Rules:

- resolve symlinks where practical
- reject missing directories
- use cwd basename as fallback project label
- keep paths relative when writing portable fixtures, absolute when installing
  a user-local scene if that improves open-file behavior

### Git

Purpose: identify repo context without expensive crawling.

Fields:

- repo root
- branch name
- short revision
- clean/dirty status
- count of changed files

Commands:

- `git rev-parse --show-toplevel`
- `git branch --show-current`
- `git rev-parse --short HEAD`
- `git status --porcelain`

Rules:

- if cwd is not a git repo, still generate a workspace scene
- git failures should become metadata/status, not hard failures, unless the
  user explicitly requires git
- do not parse large diffs

### Important Files

Purpose: show files that help users orient and act.

Default candidate order:

1. `README.md`
2. `AGENTS.md`
3. `CODING_PRINCIPLES.md`
4. `docs/gameterm-scene-workspace-discovery-scope.md`
5. `docs/gameterm-scene-agent-workspace-scope.md`
6. `docs/gameterm-scene-runtime-roadmap.md`
7. `Cargo.toml`
8. `package.json`
9. `pyproject.toml`
10. `.gitignore`

Rules:

- include only files that exist
- cap count with `--max-files`
- preserve user-provided `--open` entries ahead of auto-detected entries
- report missing user-provided files as warnings unless `--strict` is passed

### Verification Commands

Purpose: expose explicit, useful commands without running them.

Default detection:

- Rust workspace: `cargo test -p gameterm-visual scene_fixture_workspace_agent_completes_product_loop`
- GameTerm scene repo: `ci/gameterm-scene-verify.sh --fixture workspace-agent`
- Node package: `npm test` only if package scripts include `test`
- Python project: no default command unless configured

Rules:

- generated commands must be choices, not automatic execution
- commands must include explicit argv arrays
- cwd must be the discovered root
- doctor/validation should flag missing cwd

### Pane And Process Metadata

Purpose: future bridge to real GameTerm panes.

First-pass status:

- implemented through explicit pane/process metadata inputs
- live mux auto-discovery remains deferred

Supported inputs:

- active mux window id
- active pane id
- pane cwd
- foreground process name
- foreground process path
- pane progress state

Rules:

- do not block first pass on mux integration
- represent unknown pane/process fields as absent metadata
- when added, keep this data as metadata/variables unless it must drive guards
- use pane cwd as the discovery cwd only when `--cwd` is not supplied

## Output Contracts

Workspace Discovery should support two output modes.

### Generated Scene

Purpose: create a complete standalone scene that can be installed or opened.

Shape:

- title: discovered workspace name
- mode: `workspace`
- layers:
  - `workspace: overview`
  - `agent: idle`
  - `process: none`
  - `ui: scene`
- variables:
  - `workspace_mode`
  - `workspace_root`
  - `repo_branch`
  - `repo_status`
  - `active_task_id`
  - `process_phase`
  - `verification_status`
- entities:
  - workspace/project anchor
  - discovered project/repo entity
  - active task entity
  - verification process entity
  - file entities
- choices:
  - inspect workspace
  - open important files
  - run explicit verification command
  - optionally install/reload docs only through existing user action paths

Acceptance:

- generated scene validates with `ci/gameterm-scene-author.sh validate`
- generated scene can be installed as `default.json`
- no generated choice runs without explicit activation

### Generated Patch

Purpose: update an existing Agent/Workspace scene while Scene Mode is open.

Patch fields:

- status
- selected entity id
- entity updates
- variables
- optional process state only when representing a known command result

Rules:

- patch output should target entities that exist in the base
  `workspace-agent` scene unless `--allow-new-entities` is introduced later
- failed patch validation must not write to inbox
- patch mode should be secondary to scene generation for first pass

Acceptance:

- generated patch validates against `workspace-agent`
- inbox write is atomic through existing patch helper behavior
- missing entity errors are visible before inbox write

## Entity Model

The first pass should reuse existing entity kinds.

### Workspace Entity

Kind: `Project`

Metadata:

- `entity_type=workspace`
- `root=<path>`
- `repo_root=<path>`
- `repo_branch=<branch>`
- `repo_revision=<short-sha>`
- `repo_status=clean|dirty|unknown|not_git`
- `changed_files=<count>`

Flags:

- `workspace`
- `active`
- `git_clean` or `git_dirty` when known

### Project Entity

Kind: `Project`

Metadata:

- `entity_type=project`
- `root=<path>`
- `language=<rust|node|python|unknown>`
- `manifest=<path>`
- `verification=<command label>`

Flags:

- `project`
- `active`

### Task Entity

Kind: `Task`

Metadata:

- `entity_type=task`
- `objective=<task text>`
- `owner=user|agent|unknown`
- `phase=discovered|planned|running|blocked|complete`
- `related_files=<csv or concise list>`

Flags:

- `task`
- `discovered`

### Process Entity

Kind: `Task`

Metadata:

- `entity_type=process`
- `command=<display command>`
- `cwd=<path>`
- `phase=none`

Flags:

- `process`
- `idle`

### File Entity

Kind: `Memory` for docs/context files.

Kind: `Principle` for principle/config files.

Metadata:

- `entity_type=file`
- `path=<path>`
- `role=readme|principles|roadmap|manifest|config|source|doc`
- `status=present|missing`

Flags:

- `file`
- `<role>`

## Variables

Initial variables:

- `workspace_mode = "overview"`
- `workspace_root = "<path>"`
- `repo_branch = "<branch|unknown>"`
- `repo_status = "clean|dirty|unknown|not_git"`
- `active_task_id = "<id>"`
- `process_phase = "none"`
- `verification_status = "not_run"`
- `discovery_source = "cwd"`
- `discovered_file_count = <number>`

Rules:

- variables that drive guards must be small and typed
- large lists belong in metadata
- paths may be text variables only when useful to debugger/status output

## Layers

Initial layers:

- `workspace: overview`
- `agent: idle`
- `process: none`
- `ui: scene`

Later layers:

- `workspace: inspect`
- `workspace: review`
- `process: running`
- `process: succeeded`
- `process: failed`

First pass should not require new runtime layer behavior. It should generate
valid layer data using the existing model.

## User Workflows

### Workflow 1: Generate A Workspace Scene

Command:

```sh
ci/gameterm-scene-workspace.sh discover \
  --cwd /path/to/repo \
  --scene-output /tmp/gameterm-workspace.json
```

Expected behavior:

1. Helper discovers cwd/repo state.
2. Helper writes a complete scene.
3. Helper validates the scene.
4. User can inspect or install the scene.

Acceptance:

- output is valid JSON
- scene validates
- no user config is modified

### Workflow 2: Install A Workspace Scene

Command:

```sh
ci/gameterm-scene-workspace.sh discover \
  --cwd /path/to/repo \
  --install \
  --force
```

Expected behavior:

1. Helper generates to a temporary file.
2. Helper validates.
3. Helper backs up or safely replaces `default.json` only after validation.
4. User opens Scene Mode and sees the discovered workspace.

Acceptance:

- failed generation leaves existing scene untouched
- installed scene includes open-file and verification choices

### Workflow 3: Patch Active Scene

Command:

```sh
ci/gameterm-scene-workspace.sh patch \
  --cwd /path/to/repo \
  --base ci/fixtures/gameterm-scene/workspace-agent.json \
  --patch-output /tmp/gameterm-workspace.patch.json
```

Expected behavior:

1. Helper generates patchable metadata updates.
2. Helper validates against the base scene.
3. User can write the patch to an inbox.

Acceptance:

- patch validation catches missing entities before inbox write
- status explains discovered repo state

### Workflow 4: Run From Scene Mode

Expected behavior:

1. User opens generated scene.
2. User selects a file entity.
3. User activates an open-file choice.
4. User activates explicit verification command only when desired.

Acceptance:

- no command runs automatically
- missing file/cwd status is visible

## Proposed Implementation Lanes

### Lane 1: Scope And Contract

Deliverables:

- this scope document
- roadmap link
- first-pass entity/variable/layer contracts

Acceptance:

- next commits can map to a scoped lane

Commit:

- `[docs] scope Scene Workspace Discovery layer`

### Lane 2: Discovery Helper Skeleton

Deliverables:

- `ci/gameterm-scene-workspace.sh`
- commands:
  - `discover`
  - `patch`
  - `inspect`
- usage/help
- argument validation
- dry-run output

Acceptance:

- `bash -n` passes
- invalid cwd fails clearly
- helper can print discovered cwd and git summary

Commit:

- `[tools] add Scene workspace discovery helper`

### Lane 3: Generated Scene Output

Deliverables:

- scene JSON generation
- workspace/project/task/process/file entities
- variables/layers
- open-file choices
- explicit verification command choice
- validation before write

Acceptance:

- generated scene validates
- no existing files are overwritten without `--force`
- missing optional git state does not fail generation

Commit:

- `[tools] generate Scene workspace discovery scene`

### Lane 4: Install And Rollback Safety

Deliverables:

- safe install path
- temp-file generation before replace
- overwrite protection
- optional backup path or clear status output

Acceptance:

- failed validation leaves target unchanged
- install refuses overwrite without `--force`

Commit:

- `[tools] add Scene workspace discovery install path`

### Lane 5: Patch Output

Deliverables:

- patch generation for base `workspace-agent` scene
- validation before write
- optional inbox write
- selected entity/status update

Acceptance:

- generated patch validates against base fixture
- inbox write only occurs after validation
- missing base entity fails visibly

Commit:

- `[tools] add Scene workspace discovery patches`

### Lane 6: Verification

Deliverables:

- `ci/gameterm-scene-verify.sh` coverage
- deterministic temp repo fixture
- git and non-git checks
- generated scene validation
- generated patch validation
- overwrite failure check

Acceptance:

- `ci/gameterm-scene-verify.sh --all` covers discovery helper
- tests use temporary directories only

Commit:

- `[tools] verify Scene workspace discovery`

### Lane 7: Smoke And Docs

Deliverables:

- named smoke scenario, likely `workspace-discovery`
- user-facing docs in Scene Mode guide
- smoke report entry after live pass
- product smoke checklist update

Acceptance:

- `--describe-scenario workspace-discovery` explains setup and expected status
- live smoke can launch generated scene
- smoke report records result

Commit:

- `[docs] document Scene workspace discovery workflow`

## First Shippable Slice

The first shippable slice should be narrow:

1. Helper command exists.
2. Helper discovers cwd.
3. Helper detects git root/branch/status when available.
4. Helper detects up to five important files.
5. Helper generates a complete Scene Mode scene.
6. Generated scene validates.
7. Generated scene includes explicit open-file choices.
8. Generated scene includes one explicit verification command when detectable
   or provided.
9. Helper can install with validation and overwrite protection.
10. Verifier covers git and non-git temp workspaces.
11. Docs explain generation, install, and boundaries.

Patch output can ship after scene generation if needed. It should not block the
first useful discovery path.

## Acceptance Criteria

Workspace Discovery first pass is complete when:

1. The scope is documented and linked from the roadmap.
2. A real workspace can generate a valid Scene Mode scene.
3. A non-git directory can generate a valid Scene Mode scene.
4. Generated scenes include workspace, project, task, process, and file
   entities where applicable.
5. Generated scenes include guard-friendly variables and active layers.
6. Generated open-file choices point to existing files or report missing files
   during verification.
7. Generated command choices are explicit and never run during discovery.
8. Install path validates before writing.
9. Failed install leaves existing scene untouched.
10. Static verifier covers the helper.
11. Live smoke can launch a generated scene.
12. User-facing docs explain the workflow and limitations.

## Risks

### Risk: Discovery Becomes A Slow Repo Crawler

Mitigation: cap file discovery, use known filenames, avoid recursive content
search in first pass.

### Risk: Generated Scenes Become Too Noisy

Mitigation: limit entity count, prefer important files, and expose additional
details in metadata rather than entities.

### Risk: Commands Feel Automatic

Mitigation: generated commands are only `RunCommand` choices. Discovery itself
does not execute verification commands.

### Risk: Git Assumptions Break Non-Git Workspaces

Mitigation: non-git directories are valid workspaces with `repo_status=not_git`.

### Risk: User Config Is Overwritten

Mitigation: generate to temp first, validate before write, require `--force`
for overwrite, and keep dry-run/output-file paths as the default workflow.

### Risk: Helper Duplicates Author Helper Logic

Mitigation: use existing scene validation examples and helper conventions.
Keep discovery focused on deriving data, not becoming a general authoring tool.

## Deferred Work

- live pane cwd discovery through mux APIs
- foreground process discovery
- multiple workspace roots
- richer language/package detection
- generated relationship graph
- configurable discovery profiles
- persistent workspace sessions
- command policy/allowlist
- agent task bootstrap from discovered repo
- richer visual layout generation
- generated sprite manifests

## Done Definition

The first pass is done when a user can run:

```sh
ci/gameterm-scene-workspace.sh discover \
  --cwd /Users/julianabeleda/env/gameterm \
  --scene-output /tmp/gameterm-workspace.json
```

Then validate and install or launch the generated scene without hand-editing.

At that point, Scene Mode has moved from authored Agent/Workspace demos toward
a GameTerm-native workspace surface that can later support richer agent,
memory, and process-aware workflows.

## First-Pass Implementation Status

Implemented first-pass items:

- `ci/gameterm-scene-workspace.sh` with `inspect`, `discover`, and `patch`
  commands.
- cwd, git root, branch, revision, dirty/clean/not-git status, language, and
  important-file discovery.
- generated complete Scene Mode scenes with workspace, project, task, process,
  and file entities.
- generated patch output for the existing `workspace-agent` base fixture.
- validation before scene writes, patch writes, inbox writes, and install.
- overwrite protection for output and install paths.
- strict missing-file handling for user-provided `--open` paths.
- non-git workspace support.
- `ci/gameterm-scene-verify.sh --all` coverage for git, non-git, patch,
  install, overwrite, and strict-missing-file behavior.
- `workspace-discovery` smoke scenario that launches a scene generated from the
  current repository.
- user-facing Scene Mode docs and product smoke checklist entries.

Remaining deferred items:

- live pane cwd discovery through mux APIs
- automatic foreground process discovery through mux APIs
- persistent workspace sessions
- richer configurable discovery profiles
- generated relationship graphs
