# Port All Components Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Survey all 61 shadcn-svelte components, settle one set of API
rules with the maintainer, clean up the 14 existing components to those
rules (0.7.0), then port the rest family by family.

**Architecture:** Stage 1 (survey) and stage 2 (rules) are read-only
research and writing, done by many parallel subagents and merged by the
controller; they are planned in full here. Stages 3 (cleanup) and 4
(ports) depend on the survey and the maintainer's decisions; they are
outlined here and get their own detailed plans when stage 2 is approved.

**Tech Stack:** Rust, egui 0.34 (source at
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3`),
shadcn-svelte at `tools/stylegen/PIN` (493481fa), bits-ui 2.19.2,
`tools/stylegen`.

**Spec:** `docs/superpowers/specs/2026-10-18-port-all-components-design.md`

## Global Constraints

- Consistency and idiomatic egui and Rust come first; correctness beats
  minimal breakage.
- Before 1.0 the API may break whenever a decision needs it; every break
  gets a `CHANGES.md` entry that tells a user how to upgrade.
- Where upstream is inconsistent or the rules leave two reasonable answers,
  agents never choose: the case goes into the maintainer's decision list.
- Port from source at `PIN` and bits-ui 2.19.2, never from memory, a
  screenshot, or the bits-ui docs site (it documents the latest release).
- No new runtime dependency without the maintainer's consent.
- One build at a time on the machine (`--jobs 4`, `--test-threads 4`);
  read-only agents may run alongside.
- No data in `/tmp`; scratch work goes under
  `/scratch/oetiker/claude-tmp/shadcn-egui-survey/`.
- The repository is public: committed files hold no private names or local
  paths. Survey reports in scratch may hold local paths; the merged survey
  committed to the repo may not.
- Generated files are never edited by hand.
- Merging pull requests is the maintainer's.

## Review Focus

- **A report that sketches from memory.** An agent "knows" shadcn and
  writes an API without reading the files at `PIN`. Expected: every report
  cites the upstream files it read; Task 4 rejects a report without
  citations.
- **A sketch that copies Svelte composition.** `<DropdownMenu.Root><.Trigger>
  <.Content>` turned into seven structs. Expected: each sketch names the
  egui idiom it follows (from Task 2's catalogue) or says there is none.
- **A silent choice.** A sketch picks `&mut usize` over `&mut T`, or
  `side` over `orientation`, without listing it. Expected: every choice
  point is listed in the report; Task 4 cross-checks sketches of related
  components for unlisted divergence.
- **Behaviour taken from the docs site.** bits-ui's site documents a newer
  release than 2.19.2. Expected: behaviour lists cite files under the
  `bits-ui@2.19.2` tag.
- **An effort guess with no reason.** "small" for calendar. Expected:
  every estimate names what drives it (new primitive, generator gap,
  dependency, interaction model).

---

## Stage 1 — Survey

### Task 1: Prepare the sources

**Files:**
- Create: `/scratch/oetiker/claude-tmp/shadcn-egui-survey/sources.md`
- Use: `/scratch/oetiker/claude-tmp/shadcn-egui-upstream/{shadcn-svelte,shadcn-ui}`

**Interfaces:**
- Produces: `SURVEY=/scratch/oetiker/claude-tmp/shadcn-egui-survey`,
  `BITS=$SURVEY/bits-ui` (checkout of tag `bits-ui@2.19.2`), and
  `sources.md` listing every path an agent reads.

- [ ] **Step 1: Check the upstream checkouts are at the pins**

Run: `git -C /scratch/oetiker/claude-tmp/shadcn-egui-upstream/shadcn-svelte rev-parse HEAD` and the same for `shadcn-ui`.
Expected: `493481fab94f…` and `c2a67849be26…`, equal to `tools/stylegen/PIN` and `PIN_REACT`.

- [ ] **Step 2: Clone bits-ui at the locked version**

```bash
git clone --depth 1 --branch bits-ui@2.19.2 https://github.com/huntabyte/bits-ui $SURVEY/bits-ui
```
Expected: `$SURVEY/bits-ui/packages/bits-ui/src/lib/bits/` exists.

- [ ] **Step 3: Write `sources.md`**

List: the shadcn-svelte `ui/`, `examples/` and `styles/style-*.css`
directories; `$BITS/packages/bits-ui/src/lib/bits/`; the egui source path;
the crate's `src/components/`; `plugin/skills/port-component/` and
`plugin/skills/build-ui/references/component-map.md`. No commit (scratch).

### Task 2: egui idiom catalogue (one read-only agent)

**Files:**
- Create: `$SURVEY/egui-idioms.md`

**Interfaces:**
- Produces: `egui-idioms.md`, one section per topic, each with the egui
  0.34.3 item and file it comes from. The cluster agents (Task 3) cite its
  section names.

- [ ] **Step 1: Dispatch the agent** with the topics: widget builders and
  `impl Widget`; `ui.add` vs `show` containers and `InnerResponse`;
  selection (`ComboBox`, `selectable_value`, `selectable_label`,
  `RadioButton`); IDs (`id_salt`, `Id::new`, `push_id`); popups, menus and
  modals (`Popup`, `menu`, `Modal`, `Area`); `Atom`/`AtomLayout` content;
  `Response` extension methods (`on_hover_text`, `context_menu`);
  `Orientation`/`Direction` types; text and `WidgetText`; enabled state
  (`add_enabled`, `Ui::disable`); `widget_info` / accessibility. For each:
  the idiom, a 3-line usage example, and the file it is defined in.

- [ ] **Step 2: Check the output**

Expected: every section names an item that exists:
`grep -rn "pub fn <name>\|pub struct <name>" <egui src>` finds each one.

### Task 3: Component survey (19 read-only agents, two batches)

**Files:**
- Create: `$SURVEY/reports/<cluster-nn>.md`, one per cluster.

**Interfaces:**
- Consumes: `sources.md`, `egui-idioms.md`.
- Produces: one report per cluster with, per component, these headings
  exactly: `Sources`, `Markers`, `States`, `Generator gaps`,
  `Depends on`, `Behaviour`, `API sketch`, `Choice points`, `Effort`,
  `Verdict` (`port` / `very large` / `not applicable in egui` /
  `needs dependency: <crate>`), `Existing-API drift` (ported components
  only).

Clusters (preliminary kinship, not the final families):

| # | Components |
|---|---|
| 01 | button, button-group, toggle, toggle-group |
| 02 | badge, kbd, spinner, skeleton |
| 03 | card, item, empty, aspect-ratio |
| 04 | alert, progress, separator, avatar |
| 05 | checkbox, radio-group, switch, slider |
| 06 | input, textarea, input-otp, input-group |
| 07 | label, field, form, native-select |
| 08 | select, command |
| 09 | dropdown-menu, context-menu, menubar |
| 10 | popover, hover-card, tooltip |
| 11 | dialog, alert-dialog, sheet, drawer |
| 12 | tabs, accordion, collapsible |
| 13 | navigation-menu, breadcrumb, pagination |
| 14 | sidebar, scroll-area, resizable |
| 15 | calendar, range-calendar |
| 16 | table, carousel, sonner |
| 17 | chart |
| 18 | attachment, bubble, marker |
| 19 | message, message-scroller, questionnaire |

- [ ] **Step 1: Write the agent prompt** at `$SURVEY/survey-prompt.md`.
  It names the cluster's components, the report headings above, and these
  rules: read only, no cargo, no edits outside `$SURVEY/reports/`; cite
  every upstream file read (path under the pinned checkouts); read the
  docs examples in `registry/examples/` for each component; `Generator
  gaps` lists classes in the component's parts that
  `tools/stylegen/reports/vega.md` and the generator's translation tables
  (`tools/stylegen/src/`) do not handle, marked "unsure" when not
  decidable by reading; the `API sketch` cites the `egui-idioms.md`
  section it follows; every place the sketch picks one of several
  reasonable answers goes under `Choice points` with the options; for a
  ported component, `Existing-API drift` compares the crate's API with
  the sketch and with `egui-idioms.md`; `Effort` names its drivers.
- [ ] **Step 2: Dispatch batch A** (clusters 01–10), all in one message.
- [ ] **Step 3: Dispatch batch B** (clusters 11–19) when batch A has
  returned (keeps the session's concurrent agents near ten).
- [ ] **Step 4: Check every report**

Run: `for f in $SURVEY/reports/*.md; do for h in Sources Markers States "Generator gaps" "Depends on" Behaviour "API sketch" "Choice points" Effort Verdict; do grep -q "$h" "$f" || echo "$f: missing $h"; done; done`
Expected: no output, and 19 files. A report whose `Sources` cites no file
under the pinned checkouts is sent back to its agent (SendMessage), not
rewritten by the controller.

### Task 4: Generator gap scan (one build, the build slot)

**Files:**
- Use a throwaway detached worktree (no branch):
  `git worktree add --detach /scratch/oetiker/claude-worktrees/shadcn-egui-gapscan main`

**Interfaces:**
- Produces: `$SURVEY/generator-gaps.md`: per component, the classes
  stylegen reports as unsupported, and every generator stop (a missing
  `extract::BRANCHES` entry, an unknown marker) with its message.

- [ ] **Step 1:** Add every unported component to the worktree's
  `tools/stylegen/markers.txt`, with the parts from the cluster reports'
  `Markers` sections (file name `<component>.rs`).
- [ ] **Step 2:** Run
  `cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- generate /scratch/oetiker/claude-tmp/shadcn-egui-upstream/shadcn-svelte /scratch/oetiker/claude-tmp/shadcn-egui-upstream/shadcn-ui .`
  For each stop, record the message, comment out that component's line,
  run again. Expected: the run finishes.
- [ ] **Step 3:** Collect the unsupported classes per component from
  `tools/stylegen/reports/*.md` into `generator-gaps.md`, and compare them
  with the reports' `Generator gaps` sections; note where an agent was
  wrong.
- [ ] **Step 4:** Remove the worktree with `git worktree remove --force`
  (its edits are throwaway; the maintainer runs it if auto mode refuses),
  and its target dir `/scratch/oetiker/cargo-target-shadcn-egui-gapscan`
  if one was created (ask first).

### Task 5: Merge into the survey

**Files:**
- Create: `docs/superpowers/specs/2026-10-18-component-survey.md`

**Interfaces:**
- Consumes: the 19 reports, `egui-idioms.md`, `generator-gaps.md`.
- Produces: the survey, with the sections: `Families` (final grouping and
  order), `Dependency order` (a list, shared primitives first),
  `Shared primitives` (each with the components that need it),
  `Generator work`, `Verdicts` (very large, not applicable, needs
  dependency), `Existing-API drift` (all 14), `Choice points` (every
  choice point of every report, deduplicated by topic).

- [ ] **Step 1: Write the survey.** No local paths, no private names
  (public repository).
- [ ] **Step 2: Cross-check related sketches.** For each topic (selection,
  sub-parts, containers, IDs, orientation, sizing, state flags, overlays,
  menus), list how each sketch handles it. A divergence that no report
  lists as a choice point is added to `Choice points` with both sketches
  as options.
- [ ] **Step 3: Check coverage**

Every one of the 61 directory names under `registry/ui/` appears in
`Families` or `Verdicts`:
`for c in $(ls <svelte>/docs/src/lib/registry/ui); do grep -q "\b$c\b" docs/superpowers/specs/2026-10-18-component-survey.md || echo missing $c; done`
Expected: no output.
- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-10-18-component-survey.md
git commit -m "Survey of all 61 shadcn-svelte components"
```

## Stage 2 — API rules and decisions

### Task 6: Draft the rules and the decision list

**Files:**
- Create: `plugin/skills/port-component/references/api-rules.md` (lands
  with the cleanup; its rust fences describe the new API)
- Create: `docs/superpowers/specs/2026-10-18-api-decisions.md`

**Interfaces:**
- Consumes: the survey (Task 5), `egui-idioms.md`.
- Produces: `api-rules.md` with one section per topic of Task 5 Step 2,
  each stating the rule, the egui idiom it follows, and a short rust
  example; `api-decisions.md` with one numbered entry per open case: the
  case, the options, the components affected, the recommendation and the
  reason, and an empty `Decision:` line for the maintainer.

- [ ] **Step 1: Write both documents.** A rule that depends on an open
  decision says so and names its entry number.
- [ ] **Step 2: Mark the fences.** Rust fences in `api-rules.md` are
  ```` ```rust,ignore ```` until stage 3 makes them compile; the cleanup
  removes every `ignore`.
- [ ] **Step 3: Commit** both files.

### Task 7: Adversarial review of the rules

- [ ] **Step 1:** Dispatch a fresh agent to attack `api-rules.md` and
  `api-decisions.md` against the survey and `egui-idioms.md`: rules that
  contradict an egui idiom without a reason, rules two components cannot
  both follow, cases decided in the rules that belong in the decision
  list, families the rules do not cover.
- [ ] **Step 2:** Fix what holds; add new open cases to the decision list.
  Commit.

### Task 8: Maintainer decision pass

- [ ] **Step 1:** Hand the maintainer `api-decisions.md` and `api-rules.md`
  in one message, most important case first.
- [ ] **Step 2:** Write the decisions into both documents. Commit. Open a
  pull request with the design, survey, rules draft and decisions (label
  `no-changelog`).

## Stage 3 — Cleanup (outline; own plan after Task 8)

- The 14 components follow `api-rules.md`. Renames keep a `#[deprecated]`
  alias for one release; changed signatures break.
- Shapes leave room for the missing features of the 14; a feature joins
  the cleanup only where the correct shape needs it.
- Shared primitives and generator support from the survey land before the
  families that need them.
- Gallery, tests, snapshots, skills (doctests), `component-map.md` (its
  "Not yet ported" list is incomplete today) and a `CHANGES.md` migration
  entry change with it. `api-rules.md` loses its `ignore` marks.
- Released as 0.7.0 through Create release PR (feature).

## Stage 4 — Ports (outline; one plan per family)

- One pull request per family in the survey's order, through
  `port-component` and `api-rules.md`. Very-large components last.
- Only one pull request that changes stylegen output is open at a time;
  while the maintainer reviews it, read-only agents prepare the next
  family.
- New open cases go into a new decision list, decided in one pass.
