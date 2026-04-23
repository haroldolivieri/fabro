---
title: "refactor: replace merge.rs with Combine trait + derive (uv pattern)"
type: refactor
status: queued
date: 2026-04-23
sequence_after: 2026-04-23-001-refactor-collapse-settings-resolve-indirection-plan.md
---

# refactor: replace merge.rs with Combine trait + derive (uv pattern)

## For the engineer picking this up

This is the second of two related settings-architecture refactors. PR 1 (`docs/plans/2026-04-23-001-...`) collapses the resolve-indirection layer (`Resolver`, `ResolvedSettingsTree`, `*_into` methods). PR 2 — this one — replaces the 750-line hand-rolled `merge.rs` with a small `Combine` trait, a dumb derive macro, and a handful of newtypes. Land PR 1 first; the two are orthogonal but PR 1 is smaller and lower-risk.

The pattern is lifted directly from astral-sh/uv. The user's reference is `/Users/bhelmkamp/p/astral-sh/uv/crates/uv-settings/src/combine.rs` and `/Users/bhelmkamp/p/astral-sh/uv/crates/uv-macros/src/lib.rs` — read both before starting. They're short.

## Overview

`lib/crates/fabro-config/src/merge.rs` contains 42 hand-rolled `combine_*` functions implementing the v2 settings-merge matrix. Most are mechanical "field: higher.x.or(lower.x)" boilerplate. Replace them with:

1. A `Combine` trait (single `fn combine(self, other: Self) -> Self` method).
2. A dumb `#[derive(Combine)]` proc-macro that emits `Self { f: self.f.combine(other.f), ... }` field-by-field. ~25 lines.
3. `impl Combine` on every leaf type (`Option<String>`, `Option<u64>`, every settings enum, etc.). Most are one-liners (`self.or(other)`).
4. Newtypes for the three currently-bare HashMap variants (`metadata` replaces whole-map; `env` and `labels` are sticky-by-key).
5. Bespoke `impl Combine` on the half-dozen irregular cases (hooks, splices, whole-replace structs).

Every `*Layer` struct gets `#[derive(Combine)]`. `merge.rs` deletes entirely.

## Why this matters (don't skip — it's load-bearing)

I (the previous engineer / Claude) initially recommended AGAINST a derive macro because attribute-driven derives hide the merge matrix behind macro expansion. **uv's pattern sidesteps this by encoding the rule in the type, not in attributes.** The derive is dead-stupid — it dispatches to `field.combine(other.field)` and that's it. The intelligence lives one level down in `impl Combine` for each leaf type. The merge matrix is auditable in one file (`combine.rs`) with one stanza per leaf type, and the type system enforces it: you can't accidentally use the wrong merge strategy for a HashMap because `ReplaceMap` and `StickyMap` are different types.

If you're tempted to add `#[combine(strategy = "...")]` attributes — DON'T. That defeats the entire point. If a field needs a custom rule, it gets a newtype.

## Reference: how uv does it

Read these in order:

1. `/Users/bhelmkamp/p/astral-sh/uv/crates/uv-macros/src/lib.rs` lines 17-52 — the entire derive macro.
2. `/Users/bhelmkamp/p/astral-sh/uv/crates/uv-settings/src/combine.rs` — the trait, the `impl_combine_or!` macro for one-line `self.or(other)` impls, the bespoke struct impls, the `Option<Vec<T>>` impl, the `Option<BTreeMap<K, Vec<T>>>` impl.
3. `/Users/bhelmkamp/p/astral-sh/uv/crates/uv-settings/src/settings.rs` — search for `#[derive(...Combine...)]` to see how it's applied to the `*Options` structs.

The whole pattern is small — under 400 lines including the derive crate.

## Fabro-specific wrinkle: HashMap-three-meanings

uv's pattern works cleanly because every field type is structurally distinct. Fabro has three `HashMap<String, String>` fields with three different merge rules:

| Field             | Type today                | Today's `merge.rs` rule        |
|-------------------|---------------------------|--------------------------------|
| `*.metadata`      | `HashMap<String, String>` | replace whole if `higher` non-empty (`merge_string_map_replace`) |
| `run.sandbox.env` | `HashMap<String, String>` | sticky-by-key (`merge_string_map_sticky`) |
| `daytona.labels`  | `HashMap<String, String>` | sticky-by-key (`merge_string_map_sticky`) |

To use uv's "rule lives in the type" pattern, introduce two newtypes in `lib/crates/fabro-types/src/settings/`:

```
#[serde(transparent)]
pub struct ReplaceMap<V>(pub HashMap<String, V>);

#[serde(transparent)]
pub struct StickyMap<V>(pub HashMap<String, V>);
```

And impl `Combine` for each. Field declarations in the layer types now document the rule:

```
pub struct ProjectLayer {
    pub metadata: Option<ReplaceMap<String>>,   // visibly "replace whole"
    // ...
}
pub struct RunSandboxLayer {
    pub env: Option<StickyMap<String>>,         // visibly "sticky-by-key"
    // ...
}
```

This is a strict ergonomic improvement: today you have to grep `merge.rs` to find out how `metadata` merges; after, the field type tells you.

**Migration consideration:** these newtypes need careful serde handling. `#[serde(transparent)]` on the newtype + same on `Option<>` should let TOML deserialize the field as a plain table. Verify with the existing `tests/parse_*` tests in `fabro-config` and add new round-trip tests.

## Irregular cases (hand-written `impl Combine`)

These don't fit the derive pattern. Each gets a bespoke `impl Combine` on its own type. Look at `merge.rs` for current behavior:

| Type | Current fn | New shape |
|------|-----------|-----------|
| `Vec<HookEntry>` | `combine_hooks` | `impl Combine for Vec<HookEntry>` — ordered merge with optional `id` replacement |
| `Vec<ModelRefOrSplice>` | `splice_model_fallbacks` | `impl Combine for Vec<ModelRefOrSplice>` — splice with `Splice` sentinel |
| `Vec<StringOrSplice>` (events) | `splice_events` | `impl Combine for Vec<StringOrSplice>` — splice variant |
| `RunPrepareLayer` | `combine_run_prepare` (returns `higher` whole) | `impl Combine for RunPrepareLayer { fn combine(self, _other) = self }` |
| `CliTargetLayer` | `combine_cli_target` (returns `higher` whole) | same shape as above |
| `ServerListenLayer` | `combine_listen` (returns `higher` whole) | same shape as above |
| `RunCheckpointLayer.exclude_globs` | guards "if higher empty, take lower" | inline as part of derived `RunCheckpointLayer` impl, or wrap in `Replace<Vec<...>>` newtype if you want symmetry |
| `HashMap<String, NotificationRouteLayer>` | `combine_notifications` (per-key recursive combine) | `impl Combine for HashMap<String, NotificationRouteLayer>` per-key combine |
| `HashMap<String, McpEntryLayer>` (similar) | check `merge.rs` for the exact case | per-key combine variant |

About 5–8 of these total. Each is named at its type, not at a free function in `merge.rs`.

## Convention to pick: argument order / which side wins

uv uses `self.combine(other)` where **`self` wins** (self is the higher-precedence layer). Fabro's current `merge.rs` uses `combine_files(lower, higher)` where **higher (the second arg) wins**. Pick one and stick with it.

Recommendation: adopt uv's "self wins" convention. It reads naturally — `user_settings.combine(defaults)` = "user settings, with defaults as fallback." For multi-layer stacking: `workspace.combine(user).combine(defaults)` gives `workspace > user > defaults`.

This means `apply_builtin_defaults` becomes:

```
pub fn apply_builtin_defaults(layer: SettingsLayer) -> SettingsLayer {
    layer.combine(defaults_layer().clone())
}
```

And the existing `combine_files(lower, higher)` callers need their argument order swapped.

## Implementation Units

- [ ] **Unit 1: Add `Combine` trait + leaf impls + the `impl_combine_or!` macro**

**Goal:** Lay down the trait infrastructure with no derive yet.

**Files:**
- Create: `lib/crates/fabro-config/src/combine.rs`
- Modify: `lib/crates/fabro-config/src/lib.rs` (add `mod combine; pub use combine::Combine;`)

**Approach:**
- Copy uv's trait shape verbatim. Convention: `self` wins.
- `impl_combine_or!` macro for one-liners. List every leaf scalar/enum used in `*Layer` types: `String`, `bool`, `u8/16/32/64`, `PathBuf`, `InterpString`, `RunMode`, `ApprovalMode`, `SandboxProvider`, `ObjectStoreProvider`, `WebhookStrategy`, `ServerAuthMethod`, etc. Grep `lib/crates/fabro-types/src/settings/` for the full list.
- Generic `impl<T: Combine + Default> Combine for Option<T>` for nested struct combine.
- `impl<T> Combine for Vec<T>` — pick the convention (self wins whole-list if non-empty, else other).
- `impl Combine for HashMap<String, V>` only if there's a use case beyond the newtypes; otherwise rely on `ReplaceMap`/`StickyMap`.

**Test scenarios:**
- Happy path: `Some("a").combine(Some("b")) == Some("a")`.
- Happy path: `None.combine(Some("a")) == Some("a")`.
- Edge case: `Some("a").combine(None) == Some("a")`.
- Edge case: nested `Option<Option<T>>` — verify the generic impl doesn't infinite-recurse or behave surprisingly.
- Each leaf type gets at least one assertion proving `self.or(other)` semantics.

**Verification:** `cargo build -p fabro-config` clean. New unit tests pass.

---

- [ ] **Unit 2: Create `fabro-config-macros` crate with `#[derive(Combine)]`**

**Goal:** The dumb derive macro.

**Files:**
- Create: `lib/crates/fabro-config-macros/Cargo.toml` (proc-macro = true)
- Create: `lib/crates/fabro-config-macros/src/lib.rs`
- Modify: workspace `Cargo.toml` to include the new crate
- Modify: `lib/crates/fabro-config/Cargo.toml` add `fabro-config-macros` dep
- Modify: `lib/crates/fabro-config/src/lib.rs` re-export the derive

**Approach:**
- Copy uv's `derive_combine` verbatim from `/Users/bhelmkamp/p/astral-sh/uv/crates/uv-macros/src/lib.rs:17-52`.
- The derive emits `Self { f1: self.f1.combine(other.f1), ... }` for named fields. Unnamed fields / enums: `unimplemented!()` (uv does the same; we don't need them).
- The derive references `crate::Combine` — same as uv. Works because the derive output is expanded inside the consumer crate.

**Test scenarios:**
- Happy path: a small struct `#[derive(Combine)] struct Foo { a: Option<u32>, b: Option<String> }` produces a working `Combine` impl. Proven by a unit test that asserts `Foo { a: Some(1), b: None }.combine(Foo { a: Some(2), b: Some("x".into()) }) == Foo { a: Some(1), b: Some("x".into()) }`.
- Integration: nested struct with an `Option<InnerStruct>` field combines recursively.

**Verification:** `cargo build -p fabro-config-macros` clean. Derive test passes.

---

- [ ] **Unit 3: Introduce `ReplaceMap` and `StickyMap` newtypes; migrate the three HashMap fields**

**Goal:** Move the HashMap merge rules into the type system.

**Files:**
- Create: `lib/crates/fabro-types/src/settings/maps.rs` (or extend existing module)
- Modify: `lib/crates/fabro-types/src/settings/mod.rs` to export the newtypes
- Modify: `lib/crates/fabro-config/src/combine.rs` add `impl Combine for ReplaceMap<V>` and `StickyMap<V>`
- Modify field declarations in `lib/crates/fabro-types/src/settings/{project,workflow,run}.rs`:
  - `*.metadata` fields → `Option<ReplaceMap<String>>`
  - `run.sandbox.env` → `Option<StickyMap<String>>`
  - `daytona.labels` → `Option<StickyMap<String>>`
- Audit every consumer of these fields — they currently access `HashMap<String, String>` directly; with newtypes they'll need `.0` or `Deref` impls.

**Approach:**
- `#[serde(transparent)]` on the newtype.
- Implement `Deref<Target = HashMap<String, V>>` so existing read-side consumers keep working without churn.
- Implement `Default`, `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`.
- `impl<V> Combine for ReplaceMap<V>`: if `self.0` is non-empty take self, else take other (matches today's `merge_string_map_replace` with self/other swapped per the convention chosen in Unit 1).
- `impl<V> Combine for StickyMap<V>`: per-key insert-or-keep (matches `merge_string_map_sticky`).

**Test scenarios:**
- Happy path: `ReplaceMap` round-trips through TOML deserialize/serialize unchanged.
- Happy path: `ReplaceMap::combine` with both non-empty takes `self`'s map whole.
- Happy path: `StickyMap::combine` merges keys, `self` wins on conflict.
- Edge case: `ReplaceMap` empty + `StickyMap` empty — combine produces empty/empty.
- Integration: existing `tests/parse_*` tests still parse the same TOML fixtures without modification (proves transparent serde works).

**Verification:** All existing parse tests pass. New newtype tests pass.

---

- [ ] **Unit 4: Add `#[derive(Combine)]` to every `*Layer` struct; add bespoke `impl Combine` for irregular cases**

**Goal:** Cover every settings type. After this unit, `SettingsLayer.combine(other)` works.

**Files:**
- Modify: every `*Layer` definition in `lib/crates/fabro-types/src/settings/{cli,project,workflow,run,server,features,layer}.rs` to add `#[derive(Combine)]` and `Default` if not already derived.
- Modify: types from the irregular-cases table to add bespoke `impl Combine`. Place each impl next to its type definition.

**Approach:**
- Walk through `merge.rs` function-by-function. For each `combine_<name>(lower, higher) -> NameLayer`:
  - If the body is field-by-field `higher.x.or(lower.x)` for every field → add `#[derive(Combine)]` to the struct, delete the function.
  - If one or two fields are special (`merge_string_map_replace` etc.) → if those fields have been migrated to newtypes (Unit 3), the derive handles it. If not, hand-write `impl Combine` on the struct.
  - If the body returns `higher` whole → hand-written `impl Combine` returning `self`.
- For Vec types with splice semantics (model fallbacks, events) → bespoke `impl Combine for Vec<...>`. The current `splice_*` helpers can be inlined into the impls.

**Test scenarios:**
- Happy path: for each layer struct, an existing `merge.rs` test (search for `#[test]` in `lib/crates/fabro-config/src/merge.rs` or `tests/`) continues to pass with the new infrastructure substituted. (Recommended: write the new infra to coexist with `merge.rs` initially, swap call sites in Unit 5, then delete `merge.rs` in Unit 6.)
- Edge case: hooks ordered merge with id replacement — assert the exact ordering produced today is preserved.
- Edge case: model fallbacks splice — verify `Splice` sentinel handling.

**Verification:** `cargo build --workspace` clean. Existing merge tests still pass when run against the new trait dispatch.

---

- [ ] **Unit 5: Swap `combine_files` call sites to use `Combine` trait; rewrite `apply_builtin_defaults`**

**Goal:** All merging goes through `.combine()`.

**Files:**
- Modify: `lib/crates/fabro-config/src/defaults.rs` — `apply_builtin_defaults` becomes `layer.combine(defaults_layer().clone())`.
- Modify: every caller of `combine_files` — search `grep -rn "combine_files" lib/`. Audit and convert each.
- Argument-order check: today's `combine_files(lower, higher)` becomes `higher.combine(lower)` under the "self wins" convention.

**Verification:** `cargo build --workspace` clean. `cargo nextest run --workspace` green.

---

- [ ] **Unit 6: Delete `merge.rs`**

**Goal:** Final cleanup.

**Files:**
- Delete: `lib/crates/fabro-config/src/merge.rs`
- Modify: `lib/crates/fabro-config/src/lib.rs` remove `mod merge;` and any re-exports of merge helpers (`combine_files`, etc.)

**Verification:**
- `cargo build --workspace` clean.
- `cargo +nightly-2026-04-14 clippy --workspace --all-targets -- -D warnings` clean.
- `cargo nextest run --workspace` green.
- `grep -rn "combine_files\|merge_option\|merge_string_map" lib/` returns no hits.

## System-Wide Impact

- **API surface:** `fabro_config::combine_files` removed. Likely no external callers but verify with grep before deleting.
- **Consumer code:** Field reads on `metadata`/`env`/`labels` may need `.0` if `Deref` isn't sufficient for the access pattern. Audit during Unit 3.
- **Serde compatibility:** `#[serde(transparent)]` on newtypes must round-trip identically to today's bare `HashMap` fields. Existing `tests/parse_*` are the canary.
- **Test coverage:** every `merge.rs` test must keep passing, just running through the new infrastructure. Don't delete or rewrite tests — let them prove behavior parity.
- **Net code change:** ~750 lines of `merge.rs` removed; ~25 (derive crate) + ~120 (`combine.rs` trait + leaf impls) + ~50 (newtypes + irregular impls) ≈ 195 added. Net ~555-line reduction.

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| `Deref<Target = HashMap>` on newtypes doesn't cover every access pattern (e.g., methods that take `HashMap` by value or by `&mut`) | Audit consumers during Unit 3. Add `From`/`Into` impls or change consumer code to take the newtype. |
| Serde `#[serde(transparent)]` doesn't behave identically to bare HashMap for some TOML edge cases | The existing `tests/parse_*` fixtures are the contract. Run them after Unit 3; any failure is the migration bug. |
| Convention swap (lower/higher → self/other) introduces subtle bugs at `combine_files` call sites | Do Unit 5 carefully. Each call-site swap is a 2-line diff but easy to invert. Lean on existing merge tests. |
| `Vec<T>` Combine generic conflicts with bespoke `Vec<HookEntry>` / `Vec<ModelRefOrSplice>` impls | Rust orphan rules + specialization rules mean the generic and the bespoke may not coexist. Likely workaround: use newtypes (`HookList(Vec<HookEntry>)`, `ModelFallbackList(Vec<...>)`) — same pattern as the HashMap newtypes. Verify early in Unit 1. |
| `Default` requirement on the generic `Option<T: Combine + Default>` impl forces `Default` on every struct (some don't have one today) | Either derive `Default` everywhere (likely fine; layer structs are sparse-by-design) or split the generic impl into `Option<T: Combine>` without the Default bound. |

## Open Questions

### Resolved (from design conversation)

- **Trait + dumb derive vs. attribute-driven derive?** Dumb derive (uv pattern). Attributes hide the merge matrix.
- **Newtypes for the three HashMap variants?** Yes. Rule lives in the type.
- **Convention?** "Self wins" (uv). Swap argument order at call sites in Unit 5.

### Deferred to Implementation

- **Should `combine_files` survive as a thin wrapper or be deleted entirely?** Probably delete; `layer.combine(other)` reads fine.
- **Per-key recursive combine for `HashMap<String, NotificationRouteLayer>` and similar — use a generic `RecurseMap<K, V>` newtype, or inline impls?** Inline likely simpler given there are ~2 such cases.
- **Does `Vec<T>`'s generic Combine impl conflict with bespoke `Vec<HookEntry>` impl in practice?** Verify in Unit 1; if yes, convert irregular Vecs to newtypes.

## Sources & References

- **Reference implementation:** `/Users/bhelmkamp/p/astral-sh/uv/crates/uv-settings/src/combine.rs` and `/Users/bhelmkamp/p/astral-sh/uv/crates/uv-macros/src/lib.rs` (read both fully before starting).
- **Sequencing:** Land `2026-04-23-001-refactor-collapse-settings-resolve-indirection-plan.md` first.
- **Design conversation:** preserved in chat transcript with Bryan dated 2026-04-23. Key decisions: rule-lives-in-type beats attribute-driven derive; HashMap-three-meanings needs newtypes; `WorkflowSettings` includes `ServerNamespace` (PR 1 decision); `Combine` is the right shape but `Resolve` is not (resolve is per-field, not per-type).
- **Origin of current `merge.rs`:** "v2 merge matrix implementation" header note; rules trace to settings TOML redesign requirements (`docs/plans/2026-04-08-settings-toml-redesign-implementation-plan.md` and the follow-on handoffs).
