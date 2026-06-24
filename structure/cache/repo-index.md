# Repo Index — what is maintained, and why each part is kept

This index exists so the question *"is this repo maintainable?"* is answerable
from data, not vibes. Every crate/folder is classified by how much is actually
**owned** (hand-maintained) vs **generated** / **vendored** / **test**, and every
crate carries a one-line justification for being kept — or a verdict to cut.

**Measured by:** `ci/repo-index.py` (re-runnable; `--md` for the table form).
Re-run after structural changes; the verdicts below are reviewed when
`module-notes/product-scope.md` changes.

> Caveat: "owned" includes inline `#[cfg(test)]` code (only separate `tests/`
> dirs and `*_test.rs` files are split into the Test column), so the true
> non-test owned surface is somewhat smaller than the numbers below.

## True size (measured)

| class | lines | share | maintenance burden |
|---|--:|--:|---|
| **Owned** | 205,942 | 43% | the real surface a human maintains |
| **Generated** | 220,754 | 46% | tables; ~zero burden (never hand-edited) |
| **Vendored** | 41,476 | 9% | upstream code in `deps/`; not ours to maintain |
| **Test** | 7,777 | 2% | separate test files/dirs (excl. inline tests) |
| **Total** | 475,949 | 100% | |

**The headline:** the repo *looks* like ~476k lines but **only 43% is
hand-maintained**, and most of *that* is a legitimate inherited terminal
emulator. GameTerm's own product surface (`gameterm-visual`) is **~20k**. "Too
large" was mostly generated tables (46%) hiding in the LOC count.

> **Updated after the remote/SSH cut:** owned dropped from 224,717 → 205,942
> (−18,775) when the inherited multiplexer/remote/SSH subsystem (7 crates) was
> removed. The cut crates' rows below have been struck.

## Justification index (owned LOC; verdict tied to product-scope)

### Product — GameTerm's own reason to exist (KEEP)
| crate | owned | why kept |
|---|--:|---|
| `gameterm-gui` | 51,084 | main app: terminal GUI **and** Scene Mode overlay/render host (also holds the 140k generated `unicode_names.rs`) |
| `gameterm-visual` | 20,396 | the Scene engine — the product differentiator |
| `config` | 8,757 | config model + Lua loading (product-scope: user control) |
| `lua-api-crates` | 4,345 | Lua API surface — scriptability/automation |
| `gameterm` | 3,362 | the `gameterm` CLI/launcher binary |
| `gameterm-input-types` | 3,211 | key/mouse input model (GUI + scene input) |
| `gameterm-dynamic` | 2,377 | dynamic value model for config/Lua/scene |
| `gameterm-toast-notification` | 533 | notifications (e.g. the panic/error toast) |
| `gameterm-blob-leases` | 494 | scene asset blob leases (visual) |
| `gameterm-gui-subcommands` / `gameterm-open-url` / `gameterm-version` | 435 | small product glue |

### Inherited terminal core — load-bearing, not optional (KEEP)
The "retain terminal semantics" half of product-scope. The GUI is architecturally
a renderer of these; the Scene engine renders through their cell/surface/image types.

| crate | owned | why kept |
|---|--:|---|
| `window` | 22,559 | platform window / input / GL; the GUI cannot exist without it |
| `mux` | 11,805 | pane/tab/domain model; GUI is a mux renderer (deleting = rewrite) |
| `gameterm-escape-parser` | 10,845 | terminal escape parsing |
| `termwiz` | 10,340 | terminal cell/line/surface library |
| `gameterm-font` | 10,266 | font load / shaping / glyph cache |
| `term` | 9,861 | terminal state machine; also feeds scene image cells |
| `gameterm-surface` | 5,210 | surface/cell model; scene renders through it |
| `bidi` | 5,058 | bidirectional text correctness |
| `pty` | 2,822 | PTY / process spawn (shell semantics) |
| `gameterm-cell` | 1,873 | cell + `ImageCell` types; scene VN panels render through these |
| `vtparse` | 1,599 | VT parser |
| `filedescriptor` | 1,563 | fd / socketpair helper (pty, mux) |
| `color-types` | 1,206 | color model |
| `procinfo` | 980 | process inspection (titles/status) |
| `lfucache` | 823 | LFU cache (glyph/render caches) |
| `bintree` / `rangeset` / `promise` / `tabout` / `base91` / `frecency` / `ratelim` / `env-bootstrap` / `luahelper` / `async_ossl` | ~4,000 | small shared primitives used across the core |
| `gameterm-char-props` | 170 | Unicode width/emoji/nerdfont lookup — **99.8% generated** (79k tables) |

### Remote access — REMOVED ✅ (was ~18.8k owned)
Product-scope no longer retains remote access. The inherited WezTerm
multiplexer/remote/SSH subsystem was removed entirely (commits `749d6e9`,
`81415f7`, `29e1519`). See `../Development/tiny-principle-audit.md`.

Removed crates: ~~`gameterm-ssh`~~, ~~`gameterm-client`~~,
~~`gameterm-mux-server-impl`~~, ~~`codec`~~, ~~`gameterm-mux-server`~~,
~~`gameterm-uds`~~, ~~`lua-api-crates/ssh-funcs`~~. Also removed: the `mux`-core
SSH integration (`Mux.agent`/`AgentProxy`, `ssh.rs`/`ssh_agent.rs`), the
`gameterm cli` control surface, the config SSH/serial surface, the serial domain,
and the flaky `gameterm_ssh.yml` CI job.

> `mux` **core** (local pane/tab/domain model) and `mux/src/client.rs` (local
> `ClientId`/`ClientInfo` bookkeeping, not the remote crate) stay — verified by a
> green `cargo check --workspace`.

### Generated — label, do not count as burden (KEEP, untouched)
| file / crate | generated | note |
|---|--:|---|
| `gameterm-gui/.../unicode_names.rs` | ~140,378 | Unicode name table |
| `gameterm-char-props` tables | ~79,168 | emoji-variation / nerdfont / widechar-width |
| `config` generated | ~1,007 | generated config docs/data |

### Vendored / tooling / fixtures (KEEP)
| folder | lines | note |
|---|--:|---|
| `deps/` | 41,476 | vendored upstream (e.g. cairo) — not ours |
| `ci/` | 12,701 | build/release + scene shell scripts — **flag:** the large `gameterm-scene-{author,verify,smoke}.sh` should be table-driven (see loc-complexity audit) |
| `assets/` / `test-data/` | 1,226 | bundled Lua assets + test fixtures |

## What this index proves

- **Maintained surface ≈ 224k, not 500k.** Of that, GameTerm-owned product is
  ~20k; the rest is a deliberately-kept terminal emulator.
- **Remote access — the only indefensible weight — has been removed** (~18.8k
  owned + 7 crates + the flaky CI job, gone).
- Everything else has a one-line reason tied to product-scope. If a crate can't
  earn a justification here, it's a cut candidate.

## Keeping it honest
1. Re-run `ci/repo-index.py` after adding/removing crates or large files.
2. New generated files must carry a header marker (`@generated` / "generated by")
   or be added to `GEN_NAMES` in the script, so they never inflate the owned count.
3. A new crate/folder must earn a row + justification here before it's merged.
