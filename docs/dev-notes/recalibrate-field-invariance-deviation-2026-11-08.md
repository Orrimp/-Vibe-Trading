# The `## Field invariance` table's `✓`, and the ULP underneath it — 2026-11-08

**Operator ruling (2026-11-08): document the deviation, leave the two anchored bodies alone.**
This note is that documentation. Sweep finding F4 Part C; Parts A and B shipped as bug-log
`#167`. Nothing under `evidence/` was touched and all 119 anchors are byte-identical.

## 1. What the two bodies claim

`crates/forecast/src/bin/recalibrate_sigma_train.rs` renders a `## Field invariance` section
into each derivation report. Its Match column is **the literal string `✓`**, and the
"Recalibrated" cells print the same `original_metadata`-derived variables as the "Original"
cells. So the table does not *perform* the comparison it displays — it asserts the outcome.

The two affected bodies are anchored:

| body | anchor row |
|---|---|
| `evidence/v1/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs1-20261001.md` | `anchors.toml:326` |
| `evidence/v1/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs2-20261001.md` | `anchors.toml:327` |

Making the table real changes the rendered body, which moves those two body-SHAs and requires
the ADR-0038 § D6.b protocol. That is what the ruling declined, and why.

## 2. The row where it matters, and what is actually true there

The BS-2 body prints

```
| final_val_loss | 1.05101e-5 | 1.05101e-5 | ✓ |
```

and `final_val_loss` is **the one field whose on-disk text is not identical** between the
committed original and the committed overlay:

```
original : "final_val_loss":0.000010510055290069431
overlay  : "final_val_loss":0.000010510055290069433
```

The `.5e` format collapses both to `1.05101e-5`, so the report cannot show the difference even
if it compared honestly.

**But the obvious reading of that — "a second field got substituted" — is wrong**, and the
mechanism is worth more than the row. Measured in Rust:

```text
str::parse::<f64>  "…069431" -> bits 3ee60a8b00000000    "…069433" -> bits 3ee60a8b00000001
serde_json         "…069431" -> bits 3ee60a8b00000001    "…069433" -> bits 3ee60a8b00000001
```

**`serde_json`'s float parser is off by one ULP on that literal.** Rust's own
`str::parse::<f64>` and Python's `float()` both round correctly and give two *distinct* f64s;
serde_json gives the same one for both. Its shortest-round-trip re-serialisation of that f64 is
`…069433`.

So the pipeline did this, and only this:

1. `write_overlay` read the committed original. serde_json parsed `…069431` to the f64 whose
   shortest form is `…069433`.
2. `provenance::overlay_with_sigma_train` substituted `sigma_train` and **touched nothing else**.
3. `canonicalise` re-serialised, writing `final_val_loss` back as `…069433`.

The field was never substituted. Its text changed because it made a lossy round-trip through
the JSON library. ADR-0035 D2's scope invariant holds; the `✓` on that row happens to be the
right answer, reached without asking the question.

## 3. What is now gated instead

`crates/forecast/tests/recalibrate_sigma_train_field_invariance.rs` (bug-log `#167`) carries
three views of the committed pairs, deliberately at different resolutions:

| test | subject | resolution | sees the ULP? |
|---|---|---|---|
| `committed_overlays_substitute_exactly_sigma_train` | the committed **files** obey D2 | `canonicalise` over `serde_json::Value` | **no** — serde_json collapses it |
| `committed_overlays_replay_byte_identically_through_the_shipped_path` | the **shipped code** reproduces the committed bytes | raw file bytes | implicitly — the bytes include it |
| `committed_overlay_text_divergence_is_exactly_the_declared_set` | the **text-level** divergence set | raw per-key value text | **yes**, and declares it |

The third one is why this note is a record and not a residual: the BS-2 artefact is a *declared*
entry (`["final_val_loss", "sigma_train"]`), so a **new** text divergence on any other field
fails the build and has to be explained before the declaration widens. The first test's doc
comment states that it structurally cannot see this class, so its green is not mistaken for a
stronger claim than it makes.

Measured: the first test stays green through all three perturbations of the shipped substitution
function; the second catches all three. Two tests, two subjects.

## 4. The wider consequence, stated and not acted on

`model_revision` is SHA-256 over `canonicalise` output, and `canonicalise` serialises numbers
via `serde_json::Number::to_string()`. So **every `model_revision` in this repo is computed over
serde_json's rounding of the input decimals, not over correctly-rounded f64s.** For
reproducibility *within* this codebase that is harmless and in fact stabilising — one parser,
one answer. It would stop being harmless if a checkpoint's metadata were ever produced or
verified by a different JSON implementation, which would round `…069431` the other way and
compute a different `model_revision` for byte-identical input.

No such second producer exists today: `grep` finds `canonicalise` called only from
`crates/forecast`. This is recorded as a property, not opened as work — changing the number
format would move every `model_revision` and every anchored body that prints one, which is a far
larger act than the row that led here.

## 5. If the ruling is ever revisited

Part C's implementation is small and was specified during verification: thread the produced
`overlay` Value into `render_report`, derive each "Recalibrated" cell from it, and compute the
Match column as `canonicalise(orig) == canonicalise(recal)` instead of the literal. The cost is
the D6.b protocol over two bodies plus ~487 s and ~620 s of measured compute behind `#[ignore]`.
Note that an honest Match column would print `✓` for BS-2's `final_val_loss` anyway — the
comparison it would perform is the one that cannot see the ULP. Making the table truthful and
making it *sensitive* are two different jobs, and only the first is in Part C's scope.
