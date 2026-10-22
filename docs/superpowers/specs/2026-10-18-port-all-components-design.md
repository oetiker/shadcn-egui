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

One report section per upstream component (all 61, the ported ones too, so the
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

Before 1.0 there are no `#[deprecated]` aliases or forwards: a rename
(for example `toggle`, `tab_bar`), a removal (`id_source`) or a changed
signature (for example `Select` from `&mut usize` to `&mut T`) breaks at
once, and CHANGES.md gives the upgrade step for each. (Changed by the
maintainer on 2026-10-19; it replaces "a plain rename keeps a
`#[deprecated]` alias for one release".)

Shared primitives that several families need (for example a menu, roving
focus, an overlay / sheet, a scroll area) and generator support for the
gaps found in stage 1 land in a shared step of their own, before the
first port that uses them, never inside a port (Porting process). Whether a shared step is part of the cleanup or runs
just before a family is the builder's call.

**Planning rule for the shared step.** Before it starts, the step lists,
against the component list of the ports that follow it, everything two
components will both need: helpers, generator flags and variants, icon
slots, `Metrics` fields, test harness pieces. Each item gets one owner in
the shared step; no port builds a shared item in parallel with another.

The shared step owes, besides the helpers it already has:

- Icon slots of D-94 (`select_trigger`, the plain down chevron, and the
  up chevron for libraries whose up chevron is no mirrored down one),
  through icongen, with every icon crate regenerated.
- stylegen: the `LINK` flag for `[a]:` variants, `**:` child targets
  (D-95 (a)), `SIDE_*` flags, and `State` as `u128` behind a private
  field (the `u32` has one free bit left); `data-checked` must no longer
  outrank `focus-visible` (wrong border on a focused checked checkbox and
  switch); the retarget rank of `:is(.parent *)[data-slot=s]` is one class
  too high.
- The control-plus-label row (one type for checkbox, switch and radio
  item: hit area, toggle, `widget_info`, label through the Label text
  path, per-control gap in `Metrics`).
- Part opacity through one helper (`Painter::set_opacity`; conformance 2.2,
  implementation notes 3)
  and `Theme::apply` setting `disabled_alpha` from the style.
- Text placement as Blink (maintainer, 2026-10-20; conformance 2.4): baseline
  = line top + floor((line height − (A + D)) / 2) + A, with ascent A and
  descent D each rounded to whole device pixels and the line gap unused
  when a line height is set; `<input>`-type fields round the half above
  up. A and D come from the font's own tables (typo metrics if
  `USE_TYPO_METRICS`, else `hhea`), generic, with no per-font table. The
  per-font y correction stays available (a `FontFiles` field, from
  `FONTS.tsv`), 0 for every font by default; only fonts whose capitals
  still differ visibly from Chrome get a value (today oxanium, measured
  again on the new path). The measurement test compares against a Chrome
  reference render, not against centred capitals. Every text snapshot
  changes once.
- One "press of this pass" helper for the overlay stack and the hover
  timer, a group-id helper, the content-height row, the animated panel
  height for accordion and collapsible.
- The crate `ScrollArea` before the first list that scrolls (D-62).
- App variants and app sizes (rules, axiom 8; implementation notes 18):
  the app case on every variant and size enum, a variant enum with the
  single case `Default` where a component has none, `Theme::variant` and
  `Theme::size`, the theme's map and the lookup helper; with them the app
  colours of the recipe-completeness design (`Role::custom`, the theme's
  light and dark map). They land before the first component that uses
  them; the 14 existing components get the app case in the cleanup.
- The memory-state type helper (rules, Memory-owned state; implementation
  notes 19): binding, initial value, `XState` `load` and `store`.
- Infrastructure defects:
  1. The recipe box paints `shadow-*` under the box, so it shows through
     transparent and translucent fills (vega light input 242 grey; dark
     `bg-input/30`); paint it outside the border box only.
  2. A press and release in the same pass is no outside press (overlay
     stack and hover timer).
  3. Anchored placement uses egui's `find_best_align`, so a panel at the
     window edge is clamped onto its trigger; shift then flip as bits.
  4. No focus return after a modal closes from inside or after a mouse
     open; a pressed widget gets no focus.
  5. `Theme::apply` never sets `disabled_alpha` from the style.
  6. `OverlayStack::bind` read "no state" as a caller change.
  7. The item registry in an `Area`'s first (disabled) pass registers
     every item as disabled.
  8. `widget_width` fills on NaN; the placeholder text style is cached
     from the global style, so `Ui`-scoped text styles are ignored.
  9. `State` (`u32`) has one free bit after `LINK`; `SIDE_*` needs the
     `u128` change.
  10. `tests/tokens.rs` flags every `accesskit::Role::` and scans test
      modules, unlike its documentation.
- The porting standard (`plugin/skills/port-component/SKILL.md`) follows
  the rules: "Interaction states" still describes the old disabled path
  (`ui_opacity` takes 1.0, `interaction_state(.., enabled, ..)`), and the
  Slider behaviour example lists PageUp/PageDown and a roving group that
  always wraps, where bits differs. Rule 5 makes a value that only a docs
  demo sets a caller parameter; under axiom 8 such a value is a look (an
  app variant) or the caller's layout, never a setter of its own.
- The build-ui references follow the new API:
  `plugin/skills/build-ui/references/icons.md` and `component-map.md`
  show an icon-only button as `ButtonSize::Icon`; it becomes
  `Button::icon_only(icon, name)`, and `ButtonSize` keeps the text sizes
  only.

### Stage 4 — Ports

One family per pull request, in dependency order. Each port follows
`port-component` and the rules, with its own snapshots, behaviour list,
`component-map.md` row and `CHANGES.md` entry. Very-large components come
last.

## Porting process

How a port and a shared step use the rules
(`2026-10-18-api-rules.md`), the conformance spec
(`2026-10-18-conformance.md`), the implementation notes
(`2026-10-18-implementation-notes.md`) and the decisions
(`2026-10-18-api-decisions.md`).

### Reading order

- A port reads the rules first, then, in the decisions document, its
  family bundle (D-109 to D-126; the title names the components) and its
  verdict entry if it has one (D-103 to D-108). Their default proposals
  apply until the family PR decides. Other D-entries it opens only for
  options and reasons.
- A rule states the maintainer's decision, also where it differs from
  the entry's recommendation. Where an entry's text and a rule differ,
  the rule wins, and the entry's **Consequence** says what changed. A
  rule that rests on a deferred bundle line stays a draft until the
  family PR decides.
- Before a component gets its own answer, the general rule is applied to
  it. Where that gives an answer, the answer stands, and the D-entry
  records it as a Consequence.
- The porting standard (`plugin/skills/port-component/SKILL.md`) is the
  second read. Where it and the rules differ, the rules win; the cleanup
  brings the standard in line (Stage 3).
- **Open (M-n)** marks a choice the principles do not settle. It lists
  the options and a recommendation; the recommendation applies until the
  maintainer decides.

### Shared code

- Code a second component needs is shared, not copied. A port that
  writes what another component already has, or what a component the
  plan names next will need, extracts a crate-private helper in the same
  PR. A helper that exists is called, never restated. A copy is a
  defect.
- Shared code has one owner. Infrastructure several components use,
  generator output and icon slots included, lands in its own step before
  the first component that uses it. A port never ships a known-wrong
  look while it waits for one. A part that one component draws for
  another is built by the shared step when components of one step both
  need it.

### Tests and fixtures

- Snapshot fixtures copy the upstream demo's props (a demo's
  `align="start"` becomes `.align(..)` with start alignment).
- A fixture whose content clips at the window edge is wrong, and so is
  one that sidesteps a case a rule names (a wider window so a tooltip
  need not flip, a fixed height so a separator need not stretch).
- Per-style snapshots show every part a rule styles (a control's own
  label).
- Pure logic (the overlay stack, the item registry) gets unit tests.
- Per-pass mechanisms are tested under both app loops (`run_ui` and an
  own `begin_pass`/`end_pass` loop), from a root `Ui`, not an `Area`
  whose first pass is a disabled sizing pass.
- Behaviour tests send input as users and slow frames do: a press and
  its release in one pass, a pointer path as well as an accesskit
  action, a trigger at the window edge, every way an overlay closes.
- Demo and reference screens use only the components' and layout
  helpers' own responsive behaviour, never a width test of their own.
  Their snapshot fixtures, and those of every component with a
  responsive class, cover a mobile width (390 px) and a desktop width
  (at least 1024 px).
- The README's settings-screen image shows the desktop render (at least
  1024 px) and the phone render (390 px) side by side, both from the
  tested fixtures (maintainer, 2026-10-20).

## Constraints

- **One build at a time.** The machine's 4-core cap counts every build on
  the box together; only one agent builds or runs gates at a time.
  Read-only agents (survey, review) run alongside.
- **Memory.** All sessions share one 25 GiB cgroup; snapshot test runs in
  parallel worktrees risk an OOM kill in a sibling session.
- **Disk.** Each worktree gets its own cargo target directory on the
  scratch disk; keep the number of live worktrees small and remove them
  after merge.
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
- No `#[deprecated]` aliases or forwards before 1.0: renames, removals and
  changed signatures break at once, each with a CHANGES.md upgrade note
  (maintainer, 2026-10-19; replaces the one-release alias rule).
- The cleanup shapes the APIs for the missing features of the existing 14
  but does not add the features, unless the correct shape needs them.

## Left to the builder

- Family boundaries and order, from the survey.
- The survey report format beyond the items listed.
- Whether a shared step is part of the cleanup or runs just before the
  first family that needs it.
- How many read-only agents run at once.
