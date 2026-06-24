# Tiny-Principle Audit

An audit of GameTerm against the "tiny principle" (see `coding-principles.md` →
*Reducing Code The Right Way*): **maintenance burden, not line count, is the
metric; the fastest way to shrink a fork is deleting whole inherited subsystems
it does not use.**

Grounded in the measured classification in `../cache/repo-index.md`
(`ci/repo-index.py`) and the prior audits in
`loc-complexity-refactor-analysis.md` / `../cache/module-notes/`.

## "Too large" is three different problems — only two are real

1. **Generated-table illusion (not a problem).** 44% of the repo (~221k lines)
   is machine-generated tables — `unicode_names.rs` (~140k), `char-props` tables
   (~79k), generated config (~1k). Zero maintenance. Fix = *label* them (done:
   the indexer classifies them out); do not "reduce" them.
2. **CI / portability burden (real, cheap).** The macOS release builds every
   binary twice (Intel + ARM) then `lipo`-fuses them, and ~25 workflows build
   Windows/Linux distros never shipped. Pure config weight.
3. **Coupling (real, already in progress).** A few owned Scene files mix concerns
   (`gameterm-visual/src/lib.rs`, `gameterm-gui/src/overlay/visual.rs`,
   `visual_quad.rs`). This is a *refactor/extraction* problem with existing lanes
   — **not** a deletion problem. Out of scope for this audit.

## Decisions (recorded)

- **Remote access: OUT of scope.** GameTerm does not use SSH or remote
  multiplexing. `product-scope.md`'s "retain remote access" is dropped. → the
  remote/SSH/serial subsystems become cut candidates (#3, #4).
- **Universal build: dropped.** No product crate has Intel-specific code, so
  Intel support is pure cost. macOS ships **ARM-only**.

## Ranked cuts (leverage = maintenance removed ÷ effort)

| # | Cut | Removes | Effort | Risk | Status |
|---|---|---|---|---|---|
| 1 | **macOS ARM-only** (drop x86_64 build; test on aarch64) | ~half CI time; fixes "can't test Intel" | trivial (config) | none | **✅ done** |
| 2 | **Delete non-macOS CI** (`gen_{windows,ubuntu,fedora,centos9,debian}*`, `nix*` — 33 files) + trim generator `TARGETS` to macos | ~33 workflow files | low (config) | none | **✅ done** |
| 3 | **Cut remote-mux** (`gameterm-client` + `mux-server` + `-impl` + `codec` + `uds`) | ~8.3k owned, a whole networking subsystem | medium | medium | next |
| 4 | **Cut SSH + serial** (`gameterm-ssh` + serial domain) | ~6.4k owned + retires flaky `gameterm_ssh.yml` | medium | medium | next |

### Follow-ups surfaced while doing #1/#2
- **Generator fork-over (tracked).** `ci/generate-workflows.py` is only partially
  de-wezterm'd — it still emits the old macOS arch/deployment and the
  `wez/homebrew-gameterm` tap/winget/flathub. The operative `gen_macos*.yml` are
  hand-maintained with the fork's fixes; **do not regenerate** until the generator
  is fully forked over. `TARGETS` is already trimmed to macOS so a regen can't
  reintroduce the distro workflows.
- **Local cruft (not repo bloat).** ~550 MB of untracked `GameTerm-macos-0.1.{0,1,2}/`
  extracted dirs + zips sit in the working tree (gitignored). `rm -rf` them.

**Total source removed by #3+#4: ~14.4k owned lines + 6 crates + a flaky CI job.**
Modest in LOC (≈6% of the owned surface) but high in maintenance value: fewer
deps, smaller `unsafe`/security surface, simpler dep graph, no remote attack
surface, no flaky e2e.

## Do NOT cut (so effort isn't wasted)
`mux` **core**, `term` / `gameterm-cell` / `gameterm-surface` image-cell plumbing
(the Scene engine renders VN panels through `ImageCell`/`ImageData`), `window`,
`gameterm-font`, `termwiz`, `gameterm-escape-parser`, `bidi`, `char-props`
(generated). The sixel/kitty/iterm *parsers* are dead-ish input paths but
entangled with the `term` performer and share types with the visual engine →
low value, high friction.

## Sequence
1. **#1 + #2 now** — pure config, zero source risk; immediately fixes the Intel/CI
   pain and the failed-release loop.
2. **#3 + #4 next** — source surgery into `mux` + startup + subcommands; verify
   with `cargo check -p gameterm-gui` at each step.
3. **Coupling** — leave to the existing extraction lanes; not a deletion.
