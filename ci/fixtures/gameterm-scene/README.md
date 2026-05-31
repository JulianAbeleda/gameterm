# GameTerm Scene Mode Fixtures

These fixtures exercise Scene Mode without touching user config.

- `default.json`: normal scene with Inspect, OpenFile, Navigate, and explicit-argv RunCommand choices.
- `memory.json`: navigation target scene.
- `invalid.json`: syntactically valid JSON that fails scene validation.
- `patch-status.json`: valid in-memory scene patch for the default fixture.
- `patch-unknown-entity.json`: invalid patch that references a missing entity.
- `run-command-targets.json`: command target fixture for tab, right split, and down split live audits.
- `vertical-slice.json`: playable Scene Mode slice for dialogue, RPG state, and layered/process state.
- `authoring-loop.json`: in-app story-state save, mutate, and reload workflow fixture.
- `sprites.json`: sprite manifest that points at bundled test assets.
- `sprites-missing.json`: sprite manifest with one intentionally missing sprite path.
