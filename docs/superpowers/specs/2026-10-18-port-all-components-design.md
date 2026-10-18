# Port all shadcn components — design

Date: 2026-10-18
Status: draft for the maintainer's approval

## Intent

`shadcn-egui` 0.6.0 has 14 components; shadcn-svelte at `tools/stylegen/PIN`
has 61. No app needs a missing component today. The maintainer expects the
need and wants the crate ready before it comes, so that nobody building an
egui app with the crate hits a gap halfway through.

What matters most is not completeness but **one consistent API across all
components that reads like normal egui and normal Rust**. A full set of
components with drifting APIs is a failure, even if every component works.

## Goals

- Port every component in shadcn-svelte's `docs/src/lib/registry/ui/` at
  `PIN`, to the standard in `plugin/skills/port-component/`.
- A written set of API rules that every component follows, the 14 existing
  ones included.
- The work runs in stages and uses many subagents where they help: reading,
  surveying, reviewing.

## Non-goals

- Components of very large effort are not ported in this project's main
  line. `chart` is one. The survey (stage 1) marks others; the maintainer
  confirms the list. They come last or in a later project.
- Components the maintainer judges not applicable in egui (see the calls
  below) are not ported.
- No new runtime dependency without the maintainer's consent
  (dependency discipline in `port-component/SKILL.md`).
- Nothing outside shadcn-svelte's `ui/` directory (blocks, examples, themes
  beyond what stylegen already handles).

## Anti-goals

- APIs that differ between components for the same idea (selection, sizing,
  IDs, sub-parts, orientation, containers).
- APIs that copy Svelte composition one to one where egui has its own
  idiom.
- A rule chosen by an agent where upstream is inconsistent. Those cases are
  the maintainer's (below).

## Current state

The 14 components share one main pattern: a builder
(`Button::new(..).variant(..).size(..)`) added with `ui.add(..)`,
`enabled(bool)` / `invalid(bool)` flags, variant and size enums, and
`width(f32)` / `max_width(f32)` with `f32::INFINITY` meaning full width.
These stay.

Known drifts, to be settled by the rules:

1. Selection model: `Select` and `Tabs` take `&mut usize` and `&[&str]`;
   egui's `ComboBox` / `selectable_value` take any `&mut T`.
2. Sub-part helpers: `card_header`, `card_title`, the dialog's plain
   `footer`, `popover_header`.
3. Container calls: `Card::show(ui, add) -> R` against
   `Popover::show(content) -> Option<R>` (no `ui`).
4. IDs: `Dialog::new(id_salt, ..)` hashes, `Select::new(id: &str, ..)`
   takes a string, `Input` has both `id_salt` and egui's old name
   `id_source`.
5. Shortcut functions: `toggle` for `Switch`, `tab_bar` for `Tabs`;
   `select()` returns `bool` where the others return `Response`;
   `tooltip(resp, text)` is a free function where egui uses a method on
   `Response`.
6. Orientation: `SliderOrientation` and `TabsOrientation` for one idea.

## Stages

### Stage 1 — Survey (read-only, many agents in parallel)

One agent per upstream component (all 61, the ported ones too, so the
rules see the whole set). No agent builds or edits the repository. Each
writes a report to a scratch directory with:

- source files at `PIN` and the bits-ui primitive and version it wraps;
- `cn-*` markers and `slot:` parts, and the styled states;
- classes in those parts that stylegen does not support yet;
- the components and shared primitives it depends on;
- the behaviour list of the primitive (keyboard, focus, pointer, disabled,
  degenerate input, accessibility role);
- an API sketch in egui terms, and every point where the sketch had to
  choose between options;
- an effort estimate (small / medium / large / very large) with the
  reason.

The controller merges the reports into one survey: component families,
dependency order, shared primitives, generator gaps, the very-large list.

### Stage 2 — API rules

One convention document, written from the survey, covering at least:
builder shape and shortcut functions, selection model, sub-part naming,
container calls and return types, IDs, orientation and other shared enums,
sizing, state flags, and how a Svelte compound component maps to egui.
Where egui has an idiom, the rule uses it.

**Case-by-case decisions.** Where upstream is inconsistent, or where the
rules leave two reasonable answers, the controller does not choose. It
collects every such case into one decision list: the case, the options,
the components affected, and a recommendation. The maintainer decides the
list in one pass. Cases that turn up later go into a new list, not into
single questions.

The rules live in `plugin/skills/port-component/` (a new reference file,
linked from `SKILL.md`), because the skills are the source documents of
this repository and every port reads them. `build-ui` gets what a user of
the crate needs from them.

### Stage 3 — Cleanup (no new components)

The 14 existing components follow the rules. This breaks the public API,
so it is released as a feature release (0.7.0) before any port lands. The
gallery, the tests, the skills (their rust fences are doctests) and
`CHANGES.md` change with it; `CHANGES.md` tells users how to move to the
new API.

The cleanup changes the API, not the feature set. The existing components
leave out upstream features on purpose (Select: groups, labels,
separators, disabled items; Checkbox: indeterminate; Slider: a second
thumb; Tabs: vertical). The cleanup gives each API a shape that can take
these features later as additions; the features come with the ports. Where
a correct shape cannot be found without the feature, correctness wins and
the feature comes into the cleanup.

A plain rename keeps a `#[deprecated]` alias for one release (for example
`toggle`, `tab_bar`, `id_source`). A changed signature (for example
`Select` from `&mut usize` to `&mut T`) breaks without an alias.

Shared primitives that several families need (for example a menu, roving
focus, an overlay / sheet, a scroll area) and generator support for the
gaps found in stage 1 land before the ports that use them, one at a time.
Whether they belong to stage 3 or to the first port of each family is the
builder's call.

### Stage 4 — Ports

One family per pull request, in dependency order. Each port follows
`port-component` and the rules, with its own snapshots, behaviour list,
`component-map.md` row and `CHANGES.md` entry. Very-large components come
last.

## Constraints

- **One build at a time.** The machine's 4-core cap counts every build on
  the box together; only one agent builds or runs gates at a time.
  Read-only agents (survey, review) run alongside.
- **Memory.** All sessions share one 25 GiB cgroup; snapshot test runs in
  parallel worktrees risk an OOM kill in a sibling session.
- **Disk.** Each worktree gets its own `cargo-target-<name>` on `/scratch`;
  keep the number of live worktrees small and remove them after merge.
- **Generated files** (stylegen output, `markers.txt` effects) conflict
  between parallel branches and cannot be merged by hand: branches that
  touch stylegen land one after another and regenerate after each merge.
- **Snapshot review is the maintainer's.** One component gives 14 PNGs
  (7 styles × 2 modes). Each PR shows its baselines; agents say plainly
  when they did not compare with the shadcn-svelte docs page.
- **Merging is the maintainer's.** Merge commits only.
- Every gate in `CLAUDE.md` passes on every PR.
- The repository is public: no private names or paths in committed files.

## Decided by the maintainer

- Scope: everything upstream has; very-large components such as `chart`
  wait.
- Staged work as above: survey, rules, cleanup, then ports.
- Cleanup of the 14 existing components comes before new components, and
  may break the API.
- Upstream inconsistencies are decided case by case by the maintainer, in
  one list per round.
- Consistency and idiomatic egui and Rust come first.
- Before 1.0 the API may break whenever a decision needs it, as long as
  `CHANGES.md` documents each change so that a user of the crate can
  upgrade easily. Keep breakage small, but correctness comes first. A
  later decision list may change components that are already released.

## Calls made by the controller (check these)

- The survey covers all 61 components, the ported 14 included.
- Besides "very large", the survey may give the verdict "not applicable in
  egui" (a candidate: `form`, which wraps a Svelte form library). These
  verdicts go into the maintainer's decision list.
- Every new runtime dependency a component would need (for example a date
  type for `calendar`: `jiff`, `chrono` or a type inside the crate) is
  flagged by the survey and goes into the decision list.
- The rules live in a new reference file under
  `plugin/skills/port-component/references/`. The maintainer sees it with
  the decision list; it lands in the cleanup pull request, so its rust
  fences compile against the new API.
- The cleanup is its own release, before any port.
- One pull request per family, not per component.
- Only one pull request that changes stylegen output is open at a time.
  While the maintainer reviews it, agents prepare the next family
  read-only.
- Renames keep a `#[deprecated]` alias for one release; changed signatures
  break without one.
- The cleanup shapes the APIs for the missing features of the existing 14
  but does not add the features, unless the correct shape needs them.

## Left to the builder

- Family boundaries and order, from the survey.
- The survey report format beyond the items listed.
- Whether a shared primitive lands with the cleanup or with the first
  family that needs it.
- How many read-only agents run at once.
