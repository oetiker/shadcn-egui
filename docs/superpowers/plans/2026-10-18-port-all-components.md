# Port All Components Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Survey all 61 shadcn-svelte components, settle one set of API
rules with the maintainer, clean up the existing crate API to those rules
(0.7.0), then port the rest family by family.

**Architecture:** Stage 1 (survey) and stage 2 (rules) are read-only
research and writing, done by parallel subagents and merged by the
controller; they are planned in full here. Stages 3 (cleanup) and 4
(ports) depend on the survey and the maintainer's decisions; they are
outlined here and get their own detailed plans after stage 2 merges.

**Tech Stack:** Rust, egui 0.34.3 (with emath and epaint 0.34.3),
shadcn-svelte at `tools/stylegen/PIN` (493481fa), shadcn-ui at
`PIN_REACT` (c2a67849), bits-ui 2.19.2, `tools/stylegen`.

**Spec:** `docs/superpowers/specs/2026-10-18-port-all-components-design.md`

## Paths

The repository is public, so this plan names no local path. The
controller fixes these once, outside the repository, and writes the
absolute paths into every command and every agent prompt (shell variables
do not survive between Bash calls):

- `SURVEY`: a scratch directory for the survey's working files.
- `UPSTREAM`: the scratch directory holding the pinned `shadcn-svelte`
  and `shadcn-ui` checkouts.
- `EGUI`: the cargo registry sources of `egui-0.34.3`, `emath-0.34.3`,
  `epaint-0.34.3`.
- `WT`: the throwaway worktree of Task 4, under the worktree root that
  `CLAUDE.md` names for this machine.

## Global Constraints

- Consistency and idiomatic egui and Rust come first; correctness beats
  minimal breakage.
- Before 1.0 the API may break whenever a decision needs it; every break
  gets a `CHANGES.md` entry that tells a user how to upgrade.
- Where upstream is inconsistent or the rules leave two reasonable answers,
  agents and the controller never choose: the case goes into the
  maintainer's decision list.
- Port from source at the pins: shadcn-svelte at `PIN`, bits-ui 2.19.2,
  and the other primitives at the versions in shadcn-svelte's
  `pnpm-lock.yaml` at `PIN`. Never from memory, a screenshot, or a docs
  site (they document the latest release).
- No new runtime dependency without the maintainer's consent.
- **One build on the whole machine at a time** (`--jobs 4`,
  `--test-threads 4`). Before any cargo build, check that no other cargo
  or rustc process runs on the machine (`pgrep -fa 'cargo|rustc'`, any
  user, any project); wait until none does. Read-only agents may run
  alongside.
- Scratch files never go to `/tmp`.
- Before every commit: `git diff main -- docs plugin | grep -v 'grep -nE' | grep -nE '/scratch/|/home/|claude-tmp|claude-worktrees|~/\.'`
  prints nothing.
- Generated files are never edited by hand.
- Destructive commands (`git worktree remove --force`, `rm -r`) are asked
  for first. Merging pull requests is the maintainer's.

## Review Focus

- **A report that sketches from memory.** Expected: every component's
  `Sources` cites the files read under the pinned checkouts; Task 3 Step 4
  sends a report without citations back to its agent.
- **A sketch that copies Svelte composition.** Expected: each sketch
  cites the `egui-idioms.md` section it follows, or says there is none.
- **A silent choice.** Expected: every choice point carries an ID and a
  topic; Task 5 Step 2 cross-checks the topic questionnaire answers of all
  components; Task 6 Step 3 proves by grep that every ID ends as a
  decision or a rule with a "settled because" line.
- **Behaviour taken from a docs site.** Expected: `Behaviour` cites files
  under the `bits-ui@2.19.2` checkout or the pinned primitive tarball.
- **An effort guess with no reason.** Expected: every `Effort` names what
  drives it (new primitive, generator gap, dependency, interaction model).

---

## Stage 1 — Survey

One report section per upstream component. The agents cover clusters of
related components (19 agents for 61 components) instead of one agent per
component as the spec says: a cluster agent sees siblings that share a
primitive, and the fixed topic questionnaire (Task 3) keeps clusters
comparable. This is a controller call, listed for the maintainer.

### Task 1: Prepare the sources

**Files:**
- Create: `SURVEY/sources.md`, `SURVEY/pkgs/`

**Interfaces:**
- Produces: `SURVEY/bits-ui` (tag `bits-ui@2.19.2`), one unpacked npm
  tarball per non-bits-ui primitive under `SURVEY/pkgs/<name>-<version>/`,
  and `sources.md` listing every path an agent reads.

- [ ] **Step 1: Check the upstream checkouts**

Run: `git -C UPSTREAM/shadcn-svelte rev-parse HEAD` and the same for
`shadcn-ui`.
Expected: `493481fab94f68b8982949bc8a37eede786f2462` and
`c2a67849be260701852b0977ba15eb5f3a0ce2a4`.

- [ ] **Step 2: Clone bits-ui at the locked version**

`git clone --depth 1 --branch bits-ui@2.19.2 https://github.com/huntabyte/bits-ui SURVEY/bits-ui`
Expected: `SURVEY/bits-ui/packages/bits-ui/src/lib/bits/` exists.

- [ ] **Step 3: Fetch the other primitives at their locked versions**

The versions at `PIN` (from `UPSTREAM/shadcn-svelte/pnpm-lock.yaml`):
`vaul-svelte@1.0.0-next.7`, `formsnap@2.0.1`,
`sveltekit-superforms@2.30.0`, `paneforge@1.0.2`,
`embla-carousel-svelte@8.6.0`, `layerchart@2.0.3`,
`svelte-sonner@1.2.1`, `@internationalized/date@3.12.0`. Re-check each
with `grep -m1 "^  '\?<name>@" pnpm-lock.yaml`. Fetch each with
`pnpm pack <name>@<version> --pack-destination SURVEY/pkgs` (or the
registry tarball URL) and unpack it into its own empty directory.
`@shadcn-svelte/primitives` is in the shadcn-svelte checkout
(`packages/primitives`). The tarballs are data: nothing in them runs.

- [ ] **Step 4: Write `sources.md`** with: the shadcn-svelte `ui/`,
  `examples/` and `styles/style-*.css` paths; `packages/primitives`;
  bits-ui `src/lib/bits/`; each unpacked package; the `EGUI` paths; the
  crate's `src/components/`, `src/layout.rs`, `src/components/shared.rs`
  and `src/components/icon.rs`; `plugin/skills/port-component/` and
  `plugin/skills/build-ui/references/component-map.md`.

### Task 2: egui idiom catalogue (one read-only agent)

**Files:**
- Create: `SURVEY/egui-idioms.md`

**Interfaces:**
- Produces: `egui-idioms.md`, one `## <topic>` section per topic below,
  each with the idiom, a usage example of at most 5 lines, and the crate
  and file the item is defined in. Task 3 waits until Step 2 passes.

- [ ] **Step 1: Dispatch the agent** with the topics:
  widget builders and `impl Widget`; containers (`show`, `InnerResponse`,
  `Frame`, `UiBuilder`, `Sides`); selection (`ComboBox`,
  `selectable_value`, `selectable_label`, `RadioButton`); open state
  (`Window::open(&mut bool)`, `CollapsingState`, `CollapsingHeader`,
  state in `Memory` / `data_mut`); IDs (`id_salt`, `Id::new`, `push_id`);
  popups, menus and modals (`Popup`, `menu_button`, `MenuBar`, `Modal`,
  `Area`) and their anchoring (`RectAlign`); `Atom` / `AtomLayout`
  content; `Response` methods (`on_hover_text`, `context_menu`,
  `changed`, `mark_changed`); direction and layout (`Direction`,
  `Layout`); `ScrollArea`; `TextEdit`; `Slider`, `SliderOrientation`,
  `DragValue`; text (`WidgetText`, `RichText`); enabled state
  (`add_enabled`, `Ui::disable`); focus and keyboard (`request_focus`,
  `has_focus`, `memory.focused`); `widget_info` and accessibility.
  The agent writes "not in 0.34.3" for any topic item that does not exist.

- [ ] **Step 2: Check every named item exists**

For each item named in a section, run
`grep -rnE "pub (fn|struct|enum|trait|type|use) <name>\b" EGUI/egui-0.34.3/src EGUI/emath-0.34.3/src EGUI/epaint-0.34.3/src`.
Expected: a hit for every item not marked "not in 0.34.3". A miss goes
back to the agent.

### Task 3: Component survey (19 read-only agents, two batches)

**Files:**
- Create: `SURVEY/survey-prompt.md`, `SURVEY/reports/<nn>.md` (01–19)

**Interfaces:**
- Consumes: `sources.md`, `egui-idioms.md`.
- Produces: one report per cluster. Per component one
  `## <component>` section with these `### ` headings, exactly:
  `Sources`, `Markers`, `States`, `Generator gaps`, `Depends on`,
  `Behaviour`, `API sketch`, `Topics`, `Choice points`, `Effort`,
  `Verdict`; ported components also `Existing-API drift`.

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

Ported components: badge, button, card, checkbox, dialog, input, label,
popover, select, separator, slider, switch, tabs, tooltip. Cluster 03 also
covers `layout::{row, wrap_row, vstack, space_between, card, form_row}`,
cluster 07 `form_row` against `field`, cluster 02 `components::icon`
(`IconView`), and cluster 04 the public helpers of `components::shared`,
all under `Existing-API drift`.

- [ ] **Step 1: Write `survey-prompt.md`.** It states:
  - read only; no cargo; no edits outside `SURVEY/reports/`; absolute
    paths given in the prompt;
  - `Sources`: every file read, by path under the pinned checkouts or
    `SURVEY/pkgs`; read the docs examples in `registry/examples/` too;
  - `Markers`: every `cn-*` marker and `slot:` part, cross-checked with
    `grep -o "cn-<component>[a-z0-9-]*" styles/style-vega.css`;
  - `Generator gaps`: classes in the parts that the translation tables in
    `tools/stylegen/src/` (`utility.rs`, `variant.rs`, `class.rs`) do not
    handle, each marked "sure" or "unsure" (Task 4 measures the truth;
    the reports under `tools/stylegen/reports/` cover only today's
    markers and say nothing about new ones);
  - `Behaviour`: the primitive and its version, then keyboard, focus
    order, pointer, disabled, degenerate input, accessibility role; each
    item cites a file of the pinned primitive;
  - `API sketch`: the egui API, citing the `egui-idioms.md` section it
    follows, or "no egui idiom";
  - `Topics`: answer every question below, or "n/a":
    1. selection: value type, single or multiple, how items are supplied,
       what the call returns;
    2. open state: who owns it (`&mut bool`, egui memory, none);
    3. overlay: anchor, side and align, dismissal;
    4. sub-parts: how the Svelte child components map to egui;
    5. container: `ui.add` or `show`, and the return type;
    6. identity: what makes the ID;
    7. orientation and direction;
    8. sizing: `width` / `max_width` and size variants;
    9. state flags: `enabled`, `invalid`, others;
    10. shortcut function: name and return type;
  - `Choice points`: each `CP-<nn>-<k>` with its topic number, the
    options, and the sketch's pick; every place the sketch picked one of
    several reasonable answers is one;
  - `Effort`: small / medium / large / very large, and its drivers;
  - `Verdict`: `port`, `very large`, `not applicable in egui`, or
    `needs dependency: <crate>`.
- [ ] **Step 2: Dispatch batch A** (clusters 01–10) in one message.
- [ ] **Step 3: Dispatch batch B** (clusters 11–19) when batch A has
  returned.
- [ ] **Step 4: Check the reports**

Run, for each report, a count of `^## ` sections against the cluster's
component count, and of each `^### <heading>$` against the same count
(`Existing-API drift` against the ported count), and `ls SURVEY/reports | wc -l`.
Expected: all counts match, 19 files. A section whose `Sources` names no
path under the pinned checkouts or `SURVEY/pkgs` goes back to its agent
(SendMessage); the controller does not rewrite reports.

### Task 4: Generator gap scan (the one build of stage 1)

**Files:**
- Create: `SURVEY/generator-gaps.md`
- Use: `WT`, created with
  `git worktree add --detach WT main` (no branch)

**Interfaces:**
- Produces: `generator-gaps.md`: per component and per exported style,
  the unsupported classes; every stop with its message. `sera` is listed
  apart (it is not exported and already has unsupported classes).

- [ ] **Step 1: Build the marker lines mechanically.** For each unported
  component, take the markers from
  `grep -oh "cn-<component>[a-z0-9-]*" UPSTREAM/shadcn-svelte/docs/src/lib/registry/styles/style-*.css | sort -u`
  (excluding `-aria` markers), compare with the report's `Markers`, and
  add a `slot:` part only where `<name>.svelte` has exactly one class
  string without a marker. Write the lines into `WT/tools/stylegen/markers.txt`.
  Check: `git -C WT status --short` shows `markers.txt` modified.
- [ ] **Step 2: Claim the build slot.** `pgrep -fa 'cargo|rustc'` prints
  nothing (any user, any project); otherwise wait.
- [ ] **Step 3: Run the generator against `WT` only**, as one command
  with absolute paths:
  `cd WT && cargo run --manifest-path WT/tools/stylegen/Cargo.toml --jobs 4 -- generate UPSTREAM/shadcn-svelte UPSTREAM/shadcn-ui WT`
  A **stop** is an error printed before any report is written (from
  `extract::part_strings`, the svelte sources, `resolve::parts` or the
  exception parts). For a stop, record the message, comment out that
  component's line, run again. The end `<style> is in EXPORT but has N
  unsupported classes` is the expected result, not a stop: the reports
  are written.
  Check: `git -C WT status --short` shows changed `tools/stylegen/reports/`
  and the primary checkout's `git status` is unchanged.
- [ ] **Step 4: Collect the gaps** from `WT/tools/stylegen/reports/*.md`,
  exported styles per component, `sera` apart, into `generator-gaps.md`.
  Compare with the reports' `Generator gaps` and note where an agent was
  wrong.
- [ ] **Step 5: Clean up.** Ask the maintainer, then
  `git worktree remove --force WT` and remove its cargo target directory.

### Task 5: Merge into the survey

**Files:**
- Create: `docs/superpowers/specs/2026-10-18-component-survey.md`

**Interfaces:**
- Consumes: the 19 reports, `egui-idioms.md`, `generator-gaps.md`.
- Produces: the survey with `## ` sections: `Families` (final grouping
  and order), `Dependency order`, `Shared primitives`, `Generator work`,
  `Verdicts`, `Existing-API drift` (the 14 components and the other public
  API named in Task 3), `Topics` (one table per topic of the
  questionnaire: component, answer), `Choice points` (every `CP-` ID,
  grouped by topic).

- [ ] **Step 1: Cross-check per topic, one read-only agent per topic**
  (10 agents, in one message). Each reads the `Topics` answers and
  choice points of all 61 components for its topic and returns: the
  answers as a table, every divergence between components, and the
  divergences that no `CP-` covers, each written as a new choice point
  `CP-T<topic>-<k>` with the sketches as options.
- [ ] **Step 2: Write the survey** from the reports and the topic
  results. No local paths.
- [ ] **Step 3: Check coverage**

Extract the `Families` and `Verdicts` sections, then for each directory
name `c` under `UPSTREAM/shadcn-svelte/docs/src/lib/registry/ui/` run
`grep -qE "(^|[^a-z-])$c([^a-z-]|$)"` on them.
Expected: all 61 found. Every `CP-` ID of the reports appears in
`Choice points`.
- [ ] **Step 4: Commit** after the path check of Global Constraints.

## Stage 2 — API rules and decisions

### Task 6: Draft the rules and the decision list

**Files:**
- Create: `docs/superpowers/specs/2026-10-18-api-rules.md` (the draft;
  it moves into the plugin in stage 3)
- Create: `docs/superpowers/specs/2026-10-18-api-decisions.md`

**Interfaces:**
- Consumes: the survey, `egui-idioms.md`.
- Produces: `api-rules.md` with one section per questionnaire topic plus
  `Builder shape` and `Compound components` (how a Svelte compound
  component maps to egui), each with the rule, the egui idiom it follows
  and a short example; `api-decisions.md` with numbered entries `D-<n>`:
  the case, the options, the components affected, a recommendation with
  its reason, the `CP-` IDs it covers, and a line `Decision:` left empty.

- [ ] **Step 1: Write both documents.** A rule that depends on an open
  entry names it. Each rule that settles a `CP-` without a decision entry
  has a line `Settled because: <why only one answer is reasonable> (CP-…)`.
- [ ] **Step 2: Turn verdicts into decisions.** Every component whose
  verdict is not `port`, and every dependency, is a `D-` entry.
- [ ] **Step 3: Check that no choice is silent.** Every `CP-` ID of the
  survey appears in `api-decisions.md` or on a `Settled because` line.
  Expected: a script over the IDs prints none missing.
- [ ] **Step 4: Commit** after the path check.

### Task 7: Adversarial review of the rules

- [ ] **Step 1:** Dispatch a fresh agent to attack both documents against
  the survey and `egui-idioms.md`: rules that contradict an egui idiom
  without a reason, rules two components cannot both follow, `Settled
  because` lines that hide a real choice, families no rule covers.
- [ ] **Step 2:** Fix what holds; a finding that turns a settled point
  into a real choice becomes a `D-` entry. Findings rejected by the
  controller go into a `Rejected review findings` section of
  `api-decisions.md`, for the maintainer to see. Commit.

### Task 8: Maintainer decision pass

- [ ] **Step 1:** Hand the maintainer `api-decisions.md` (and the rules
  draft) in one message, most important entry first. Also list the
  controller's calls of this plan (clusters instead of one agent per
  component).
- [ ] **Step 2:** Write the decisions into both documents. An entry
  answered "your call" gets the choice made written back and is shown to
  the maintainer.
- [ ] **Step 3: Gate.** `grep -nE '^Decision: *$' docs/superpowers/specs/2026-10-18-api-decisions.md`
  prints nothing.
- [ ] **Step 4:** Claim the build slot, then run every gate in `CLAUDE.md`
  ("Build & verify"); all pass. Path check. Commit, push the branch,
  open a pull request with the design, plan, survey, rules draft and
  decisions (label `no-changelog`). Stage 3 starts only after the
  maintainer merges it.

## Stage 3 — Cleanup (outline; own plan after the stage 2 merge)

- The 14 components, `layout`, `components::icon` and the public
  `components::shared` helpers follow the rules. Renames keep a
  `#[deprecated]` alias for one release; changed signatures break.
- Shapes leave room for the missing features of the 14; a feature joins
  the cleanup only where the correct shape needs it.
- Shared primitives and generator support from the survey land before the
  families that need them.
- The rules move to `plugin/skills/port-component/references/api-rules.md`,
  linked from `SKILL.md`, included in `skill_docs` in
  `crates/shadcn-egui/src/lib.rs`, fences compiling. `build-ui` gets what
  a user of the crate needs from them.
- Gallery, tests, snapshots, skills, `component-map.md` (its "Not yet
  ported" list is incomplete today) and a `CHANGES.md` migration entry
  change with it.
- Released as 0.7.0 through Create release PR (feature).

## Stage 4 — Ports (outline; one plan per family)

- One pull request per family in the survey's order, through
  `port-component` and the rules. Very-large components last.
- Only one pull request that changes stylegen output is open at a time;
  while the maintainer reviews it, read-only agents prepare the next
  family.
- Every PR shows its snapshots and says plainly where no comparison with
  the shadcn-svelte docs page was made.
- New open cases go into a new decision list, decided in one pass.
