# GameTerm Scene Mode Multi-Agent Coordination Scope

This document scopes Product Layer 7 from the broader Scene Mode product pass:
Multi-Agent Coordination.

## Goal

Scene Mode should represent more than one active actor in a workspace.

The first agent/workspace pass is single-agent friendly. Real workflows may
include the user, assistant, scripts, verification processes, review agents, or
future background task actors.

## End Goal

A user can see:

- multiple agents or actors
- the task each actor owns or watches
- current lifecycle phase per actor
- blockers and waiting states per actor
- completed work per actor
- selected-entity-specific actions

Multiple agent patches must not overwrite each other accidentally.

## First-Pass Product Contract

The first pass should model two agents and two tasks in one fixture/generated
scene.

Required states:

- idle
- planning
- running
- waiting
- blocked
- completed
- failed or cancelled

Required relationships:

- agent owns task
- agent waits_for task or process
- task verified_by process

## Patch Contract

Existing agent helper conventions should be extended conservatively:

- agent id is explicit
- task id is explicit
- patch source can identify the actor
- process state references the specific entity it updates

Rules:

- patches update one intended agent/task unless explicitly given more ids
- patch validation rejects unknown ids
- helper defaults must not target a generic singleton when multiple agents exist
- status text should include the agent/task label or id

## Rendering Contract

Normal view should show:

- selected agent/task metadata
- current phase
- ownership/waiting relationship summary
- blockers when present

Tile Debugger should show:

- all active agents
- selected agent details
- last patch source and source pane
- process state if connected to the selected task/process

## Verification

Deterministic verification should cover:

- fixture with at least two agents and two tasks
- independent lifecycle patch updates
- blocked/review guarded choices
- selected entity drives visible action context
- patches do not overwrite unrelated agent/task metadata
- `ci/gameterm-scene-verify.sh --all`

Live smoke should cover at least two lifecycle updates while Scene Mode is open
after deterministic checks are stable.

## Commit Lanes

1. `[docs] scope Scene multi-agent coordination layer`
2. `[visual] add Scene multi-agent fixture support`
3. `[visual] add Scene multi-agent helper support`
4. `[test] verify Scene multi-agent coordination`
5. `[docs] document Scene multi-agent workflow`
6. `[tools] record Scene multi-agent smoke`

## Deferred Work

- real concurrent agent processes
- agent scheduling
- conflict resolution
- shared locks
- task assignment UI
- network or remote agent status

## Done Definition

The layer is first-pass complete when two agents can be represented, updated,
blocked, completed, and inspected independently through explicit local patches
and deterministic tests.
