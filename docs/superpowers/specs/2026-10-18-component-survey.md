# Component survey: all 61 shadcn-svelte components

Stage 1 of the plan `docs/superpowers/plans/2026-10-18-port-all-components.md`
(spec `docs/superpowers/specs/2026-10-18-port-all-components-design.md`).
It merges 19 cluster reports, 11 topic cross-checks (T1–T10 are the
questionnaire topics, T11 the cross-cutting conventions), a generator gap
scan and an egui 0.34.3 idiom catalogue. Pins: shadcn-svelte `493481fa`,
shadcn-ui `c2a67849`, bits-ui 2.19.2, egui 0.34.3. Upstream files are named
`shadcn-svelte/...` (registry files as `ui/...`, `examples/...`,
`styles/...`), `bits-ui/...`; crate files by repository path.

This document decides nothing. Every API question is a choice point
(`CP-<cluster>-<k>` from the reports, `CP-T<topic>-<k>` from the
cross-checks); the maintainer settles them in stage 2. Sketch picks are
shown so the maintainer sees what a report assumed, not as answers. The
family grouping and order below is the builder's call under the spec
("Left to the builder"); the maintainer can change it.

Idiom references ("idioms §Selection" and the like) name sections of the
egui idiom catalogue: Widget builders and impl Widget; Containers; Selection;
Open state; IDs; Popups, menus and modals; Anchoring (RectAlign); Atom /
AtomLayout content; Response methods; Direction and layout; ScrollArea;
TextEdit; Slider; Text (WidgetText, RichText); Enabled state; Focus and
keyboard; widget_info and accessibility.

## Families

One family per port pull request, in this order. Existing components sit
in the family that adds their missing features; their API shape changes
in the stage 3 cleanup (0.7.0) first. "Needs" lists earlier families and
the shared primitives (next sections) that must land first.

| # | Family | Components | Needs | Notes |
|---|---|---|---|---|
| F1 | Display leaves | spinner, skeleton, kbd, progress, aspect-ratio, badge (exists), separator (exists), label (exists) | icon slot `spinner` and rotated icon painting; `Glyph` decision (CP-02-2) | badge spinner slot, separator struct; kbd host embedding rides on later hosts |
| F2 | Content blocks | card (exists), item, empty, alert, avatar | F1 (separator, icon) | image media through egui loaders, no new dependency |
| F3 | Buttons and groups | button (exists), toggle, button-group, toggle-group | F1; roving-focus helper; group-position protocol, per-corner radius | first users of the group protocol (CP-01-7) |
| F4 | Choice controls | checkbox (exists), switch (exists), radio-group, slider (exists) | F3 (roving focus); `IconSlots::minus` | indeterminate, second thumb, radio group |
| F5 | Text entry | input (exists), textarea, input-otp, input-group | F1 (kbd, spinner), F3 (button) | group recipe composition across Input, Textarea, Button |
| F6 | Forms | field, native-select, form | F4, F5, label | form: `not applicable in egui`, ships only a build-ui map row pointing to Field |
| F7 | Anchored overlays | popover (exists), tooltip (exists), hover-card | overlay frame helper; hover timer | sets the overlay shape (CP-10-1/2/3) the menus follow or not (CP-10-25) |
| F8 | Menus | dropdown-menu, context-menu, menubar | F7; menu primitive; list keyboard model; `chevron_right`, `minus` slots | one primitive, three wrappers |
| F9 | Modals | dialog (exists), alert-dialog, sheet, drawer | shared modal shell; edge `Side` | drawer without snap points and background scaling |
| F10 | Pickers | select (exists), command | F5 (input-group), F8 (list keyboard model), F9 (CommandDialog); fuzzy score; themed ScrollArea (F12) or raw egui | select rework: generic value, groups, labels, disabled items |
| F11 | Tabs and disclosure | tabs (exists), accordion, collapsible | F3 (roving focus); `chevron_up` and plain down-chevron slots; `OPEN` flag | vertical tabs, per-trigger disabled |
| F12 | Scroll and panes | scroll-area, resizable | ScrollStyle theming; pane splitter primitive | scroll-area may land earlier, with the cleanup, if select adopts it (CP-14-30) |
| F13 | Paging | breadcrumb, pagination | F3 (button recipes, roving focus); `chevron_left/right`, `more_horizontal` slots | |
| F14 | Data display | table, carousel | F13 (chevron slots); drag/fling/snap scroller | |
| F15 | Toasts | sonner | F1 (spinner); five toast icon slots; global toast queue | look lives in svelte-sonner CSS, few recipes |
| F16 | Chat parts | attachment, bubble, marker, message | F1, F2 (avatar), F3; shimmer painter; scroll-fade | bubble and message share one alignment context (CP-18-16, CP-19-6) |
| F17 | Questionnaire | questionnaire | F1 (kbd chip), F4 (indicators), F5 (input), F3 | large form state machine |
| F18 | Dates | calendar, range-calendar | date type (CP-15-1, CP-15-26); F10 (select for caption dropdowns); F3 | calendar effort is very large; verdict `port` only with a crate-owned date, else `needs dependency: jiff` |
| F19 | Sidebar | sidebar | F1 (skeleton), F5 (input), F7 (tooltip side), F9 (sheet), F11 (collapsible), F8 (dropdown-menu in demos); eight `sidebar-*` palette roles | very large |
| F20 | Very-large tail | navigation-menu, message-scroller, chart | navigation-menu: F7, F3, hover timer; message-scroller: F12 or raw `ScrollArea`, F16; chart: optional `egui_plot` (maintainer consent) | spec: very-large components come last or in a later project; message-scroller's follow-bottom subset is a `port` and could move into F16 |

## Dependency order

Prerequisites, earliest first. Each line names what it unblocks.

1. Stage 3 cleanup of the 14 existing components plus `layout`,
   `components::icon`, `components::shared` (0.7.0), after the stage 2
   rules. It fixes the anchors that every family uses: disabled form
   (CP-01-1, CP-T9-1/2), icon form (CP-01-3), setter form (CP-01-31),
   orientation type (CP-05-37), shortcut return (CP-08-13), overlay shape
   (CP-10-1/2/3, CP-10-25, CP-11-35), binding setter name (CP-14-43),
   flag budget (CP-18-30), container return (CP-T5-2), root id (CP-T6-1),
   width setter (CP-T8-1), text type (T11 G7).
2. `IconSlots` made `#[non_exhaustive]` and the combined slot list through
   icongen (CP-05-6, CP-12-33, CP-T11-13): before F1 (spinner), F4
   (minus), F8, F11, F13, F14, F15, F16, F18, F19.
3. `State` widened or budgeted (CP-18-30) and the generator gap families
   below: before F3 onward (most families add flags or variants).
4. Roving-focus helper in `shared.rs` (extracted from
   `crates/shadcn-egui/src/components/tabs.rs:185-252`): F3, F4, F8
   (menubar), F11, F13, F17, F20 (navigation-menu).
5. Group-position protocol and per-corner radius / single-side border in
   the shared painters: F3, then F5 (input-group), input-otp.
6. Overlay frame helper (from
   `crates/shadcn-egui/src/components/popover.rs:48-117`): F7, F8, F10.
7. List keyboard model (from `crates/shadcn-egui/src/components/select.rs:186-282`):
   F8, F10, F6 (native-select).
8. Modal shell (from `crates/shadcn-egui/src/components/dialog.rs`) and edge `Side`: F9, then
   F10 (CommandDialog), F19 (mobile sheet).
9. Themed `ScrollArea` / `ScrollStyle`: F10 (select list), F12,
   F20 (message-scroller), sheet bodies.
10. Date primitive: F18 only.
11. Family edges: F1 → F2, F3 → F4 → F6; F1 + F3 → F5 → F6, F10;
    F7 → F8 → F10; F9 → F10, F19; F3 → F11, F13 → F14; F2 + F3 → F16 →
    F20 (message-scroller); F10 → F18; F5 + F7 + F9 + F11 → F19.

## Shared primitives

| Primitive | Needed by | Today | CPs |
|---|---|---|---|
| Roving focus (one Tab stop, arrows, Home/End, wrap) | toggle-group, radio-group, tabs, accordion, menubar, navigation-menu, pagination, questionnaire | inline in `crates/shadcn-egui/src/components/tabs.rs:185-252` | CP-01-29, CP-05-21, CP-T7-3 |
| Group-position protocol (child knows first/middle/last, orientation) | button-group, toggle-group, input-group, input-otp | none; `State::IN_BUTTON_GROUP` exists but is never set | CP-01-7, CP-01-14, CP-T4-5, CP-T9-11 |
| Per-corner radius and single-side borders in painters | button-group, toggle-group, input-group, input-otp, sheet, drawer | `shared::box_shapes`/`ring_shape` take one `f32` radius (`crates/shadcn-egui/src/components/shared.rs:88-150`) | drift list |
| List keyboard model (highlight in memory, focus-lock filter, typeahead, skip disabled) | select, native-select, dropdown-menu, context-menu, menubar, command | inside `select.rs:186-282` | CP-09-1, CP-08-16, CP-08-17 |
| Menu primitive (frame, item row: indicator, icon, text, shortcut, chevron; sub-menus) | dropdown-menu, context-menu, menubar | none; `Theme::menu_theme()` exists | CP-09-1, CP-09-2, CP-09-17 |
| Anchored overlay frame and shape (`RectAlign`, offset, open state, return) | popover, hover-card, tooltip, menus, select, command popover, navigation-menu | `popover.rs:48-117` | CP-10-1, CP-10-2, CP-10-3, CP-10-25, CP-T3-1..4 |
| Hover-intent timer (open/close delay, grace, repaint) | hover-card, navigation-menu, tooltip (keyboard focus open) | none (egui `Tooltip` timing only) | CP-10-15, CP-T3-7 |
| Modal shell on `egui::Modal` (overlay, panel, header, close, footer) | dialog, alert-dialog, sheet, drawer, CommandDialog, sidebar mobile | `dialog.rs` (header private, `dialog.rs:164`) | CP-11-35, CP-11-2, CP-11-8 |
| Edge `Side` enum | sheet, drawer, sidebar | none | CP-11-18, CP-11-19, CP-14-13 |
| Shared `Orientation` enum | button-group, toggle-group, separator, slider, tabs, field, resizable, carousel, attachment | `SliderOrientation`, `TabsOrientation` | CP-05-37, CP-T7-1 |
| `IconSlots` additions (`minus`, `chevron_up`, `chevron_left`, `chevron_right`, `more_horizontal`, plain down chevron, `spinner`/loader, `sidebar`, `arrow_down`, four toast kinds) | checkbox, input-otp, menus, accordion, navigation-menu, breadcrumb, pagination, calendar, carousel, spinner, sonner, sidebar, message-scroller | `check`, `chevron_down`, `close` (`crates/shadcn-egui-core/src/icon.rs`) | CP-05-6, CP-12-33, CP-13-31, CP-15-36, CP-16-44, CP-T11-13 |
| Rotated icon painting and a `Glyph` (icon or spinner) slot | spinner, badge, button, attachment, marker, sonner | `paint_icon` cannot rotate (`components/icon.rs:25-42`) | CP-02-2, CP-01-32, CP-18-35 |
| Crate `Icon` inside `IntoAtoms` content | tabs, navigation items, alert-dialog buttons, native-select option, kbd | none | CP-T4-13, CP-02-12 |
| `State` widening and `data-open` as a state | nearly every new family | `State(u32)`, 9 free bits | CP-18-30, CP-T11-14, CP-T11-15 |
| Themed `ScrollArea` (transparent track, `bg-border` thumb) | scroll-area, select, command, message-scroller, sheet | theme sets no `ScrollStyle` | CP-14-26, CP-14-27, CP-T4-12 |
| Drag / fling / snap scroller | carousel (drawer drag is related) | none | CP-16-17..21 |
| Pane splitter layout (paneforge algorithm) | resizable | none | CP-14-32..41 |
| Date type, arithmetic, local "today", names | calendar, range-calendar | none | CP-15-1, CP-15-26, CP-T7-10 |
| Fuzzy score (port of `compute-command-score.ts`, pure) | command | none | CP-08-20, CP-08-21 |
| Global toast queue in `ctx` | sonner | none | CP-16-25 |
| Shimmer text painter, scroll-fade mask | attachment, marker (demos) | none | CP-18-8, CP-18-27 |
| Transcript scroll primitive (follow-bottom, anchoring, prepend) | message-scroller | none (`ScrollArea::stick_to_bottom` covers a subset) | CP-19-10, CP-19-13 |
| Context passing through the `Ui` stack | kbd, grouped button, item in menu, bubble from message, field to control | `UiStackInfo` used nowhere in the crate yet | CP-T9-11, CP-T7-2 |

## Generator work

Source: one `stylegen generate` run with marker lines for every unported
component, seven exported styles. Counts are unique (part, class) pairs
that block export.

Gap families, by pairs (all components):

| Pairs | Family | Components |
|---|---|---|
| 182 | `data-*` / `aria-*` attribute states (`data-[size]`, `data-[variant]`, `data-[side]`, `data-[spacing]`, `data-[state=…]`, `data-open`) | attachment, avatar, bubble, calendar, context-menu, drawer, dropdown-menu, field, form, menubar, message, message-scroller, navigation-menu, range-calendar, resizable, table, toggle, toggle-group |
| 155 | named groups `group-*/name`, `group-has-*/name` (only the `/tabs` forms translate today) | alert, alert-dialog, attachment, avatar, bubble, command, context-menu, drawer, dropdown-menu, field, input-group, item, marker, menubar, message, native-select, navigation-menu, questionnaire, toggle-group |
| 133 | `has-*` (child content) | alert, alert-dialog, attachment, bubble, button-group, field, input-group, input-otp, item, native-select, questionnaire, table |
| 108 | `*:` / `[&…]` child targets (`*:` is in the IGNORED list, so bubble variant colours drop without a diagnostic) | accordion, alert, bubble, button-group, calendar, empty, field, input-group, item, questionnaire, range-calendar, resizable, table |
| 71 | utilities with no translation rule | 24 components |
| 31 | spacing values (`size-full`, `-space-x-2`, `gap-[--spacing(var(--gap))]`, …) | 16 components |
| 22 | `in-*` (ancestor slot) | calendar, command, input-group, item, kbd, navigation-menu |
| 19 | pseudo elements (`before:`, `after:`, `placeholder:`, `selection:`) | drawer, marker, native-select, questionnaire |
| 18 | `first:` / `last:` / `not-last:` | accordion, field, input-otp, range-calendar |
| 11 | radius values (CSS-variable radii, `rounded-[inherit]`) | calendar, input-group, kbd, menubar, navigation-menu, range-calendar, scroll-area |
| 10 | `max-w-*` values | alert-dialog, attachment, bubble, message, navigation-menu |
| 6 | container query / breakpoint (`@md`) | field |
| 5 | `focus-within` | attachment, calendar, range-calendar |
| 4, 3, 1 | translate values; CSS variable definitions (calendar `--cell-radius`); one single-side border form (scroll-area) | bubble, questionnaire; calendar; scroll-area |

Not measured or partial:

- Stopped by the extractor (nothing measured): carousel
  (`carousel-content.svelte`, `class={cn(`), chart (`chart-tooltip.svelte`,
  `class={cn(`), pagination (`pagination-link.svelte`, `class={cn(`),
  sidebar (`sidebar-menu-action.svelte`, `showOnHover &&` in `cn()`). The
  extractor needs these forms (also carousel orientation ternaries).
- Markers with no class string at the pin, so unmeasured:
  `cn-accordion-trigger-icon` (two strings), `cn-calendar-dropdown-root`,
  `cn-calendar-caption`, `cn-calendar-caption-label`,
  `cn-context-menu-subcontent`, `cn-dropdown-menu-subcontent`,
  `cn-drawer-popup`, `cn-drawer-swipe-handle`, `cn-drawer-header-base`,
  `cn-drawer-footer-base`, `cn-navigation-menu-positioner`,
  `cn-navigation-menu-popup`, `cn-progress-track`, `cn-progress-label`,
  `cn-progress-value`. Markers sharing one string: `cn-input-otp` /
  `-input`, questionnaire choice label and shortcut.
- Unmarked files with several class strings (e.g. `calendar-month-select`,
  `range-calendar-month-select`, `carousel-content`) were not measured.
- Nothing to measure (no markers, no class strings): aspect-ratio,
  collapsible, sonner (its look is svelte-sonner CSS outside the style
  files).

Generator decisions and defects found on the way:

- **Specificity rank (bug).** `shadcn-svelte/packages/cli/src/tailwind.css`
  defines `data-checked`, `data-unchecked`, `data-disabled`,
  `data-active`, `data-horizontal`, `data-vertical` as `:where(…)`
  variants (zero specificity); `tools/stylegen/src/variant.rs` ranks them
  `CLASS`. Effect: in the generated `checkbox` recipe a checked,
  keyboard-focused checkbox gets a primary border, upstream a ring border
  (report 05; report 14 flags the same rule as unverified).
- `data-open` is dropped as animation although it carries the open
  background (accordion, menus, navigation-menu, sidebar): needs a real
  state and the `OPEN` / `EXPANDED` choice (CP-T11-14).
- `data-[side=*]` is ignored as a popup side, so sheet borders, `w-3/4`
  and `max-w` drop silently; a leading `*` drops the alert-dialog media
  icon size (report 11).
- Upstream writes `data-[active=true]` where bits-ui sets `data-active=""`
  (input-otp CP-06-17, navigation-menu in six styles); stylegen maps both
  to `SELECTED`, so egui would show rules the browser never applies.
- Scope calls for the maintainer: animation classes `animate-spin`,
  `animate-pulse` (CP-02-21), `font-sans` (CP-02-15), shimmer, caret
  animation.
- Eight `sidebar-*` palette roles are skipped by the palette import
  (`themes.rs:217`, report 14); sidebar needs them in every palette.
- Diagnostics only: `utility.rs` labels bare `border-b/l/r` and
  `border-collapse` as "border-* value"; `-translate-x-1/2` is ignored
  with the dialog-centring reason; `(--bits-` and `**:` ignores carry
  reasons that do not fit.
- `sera` is read but not exported (618 unsupported pairs).

Unsupported pairs per component (exported styles):

| Component | Pairs | Not measured |
|---|---|---|
| accordion | 5 | cn-accordion-trigger-icon |
| alert | 16 | — |
| alert-dialog | 13 | — |
| aspect-ratio | 0 (nothing to measure: no markers, no class strings in the svelte source) | — |
| attachment | 59 | — |
| avatar | 20 | — |
| breadcrumb | 2 | — |
| bubble | 33 | — |
| button-group | 29 | — |
| calendar | 26 | cn-calendar-dropdown-root, cn-calendar-caption, cn-calendar-caption-label |
| carousel | not measured (extractor stop) | — |
| chart | not measured (extractor stop) | — |
| collapsible | 0 (nothing to measure: no markers, no class strings in the svelte source) | — |
| command | 17 | — |
| context-menu | 27 | cn-context-menu-subcontent |
| drawer | 45 | cn-drawer-popup, cn-drawer-swipe-handle, cn-drawer-header-base, cn-drawer-footer-base |
| dropdown-menu | 27 | cn-dropdown-menu-subcontent |
| empty | 4 | — |
| field | 53 | — |
| form | 5 | — |
| hover-card | 0 | — |
| input-group | 75 | — |
| input-otp | 25 | cn-input-otp-input |
| item | 22 | — |
| kbd | 6 | — |
| marker | 9 | — |
| menubar | 28 | — |
| message | 9 | — |
| message-scroller | 27 | — |
| native-select | 10 | — |
| navigation-menu | 61 | cn-navigation-menu-positioner, cn-navigation-menu-popup |
| pagination | not measured (extractor stop) | — |
| progress | 1 | cn-progress-track, cn-progress-label, cn-progress-value |
| questionnaire | 40 | cn-questionnaire-choice-label, cn-questionnaire-choice-shortcut |
| radio-group | 1 | — |
| range-calendar | 35 | — |
| resizable | 10 | — |
| scroll-area | 5 | — |
| sheet | 2 | — |
| sidebar | not measured (extractor stop) | — |
| skeleton | 1 | — |
| sonner | 0 (nothing to measure: no markers, no class strings in the svelte source) | — |
| spinner | 1 | — |
| table | 11 | — |
| textarea | 2 | — |
| toggle | 2 | — |
| toggle-group | 38 | — |

## Verdicts

Effort and verdict per component, from the reports. "Exists" marks the
14 ported components. Very large: navigation-menu, sidebar,
message-scroller (full primitive), chart (without `egui_plot`), and
calendar by effort (verdict `port`). Not applicable: form. New
dependencies to approve: `egui_plot` (chart), `jiff` (calendar,
range-calendar, only if the crate does not own a date type).

| Component | Cluster | Family | Effort | Verdict | Main drivers |
|---|---|---|---|---|---|
| button | 01 | F3 | small (exists) | port | API drift; group awareness counted under button-group |
| button-group | 01 | F3 | large | port | group-position protocol, per-corner radius, child-target variants |
| toggle | 01 | F3 | small | port | Button-like painting, pressed flag (CP-01-17) |
| toggle-group | 01 | F3 | medium | port | roving focus, position and spacing flags, two modes |
| badge | 02 | F1 | small (exists) | port | link mode, `invalid`, glyph slot, empty-text gap |
| kbd | 02 | F1 | small | port | two recipes, context flags; host embedding rides on hosts |
| spinner | 02 | F1 | small | port | icongen slot, rotated icon paint |
| skeleton | 02 | F1 | small | port | one recipe, pulse fade |
| card | 03 | F2 | small (exists) | port | API reshape (builder header, `InnerResponse`, width) |
| item | 03 | F2 | medium | port | new layout, flags, rounded image clip, click sense |
| empty | 03 | F2 | small | port | centred column, icon tile |
| aspect-ratio | 03 | F1 | small | port | rect allocation and child `Ui`; could live in `layout` (CP-03-29) |
| alert | 04 | F2 | medium | port | five generator gaps, grid with placed action |
| progress | 04 | F1 | small | port | painted track; markers without class strings |
| separator | 04 | F1 | small (exists) | port | API reshape only |
| avatar | 04 | F2 | medium | port | three parts plus group, nine gaps, image loaders |
| checkbox | 05 | F4 | small (exists) | port | indeterminate, minus slot, Enter filter |
| radio-group | 05 | F4 | medium | port | group container, roving focus |
| switch | 05 | F4 | small (exists) | port | drift fixes |
| slider | 05 | F4 | small; medium for several thumbs (exists) | port | per-thumb focus, nearest-thumb pick |
| input | 06 | F5 | small (exists) | port | API cleanup, input-group control mode |
| textarea | 06 | F5 | small | port | Input with `multiline` |
| input-otp | 06 | F5 | medium | port | cell model, paste, filter, first/last gaps |
| input-group | 06 | F5 | large | port | addon placement, recipe composition, most gaps of cluster 06 |
| label | 07 | F1 | small (exists) | port | drift fixes |
| field | 07 | F6 | large | port | eleven markers, flags, `has-*`, container query, choice card |
| form | 07 | F6 | small (docs only) | not applicable in egui | formsnap and sveltekit-superforms bind SvelteKit stores, validation and submission; Field covers the visible parts |
| native-select | 07 | F6 | medium | port | trigger box, shares Select popup code |
| select | 08 | F10 | medium (exists) | port | generic value, groups, disabled items, keyboard parity |
| command | 08 | F10 | large | port | filtered list primitive, fuzzy score, needs input-group and dialog |
| dropdown-menu | 09 | F8 | large | port | shared menu primitive, two icon slots |
| context-menu | 09 | F8 | medium on the primitive | port | pointer anchor, long press |
| menubar | 09 | F8 | medium on the primitive (large alone) | port | roving triggers, hover switch |
| popover | 10 | F7 | small (exists) | port | align, open binding, return type, frame helper |
| hover-card | 10 | F7 | medium | port | hover state machine |
| tooltip | 10 | F7 | medium (exists) | port | sides with arrow, closure content, disabled triggers |
| dialog | 11 | F9 | small (exists) | port | drift list, extract modal shell |
| alert-dialog | 11 | F9 | medium | port | media grid, flags |
| sheet | 11 | F9 | medium | port | edge placement, side flags |
| drawer | 11 | F9 | large | port (no snap points, no background scaling) | drag gesture, non-modal mode |
| tabs | 12 | F11 | medium (exists) | port | vertical, per-trigger disabled, atom content, generic value |
| accordion | 12 | F11 | large | port | closure items with roving focus, wrapped trigger, clip, `chevron_up` |
| collapsible | 12 | F11 | small | port (thin helper) | no visual of its own |
| navigation-menu | 13 | F20 | very large | very large | hover intent, shared viewport, focus proxy, ~15 gaps |
| breadcrumb | 13 | F13 | small | port | two gaps, handle type |
| pagination | 13 | F13 | small to medium | port | page algorithm, roving arrows; not measured by the generator |
| sidebar | 14 | F19 | very large | very large | palette roles, ~25 variants, ~15 parts, deferred dependencies |
| scroll-area | 14 | F12 | medium | port | custom bars on egui state, hover timer |
| resizable | 14 | F12 | medium to large | port | pane layout primitive, keyboard |
| calendar | 15 | F18 | very large | port with a crate-owned `Date` (CP-15-1); otherwise needs dependency: `jiff` | date primitive, grid keyboard, caption dropdowns, many gaps |
| range-calendar | 15 | F18 | large (medium after calendar) | port, same proviso | range logic, range flags |
| table | 16 | F14 | medium | port | column measurement, colspan, row background |
| carousel | 16 | F14 | large | port | drag/fling/snap scroller; not measured by the generator |
| sonner | 16 | F15 | large | port | queue, timers, stacked overlay, swipe |
| chart | 17 | F20 | very large in-crate; medium with `egui_plot` | needs dependency: `egui_plot` | theming wrapper around a plot library; tooltip, legend, config port without it |
| attachment | 18 | F16 | large | port | gaps, flags over budget, spinner and shimmer |
| bubble | 18 | F16 | medium | port | child-slot retarget, relative colour, reactions layout |
| marker | 18 | F16 | small | port | one leaf widget |
| message | 19 | F16 | small; medium with generator work | port (after avatar and bubble) | two flags, named-group variants |
| message-scroller | 19 | F20 | very large (follow-bottom subset medium) | very large; the subset alone is `port` | transcript scroll primitive |
| questionnaire | 19 | F17 | large | port | form state machine, keyboard model, ~15 parts |

## Existing-API drift

The 14 ported components (badge, button, card, checkbox, dialog, input,
label, popover, select, separator, slider, switch, tabs, tooltip) and the
other public API named in the plan (`layout::{row, wrap_row, vstack,
space_between, card, form_row}`, `components::icon`, the public helpers of
`components::shared`). Paths are repository paths; short names
(`button.rs:38`) are in `crates/shadcn-egui/src/components/` unless they
say otherwise.

### Bugs noticed in the existing crate

- **bug** · `crates/shadcn-egui/src/components/badge.rs:109,116` — `content_w`
  adds one gap per icon even when the text is empty: an icon-only badge
  gets a stray gap and an off-centre icon.
- **bug** · `crates/shadcn-egui/src/components/badge.rs:76-78,118` — a
  `Link` badge senses hover only: it underlines on hover but can never be
  clicked or focused.
- **bug** (latent) · `crates/shadcn-egui/src/components/button.rs:172-177`
  — `State::IN_BUTTON_GROUP` is never set, so the generated
  `in-data-[slot=button-group]` rules are dead until a group sets it.
- **bug** · `crates/shadcn-egui/src/components/popover.rs:91` — no
  `.align(..)`, so egui's `RectAlign::BOTTOM_START` applies; upstream
  centres (`align = "center"`), and the `popover-alignments` demo cannot be
  built.
- **bug** · `crates/shadcn-egui/src/components/input.rs:138` — width from
  `ui.available_width().min(max_width)`, which the porting standard
  forbids (rule 6).
- **bug** · `crates/shadcn-egui/src/components/select.rs:210-259` — while
  open: Tab does not close, no PageUp/PageDown, no Alt+ArrowUp;
  `select.rs:232-254` typeahead matches the first letter only (bits-ui
  buffers 1 s and selects on a closed trigger); `select.rs:213,260-270`
  opening with ArrowUp and no value highlights item 0, bits-ui the last.
- **bug** · `crates/shadcn-egui/src/layout.rs:142-143,150` —
  `form_row` fixes the row height, so a taller control is clipped or
  overlaps, and draws the label with egui's `ui.label`, not the crate's
  `Label`.
- **bug** · `tools/stylegen/src/variant.rs` (`TRANSLATED`, the
  `data-checked` … `data-vertical` entries) — ranked `CLASS`, upstream
  `:where()` gives zero specificity; in the generated `checkbox` recipe
  (`crates/shadcn-egui/src/style/vega.rs`, fn `checkbox`, `border_color`)
  `CHECKED → Primary` beats `FOCUS_VISIBLE → Ring`, the reverse of the
  browser. Generated files are fixed through stylegen.
- **bug** · `crates/shadcn-egui/src/components/slider.rs:30,216-217,230,233`
  and `plugin/skills/port-component/SKILL.md:130-131` — comments and the
  skill credit a 1 % default step and PageUp/PageDown ten steps to bits-ui;
  bits-ui 2.19.2 has neither (default step 1, no Page keys).
- **bug** · `crates/shadcn-egui/src/components/slider.rs:231-232` —
  Left/Right also move a vertical slider (bits-ui ignores them);
  Meta+Arrow to the ends is missing; `slider.rs:242-244` End sets `hi`
  even when the step does not divide the range; `slider.rs:139` uses
  `available_width()` arithmetic.
- **bug** (or a choice, CP-05-4) · `crates/shadcn-egui/src/components/checkbox.rs:68-77`
  — `Sense::click()` toggles on Enter (egui fake click); bits-ui toggles on
  Space only.
- **bug** · `crates/shadcn-egui/src/components/tabs.rs:193-199` — with
  `*active` out of range no trigger is focusable, so the list cannot be
  reached with Tab; bits-ui and the porting standard make the first
  trigger the Tab stop.
- **bug** · `crates/shadcn-egui/src/theme.rs:895-927` — the theme sets no
  `ScrollStyle`, so the Select list (`select.rs:442`) and every app
  `egui::ScrollArea` show egui's bars, not shadcn's transparent track and
  `bg-border` thumb.
- **bug** (diagnostics) · `tools/stylegen/src/variant.rs` — `*:` sits in
  the IGNORED list, so classes such as bubble's variant colours drop with
  no diagnostic; `tools/stylegen/src/utility.rs` labels bare
  `border-b/l/r` as "border-* value", and `-translate-x-1/2` is ignored
  with the dialog-centring reason.

### Drift per component

#### button (report 01)

- `button.rs:38`: `pub struct Button` has no `#[must_use]` (egui builder idiom, `egui-idioms.md#Widget builders and impl Widget`); no type in `components/` carries one.
- `button.rs:66-107`: setters are not `#[inline]` (egui builder idiom).
- `button.rs:76`: `.enabled(bool)` on a leaf widget; egui leaf widgets have none and use `ui.add_enabled` (`egui-idioms.md#Enabled state (add_enabled, Ui::disable)`). See CP-01-1.
- `button.rs:52`: `new(text: impl Into<String>)` while egui `Button::new` takes `impl IntoAtoms<'a>` (`egui-idioms.md#Text (WidgetText, RichText)`). See CP-01-2.
- `button.rs:83,88`: `icon(&Icon)`/`icon_end(&Icon)` take a reference to a `Copy` type.
- `button.rs:15,25`: `ButtonVariant`/`ButtonSize` derive no `Default`, while `TabsVariant` does (`tabs.rs:18`); no `#[default]` variant.
- `button.rs:172-177`: the context never sets `State::IN_BUTTON_GROUP`, so the generated `in-data-[slot=button-group]:rounded-*` rules (`style/vega.rs:288,314`, `style/nova.rs:287,320,380,394`) are dead until a group sets it.
- `button.rs:259,265,274`: one `f32` radius for all corners; a grouped button needs per-corner radii (`CornerRadius`) and suppressed left/top border.
- no `button(ui, text)` shortcut (egui `ui.button`); `components/mod.rs` exports only the struct.
- no width option (egui `Button::min_size`); a vertical group stretches its children (`items-stretch`).
- no `href`/link mode (upstream `button.svelte` `{#if href}`).

#### badge (report 02)

- `crates/shadcn-egui/src/components/badge.rs:22` — no `#[must_use]` on `Badge` (`egui-idioms.md#Widget builders and impl Widget`); same for every crate widget (`grep must_use` finds none).
- `badge.rs:38-51` — setters lack `#[inline]`.
- `badge.rs:30` — `new(text: impl Into<String>)`; egui text parameters are `impl Into<WidgetText>` or `impl IntoAtoms` (`egui-idioms.md#Text (WidgetText, RichText)`); RichText is impossible today.
- `badge.rs:43,48` — `icon(&Icon)` copies into `Option<Icon>`; `IconView::new(&'a Icon)` borrows (`icon.rs:47-54`): two conventions for one type.
- `badge.rs:43-51` — slots take only a static `Icon`: the spinner badges of `badge-spinner.svelte` / `spinner-badge-demo.svelte` cannot be built.
- `badge.rs:67-69` — shortcut takes the variant positionally; egui shortcuts take content only (`ui.button(text)`), and the doc line "A badge with the default variant order of the shadcn docs" does not describe what the function does.
- `badge.rs:76-78,118` — `Sense::hover()` only and `[a]` ignored: a `Link` variant badge underlines on hover but can never be clicked or focused, so it looks like a link that does nothing; upstream's link form (`href`) is missing.
- `badge.rs:79-81` — the state never carries `INVALID`, although the base classes style `aria-invalid`; `Button` has `.invalid` (`button.rs:98`).
- `badge.rs:12` — `BadgeVariant` derives no `Default`, while `Badge::new` hard-codes `Default` (`badge.rs:33`).
- `badge.rs:109,116` — `content_w` adds one `gap` per icon even when the text is empty: an icon-only badge gets a stray gap and an off-centre icon (CSS gap is only between present items). Defect.
- `badge.rs:135` — fallback text colour via `t.role(Role::Foreground)`; `IconView` uses `t.palette.foreground` (`icon.rs:85`) for the same thing.
- `badge.rs` — no `fill`/`text_color` override (badge-colors demo); egui `Button::fill` is the idiom.
- `badge.rs` — `uppercase` and `tracking` recipe fields are never read (matters only for sera, see Generator gaps).

#### components::icon (report 02)

- `crates/shadcn-egui/src/components/icon.rs:47` — `IconView` has no `#[must_use]` (`egui-idioms.md#Widget builders and impl Widget`); setters (`icon.rs:61,65`) lack `#[inline]`.
- `icon.rs:47-54` — `IconView::new(&'a Icon)` borrows while `Badge::icon`/`Button::icon` take `&Icon` and copy (`badge.rs:43`, `button.rs:83`); `Icon` is `Copy`, so a by-value parameter would serve both (CP-02-8).
- `icon.rs:85` — default colour `t.palette.foreground`, not the enclosing `Ui`'s text colour: an `IconView` inside a muted or primary context stays foreground, unlike CSS `currentColor` (CP-02-20); `badge.rs:135` reaches the same colour through `t.role(Role::Foreground)`.
- `icon.rs:65` — `color(Color32)`; `egui::Spinner::color` and `Image::tint` take `impl Into<Color32>`.
- `icon.rs:86` — stroke width fixed to `t.metrics.icon_stroke`; no setter (CSS `stroke-width` is settable per icon upstream).
- `icon.rs:25-42` — `paint_icon` cannot rotate; the spinner needs a rotated variant (or a `rotation` parameter).
- `icon.rs:44-46,76-83` — no accessible name of its own; `labelled_by` is the only route. A spinner with its own "Loading" label needs `WidgetInfo.label`.
- `icon.rs` — `Icon` has no `Into<Atom>`/`Image` conversion, so `ui.button((icon, "text"))` and egui `IntoAtoms` hosts (`Button::shortcut_text`, `TextEdit::prefix`) cannot take crate icons; the rasterised texture from `icon_raster::texture` could back an `egui::Image` atom. Feeds CP-02-1/CP-02-12.
- `icon.rs:61` — `size(f32)` square only; fine for icons (egui `Spinner::size` is the same form).

#### card (report 03)

- `card.rs:65-66` `Card` has no `#[must_use = "You should call .show()"]` (`egui-idioms.md#Containers`).
- `card.rs:99` `show` returns `R`; the card's `Response` is built (`card.rs:251`) and dropped. egui containers return `InnerResponse<R>`.
- `card.rs:261` `card_content` returns `R`, not `InnerResponse<R>`; in the sketch it disappears (the `show` closure is the content).
- `card.rs:275` `card_header(ui, &str, &str)` requires a description; sketch makes it optional and moves it into the builder.
- `card.rs:289` `card_header_with_action` is a second function for the same part; sketch has one `.action(..)` setter.
- `card.rs:92` footer is a builder closure while header and content are free functions inside the body (`card.rs:261`, `275`): two sub-part idioms in one component.
- `card.rs:178-190` the footer is always a left-to-right row one `interact_size.y` high; upstream demos with `flex-col` footers cannot be expressed directly.
- `card.rs:275`, `289`, `326`, `346` text parameters are `&str`; egui labels take `impl Into<WidgetText>` (`egui-idioms.md#Text`), so `RichText` cannot be passed.
- `card.rs:49-57`, `155-164` the size reaches `card_title`/`card_description` through one global `ctx.data` temp key, not tied to the card's Ui; a part shown in another layer during the closure (a popover opened from the card) sees the card's size.
- `card.rs:128` width is always `ui.available_width()`; no `max_width`/`width` setter although every demo constrains the card (`max-w-sm`, `max-w-xs`).
- `card.rs:137` the body child Ui is bounded by `ui.available_height()`.
- `card.rs:150-154` the card overrides the `Body` and `Button` text style sizes for everything in its body, not only text.
- `layout.rs:135` `layout::card` duplicates `Card::new().show` (see the layout section).

#### layout (report 03)

- `layout.rs:25` `row(ui, gap, add) -> R`: returns `R`, dropping the row's `Response`; egui's `ui.horizontal` returns `InnerResponse<R>` (`egui-idioms.md#Containers`).
- `layout.rs:25-100` `row` keeps its height in `ctx.data` keyed by `ui.next_auto_id()` and calls `request_discard` when it changes: a multi-pass layout helper with no egui counterpart; the id depends on call order (no `id_salt`).
- `layout.rs:105` `wrap_row` returns `R` (drops `horizontal_wrapped`'s `InnerResponse`); it sets `item_spacing` both axes to one `gap`, no separate row gap.
- `layout.rs:114` `vstack` returns `R`; it is `ui.vertical` plus `item_spacing.y`. The pair `row`/`vstack` mixes naming families (CSS "row" vs SwiftUI "vstack"); egui names them `horizontal`/`vertical` (`egui-idioms.md#Direction and layout`).
- `layout.rs:124` `space_between(ui, left, right)` duplicates `egui::containers::Sides` (`egui-idioms.md#Containers`, Sides): closures return `()`, the call returns `()`, no gap or height control; `Sides::show` returns `(RetL, RetR)`.
- `layout.rs:135` `layout::card(ui, add) -> R` is a one-line duplicate of `Card::new().show(ui, add)` (`card.rs:99`): two public paths to the same default-size card, and it inherits `show`'s `R` return. It also sits in a module of "flexbox substitutes" though it is a component.
- `layout.rs:140` `form_row(ui, label_width, label: &str, control)`: label is `&str` (not `impl Into<WidgetText>`), drawn with `ui.label` rather than the crate's `Label` component, not linked to the control with `labelled_by` (`egui-idioms.md#widget_info and accessibility`), returns `()`, and `control` cannot return a value.
- `layout.rs:7`, `144-158` `form_row` is the only user of `egui_extras` (`StripBuilder`) in the crate, so a runtime dependency exists for one helper (`crates/shadcn-egui/Cargo.toml` `[dependencies]`).
- `layout.rs:143` `form_row` sizes by `ui.available_width()`, no `max_width`.
- `layout.rs:25`, `105`, `114`, `124`, `140` all take a raw `gap: f32`/`label_width: f32` in px; no helper takes a spacing token, so callers write literals.

#### separator (report 04)

- `crates/shadcn-egui/src/components/separator.rs:30` `pub fn separator(ui: &mut Ui)` returns `()`; egui's `Ui::separator` returns `Response` (`egui-0.34.3/src/ui.rs:2169`) and the sketch does too.
- `separator.rs:40` `separator_vertical(ui)` likewise returns `()`.
- `separator.rs:11-27` no `Separator` struct / `impl Widget`, so it cannot go through `ui.add`, `ui.add_sized`, `ui.add_enabled` (`egui-idioms.md#Widget builders and impl Widget`).
- `separator.rs:21` `allocate_exact_size(.., Sense::hover())` drops the `Response` instead of returning it.
- `separator.rs:35,45` sizes from `ui.available_width()` / `ui.available_height()`; the porting standard says "no `available_width()` arithmetic" (`plugin/skills/port-component/SKILL.md:86`). egui uses `available_size_before_wrap()` and returns `Vec2::ZERO` in a sizing pass (`egui-0.34.3/src/widgets/separator.rs`); the crate's version has no sizing-pass guard, so a separator inside a sizing pass (popover, menu with auto width) claims the full width.
- `separator.rs:29-47` orientation is chosen by function name; egui's `Separator` also infers it from the layout (`is_horizontal_line: None`).
- `separator.rs` no `decorative` / accessibility choice; no `widget_info` (matches egui's own `Separator`, which reports none either).
- The CSS markers `cn-separator`, `cn-separator-horizontal`, `cn-separator-vertical` exist in every style but the crate (correctly) uses `slot:separator`; `cn-separator-vertical` sets `h-full`, which the crate's vertical size (row height) matches by behaviour, not by recipe.

#### components::shared (report 04)

The module is `pub mod shared` (`crates/shadcn-egui/src/components/mod.rs:13`), so every item below is public API of the crate and under semver. Audience as found: port authors (in the crate, and in apps per `plugin/skills/port-component/references/in-app.md`); none of it is useful to an app author who only places widgets.
| Item (`crates/shadcn-egui/src/components/shared.rs`) | Signature | Audience | Documented in the skills |
|---|---|---|---|
| `DEFAULT_SIDE_OFFSET` (`:12`) | `pub const DEFAULT_SIDE_OFFSET: f32 = 4.0` | ports (popover, select) | `references/upstream.md` |
| `opacity` (`:16`) | `pub fn opacity(chain: &[&[&Recipe]], s: State) -> f32` | ports | `SKILL.md:154` |
| `ui_opacity` (`:30`) | `pub fn ui_opacity(ui: &Ui, chain: &[&[&Recipe]], s: State) -> f32` | ports | `SKILL.md:158` |
| `style_painter` (`:49`) | `pub fn style_painter(ui: &Ui, chain: &[&[&Recipe]], s: State) -> Painter` | ports | `SKILL.md:156` |
| `faded` (`:61`) | `pub fn faded<R>(ui: &mut Ui, opacity: f32, add: impl FnOnce(&mut Ui) -> R) -> R` | ports | `SKILL.md:165` |
| `line_box` (`:76`) | `pub fn line_box(theme: &Theme, layers: &[&Recipe], s: State, size: TextSize) -> f32` | ports | `SKILL.md:169` |
| `corner` (`:82`) | `pub fn corner(r: f32) -> CornerRadius` | ports (card, dialog, tooltip, theme, icon_raster) | `build-ui/references/cpu-rendering.md` only |
| `ring` (`:88`) | `pub fn ring(painter: &egui::Painter, rect: Rect, radius: f32, width: f32, color: Color32)` | ports | no |
| `ring_shape` (`:93`) | `pub fn ring_shape(rect: Rect, radius: f32, width: f32, color: Color32) -> Option<Shape>` | ports | no |
| `box_shapes` (`:108`) | `pub fn box_shapes(rect: Rect, radius: f32, fill: Option<Color32>, border: Option<Stroke>, clip_padding: bool) -> Vec<Shape>` | ports | no |
| `paint_box` (`:141`) | `pub fn paint_box(painter: &egui::Painter, rect: Rect, radius: f32, fill: Option<Color32>, border: Option<Stroke>, clip_padding: bool)` | ports | `references/styles.md`, `references/in-app.md` |
| `shift` (`:154`) | `pub fn shift(rect: Rect, x: Option<Shift>, y: Option<Shift>) -> Rect` | ports (button, switch) | no |
| `interaction_state` (`:171`) | `pub fn interaction_state(theme: &Theme, resp: &Response, enabled: bool, context: State) -> State` | ports (all interactive widgets) | `SKILL.md:146` |
Differences from the sketch conventions and `egui-idioms.md`:
- `shared.rs:61` `faded` returns `R`; egui scope-like helpers return `InnerResponse<R>` (`egui-idioms.md#Containers (show, InnerResponse, Frame, UiBuilder, Sides)`). It adds no layout, so `R` is defensible, but it differs from `ui.scope`.
- `shared.rs:88,93,108,141` take `radius: f32`; egui painting APIs take `impl Into<CornerRadius>` (`Image::corner_radius`, `Frame::corner_radius`, `Shape::rect_*`).
- `shared.rs:108,141` positional `clip_padding: bool` parameter; egui expresses such options as builder setters or enums (`egui-idioms.md#Widget builders and impl Widget`), not trailing bools.
- `shared.rs:88` vs `shared.rs:141`: argument order differs between siblings (`ring(painter, rect, radius, width, color)` vs `paint_box(painter, rect, radius, fill, border, clip)` vs `ring_shape(rect, …)` with no painter); `style_painter` takes `&Ui` while `ring`/`paint_box` take `&egui::Painter`.
- `shared.rs:82` `corner(r)` duplicates epaint's `impl From<f32> for CornerRadius` (`epaint-0.34.3/src/corner_radius.rs:41`, also `same(radius.round() as u8)`); `r.into()` or `CornerRadius::from(r)` does the same. Its generic name in a public module says nothing about what it builds.
- `shared.rs:171` `interaction_state` takes `enabled: bool` beside `resp` although `resp.enabled()` is read inside; callers pass the widget's own `.enabled(bool)` builder flag, a crate-only idiom (egui leaf widgets have no `.enabled`, `egui-idioms.md#Enabled state (add_enabled, Ui::disable)`).
- `shared.rs:12` `DEFAULT_SIDE_OFFSET` is a bits-ui prop default exported as crate API; popover/select-specific, not shared painting.
- Module placement: port helpers sit in `components::shared` beside app-facing widgets; nothing marks them as port API (no `#[doc(hidden)]`, no separate `port`/`paint` module), so seven undocumented helpers (`corner`, `ring`, `ring_shape`, `box_shapes`, `shift`, `DEFAULT_SIDE_OFFSET`, partly `paint_box`) are public by accident or by intent; the skills do not say which.

#### checkbox (report 05)

- `crates/shadcn-egui/src/components/checkbox.rs:7-8`: no indeterminate state; adding `.indeterminate(bool)` is additive, no break (fields are private).
- `checkbox.rs:22`: no `#[must_use]` on the builder (`egui-idioms.md#Widget builders and impl Widget`); same for every crate builder (no `must_use` anywhere under `crates/shadcn-egui/src/components/`).
- `checkbox.rs:37,42`: setters lack `#[inline]` (egui convention).
- `checkbox.rs:37`: `.enabled(bool)` on a leaf widget, which egui does not offer (`egui-idioms.md#Enabled state`); see CP-05-3.
- `checkbox.rs:68-77`: `Sense::click()` also toggles on Enter (egui fake click), bits-ui toggles on Space only (`checkbox.svelte.ts:198-209`).
- `checkbox.rs:145`: shortcut returns `Response`; `select.rs:567` and `tabs.rs:381` return `bool` (cross-crate inconsistency, CP-05-5).
- `checkbox.rs:132-138`: no label, named through `labelled_by` — differs from egui's `Checkbox::new(&mut b, atoms)` but matches upstream; keep consistent with radio items (CP-05-2).
- Generated recipe `style/vega.rs` fn `checkbox`, `border_color`: CHECKED outranks FOCUS_VISIBLE, opposite of the browser (cluster-wide finding).

#### switch (report 05)

- `crates/shadcn-egui/src/components/switch.rs:27`: no `#[must_use]`; setters `:43,48,53` lack `#[inline]`.
- `switch.rs:48`: `.enabled(bool)` on a leaf widget (`egui-idioms.md#Enabled state`); CP-05-3.
- `switch.rs:170-175`: reports `WidgetType::Checkbox`; AccessKit offers `Role::Switch` (CP-05-23). The doc comment at `:23-26` says so.
- `switch.rs:182`: `toggle` returns `Response`; `select`/`tab_bar` return `bool` (CP-05-5).
- Generated recipes: `data-checked`/`data-unchecked` ranks (cluster-wide finding); no visible conflict found for the switch track (no rule competes with `data-checked:bg-primary`), but `focus-visible:border-ring` vs luma/rhea `data-checked:border-primary` has the same inversion as the checkbox.
- No drift in keyboard behaviour: Enter and Space toggle in both.

#### slider (report 05)

- `crates/shadcn-egui/src/components/slider.rs:24-25,33-34`: one thumb only; `Slider::multiple(&mut [f32], ..)` can be added later without a break (the `value` field is private; `new` keeps its signature).
- `slider.rs:33`: no `#[must_use]`; setters `:54,58,63,68` lack `#[inline]`.
- `slider.rs:16-22`: own `SliderOrientation` shadows `egui::SliderOrientation` (same name, different type); `tabs.rs:31` has a third orientation enum (CP-05-37). egui also offers `.vertical()`.
- `slider.rs:47,53-57`: default `step = 0` (continuous) vs bits-ui `step = 1`; egui calls it `step_by(f64)`.
- `slider.rs:216-217`: comment "Without a step, bits-ui moves by 1% of the range" and `behaviour-ok: bits-ui's 1% default step` have no basis in bits-ui 2.19.2 (it always has a step, default 1; `helpers.ts` `normalizeSteps`).
- `slider.rs:30,230,233`: PageUp/PageDown "bits-ui's … ten steps": bits-ui 2.19.2 handles no Page keys (`slider.svelte.ts:743-750`). The same claim is in `plugin/skills/port-component/SKILL.md:130-131`.
- `slider.rs:231-232`: Left/Right also move a vertical slider; bits-ui ignores them when vertical (`slider.svelte.ts:883,894`). Meta+Arrow → ends is missing.
- `slider.rs:242-244`: End sets `hi` even when `step` does not divide the range; bits-ui snaps End to the last grid value.
- `slider.rs:196-197`: pointer maps over the thumb travel; bits-ui maps over the whole root (CP-05-31).
- `slider.rs:139`: `ui.available_width()` arithmetic for the fill width; the porting standard says use the `layout` helpers, no `available_width()` arithmetic (`plugin/skills/port-component/SKILL.md`, "egui sizing"); no `.width` setter (`Select`, `Tabs` have one).
- `slider.rs:68-71`: `.label(String)` names the slider, while Checkbox and Switch are named with `labelled_by` (CP-05-35); egui's own name for a slider caption is `.text(..)`, which also draws it.
- `slider.rs:63`: `.enabled(bool)` on a leaf widget (CP-05-3).
- No shortcut function, unlike `checkbox` and `toggle` (CP-05-38).
- `slider.rs:34`: `&mut f32` only; egui's `Slider` takes any `emath::Numeric` (CP-05-26).

#### input (report 06)

- `input.rs:14`: no `#[must_use]` on `Input` (`egui-idioms.md#Widget builders and impl Widget`).
- `input.rs:36-39`: `hint(impl Into<String>)`; TextEdit's `hint_text(impl IntoAtoms<'static>)` takes an icon atom too (`egui-idioms.md#TextEdit`).
- `input.rs:18,30,44-49,121-125`: `max_width` uses an `f32::NAN` sentinel instead of `Option<f32>`.
- `input.rs:59-63`: deprecated `id_source` still public.
- `input.rs:64-68`: `.enabled(bool)` on a leaf widget; egui disables leaves through `ui.add_enabled` (`egui-idioms.md#Enabled state`). Same pattern in `button.rs:76`, `checkbox.rs:37`, `switch.rs:48`, `select.rs:87`, `slider.rs:63`, `label.rs:30`.
- `input.rs:76-176`: only `impl Widget`; no `show(ui) -> TextEditOutput`, so cursor range, galley and state are out of reach (`egui-idioms.md#Widget builders and impl Widget`, two-step variant).
- `input.rs` has no shortcut function, unlike `checkbox.rs:145` (`checkbox`), `switch.rs:182` (`toggle`), `label.rs:36` (`label`), `select.rs:567` (`select`).
- `input.rs:138`: width from `ui.available_width().min(max_width)`, which `SKILL.md` rule 6 forbids ("no `available_width()` arithmetic"; use `shadcn_egui::layout`).
- `input.rs:113-120`: `char_limit`, `return_key`, `horizontal_align` of `TextEdit` not reachable.
- Width setter name differs between components: `Input::max_width` (`input.rs:46`) vs `Select::width` (`select.rs:97`).
- No control-in-group mode: InputGroup needs the stack `[cn-input, cn-input-group-input]` and the group, not the input, painting box and ring (`input.rs:79` hard-codes `[&*t.style.input]`).

#### label (report 07)

- `crates/shadcn-egui/src/components/label.rs:17`: no `#[must_use]` on `Label` (egui-idioms.md#Widget builders and impl Widget). No component in `crates/shadcn-egui/src/components/` carries one.
- `label.rs:23`: `new(text: impl Into<String>)`; egui labels take `impl Into<WidgetText>` (egui-idioms.md#Text). The `class="font-normal"` overrides in `examples/field-checkbox.svelte` and `field-radio.svelte` cannot be expressed.
- `label.rs:29-33`: `.enabled(bool)` on a leaf widget; egui disables leaf widgets through the `Ui` (egui-idioms.md#Enabled state). The meaning also differs: this is "the peer is disabled", not "this label is disabled".
- `label.rs:36`: `label(ui, text: &str)` takes `&str`, while the builder takes `Into<String>`. The two entry points disagree.
- `label.rs:55`: `layout_no_wrap`: a long label never wraps; upstream wraps.
- `label.rs:60`: `Sense::hover()`: a click on the label does not reach the control (the `<label for>` behaviour in `examples/label-demo.svelte`).
- `label.rs:72-89`: `description()` reads `cn-card-description` because Label had no description marker. With Field ported, `cn-field-description` is that marker (muted, `text-sm`, `leading-normal`). The helper should move to `field` and read it. It also returns egui's `ui.label` Response, which has its own widget_info.
- `label.rs:51`: falls back to `Role::Foreground`. Inside an invalid Field the inherited colour is `destructive` (`cn-field data-[invalid=true]:text-destructive`). Label cannot inherit it today.

#### field (report 07)

`layout::form_row` (`crates/shadcn-egui/src/layout.rs:139-160`) against upstream Field:
- `layout.rs:150`: draws the label with egui's `ui.label(label)`, not the crate's `Label`, so the label ignores `cn-label` / `cn-field-label` (size, weight, leading, disabled fade, invalid colour).
- `layout.rs:140,145`: a fixed `label_width` column. Upstream horizontal Field has no column: the label is `flex-auto`, and vertical (label above control) is the default. `form_row` offers only a horizontal grid row.
- `layout.rs:140`: returns `()`, and the control closure is `FnOnce(&mut Ui)` without `R`. Containers return `InnerResponse<R>` (egui-idioms.md#Containers).
- `layout.rs:140`: `label: &str`, not `impl Into<WidgetText>` (egui-idioms.md#Text).
- `layout.rs:142-143`: the row height is fixed at `max(interact_size.y, line_px(Sm))` inside `allocate_ui`. A taller control (textarea, choice card, a control with a description) is clipped or overlaps. Field grows with its content.
- `layout.rs:140-160`: no gap from a recipe (upstream `cn-field gap-3` vega/maia/luma/rhea/sera, `gap-2` nova/lyra/mira), no description, no error, no invalid or disabled state, no orientation.
- `layout.rs:140-160`: no `labelled_by` between label and control; upstream ties them with `<label for>`.
- `layout.rs:140`: a free function in `layout`, while the build-ui map (`plugin/skills/build-ui/references/layout-map.md:16`) presents it as the form row. Once Field ships, the map should point to `Field::new().horizontal()` and `form_row` becomes redundant. Whether to keep, deprecate or rebuild it on Field is a decision for the maintainer.
- `crates/shadcn-egui/src/components/label.rs:72-89`: `label::description` duplicates FieldDescription and reads the wrong recipe (`cn-card-description`) (see label drift).

#### select (report 08)

- `select.rs:50` no `#[must_use]` on `Select`; every egui builder carries it (`egui-idioms.md#Widget builders and impl Widget`).
- `select.rs:51,64,181` `id: &'a str` hashed into the id; egui and `Dialog::new` (`dialog.rs:33`) take `id_salt: impl Hash` (`egui-idioms.md#IDs (id_salt, Id::new, push_id)`).
- `select.rs:52,64` `&mut usize`: the index form has one egui precedent (`ComboBox::show_index`), but egui's main idiom writes the value itself through `&mut T` (`selectable_value`); the caller keeps a parallel index-to-value mapping.
- `select.rs:63,115` "no value" is an out-of-range index sentinel; with `&mut T` it would be `Option` or a value not among the items.
- `select.rs:53,64` `&[&str]` cannot carry groups, labels, separators, per-item disabled or icons. Additive escape hatches (`.disabled_items(&[usize])`, `.group_starts(&[(usize, &str)])`, `Select::multiple(&mut Vec<usize>, …)`) would avoid a break but read worse than a value-typed rewrite; the value type change itself is breaking.
- `select.rs:55,82` placeholder `impl Into<String>`; egui text idiom is `impl Into<WidgetText>` (`egui-idioms.md#Text (WidgetText, RichText)`).
- `select.rs:86-90` `.enabled(bool)` on a leaf widget; egui disables leaves through `ui.add_enabled` (`egui-idioms.md#Enabled state (add_enabled, Ui::disable)`). It also honours `ui.is_enabled()` (`:113`), so both work.
- `select.rs:96-99` `width` only, no `max_width`; `Input` has `max_width` (`input.rs:46`), upstream demos use `w-full max-w-48`.
- `select.rs:210-259` while open: no Tab-close (bits `select.svelte.ts:850`), no PageUp/PageDown ±10 (`:887-890`), no Alt+ArrowUp close (`:868`).
- `select.rs:232-254` typeahead matches the first letter only; bits accumulates for 1 s (`data-typeahead.svelte.ts:22-41`), and on a closed trigger selects without opening (`select.svelte.ts:829`).
- `select.rs:213,260-270` opening with ArrowUp and no value highlights item 0; bits highlights the last (`select.svelte.ts:843-845`).
- `select.rs:209` opens on click (release); bits opens on pointerdown and selects on pointerup (`select.svelte.ts:932-964`, `:1197-1232`). Matches egui `ComboBox`, so arguably correct for egui.
- `select.rs:566-569` shortcut returns `bool`; egui shortcuts (`ui.checkbox`, `ComboBox::show_index`) return `Response`. The crate's `tab_bar` (`tabs.rs:381`) also returns `bool`, so the choice is crate-wide.
- `select.rs:19-22` (module docs) and `component-map.md:21` state the missing groups, labels, separators and disabled items; their recipes resolve already (`markers.txt` lists `cn-select-label`, `cn-select-separator`), so adding them needs no generator work.

#### command (report 08)

n/a (not in the crate). Related crate APIs: `Dialog::show` takes `&egui::Context` and `&mut bool` (`dialog.rs:69-74`), which CommandDialog would follow; `Dialog` has no placement option (it centres, `dialog-content.svelte` `top-1/2`), needed for `top-1/3`.

#### dropdown-menu (report 09)

n/a (not in the crate). Related crate text that the port must update:
`theme.rs:117` doc of `MenuColors` ("today the Select list"),
`component-map.md:28` ("Not yet ported: … Menubar, … DropdownMenu").

#### popover (report 10)

- `crates/shadcn-egui/src/components/popover.rs:20`: no `#[must_use = "You should call .show()"]` (egui-idioms.md#Containers).
- `popover.rs:47,116`: `show` returns `Option<R>`; egui `Popup::show` returns `Option<InnerResponse<R>>`, sketch follows egui.
- `popover.rs:91`: no `.align(..)`, so egui's default `RectAlign::BOTTOM_START` (`popup.rs:202`) applies; shadcn sets `align = "center"` (`ui/popover/popover-content.svelte`), i.e. `RectAlign::BOTTOM`. The popover is placed start-aligned where upstream centres it, and the `popover-alignments` demo cannot be built.
- `popover.rs:91`: fallback alignments are egui's default list (`symmetries` then all 12 `MENU_ALIGNS`), not bits' flip to the opposite side.
- `popover.rs:91`: open state is egui memory only; no `.open_bool` for bits `bind:open`; the doc (`popover.rs:10-12`) notes that opening closes any other popup, a consequence of the memory mode.
- `popover.rs:26-45`: no way to read the open state before the trigger is drawn, so the trigger cannot set `Button::expanded` (`button.rs:92-96`), and no `.id`/`.id_salt`.
- `popover.rs:123`: `popover_header(ui, &str, &str)` returns `()`; egui text idiom is `impl Into<WidgetText>` and a `Response` (egui-idioms.md#Text); `card.rs:326,346` offers separate `card_title`/`card_description` while popover offers no `popover_title`/`popover_description`.
- `popover.rs:6-8`: focus stays on the trigger while open and is not returned on close; bits traps and returns focus.
- `popover.rs:91-110`: no `widget_info` for the content (egui-idioms.md#widget_info and accessibility).

#### tooltip (report 10)

- `crates/shadcn-egui/src/components/tooltip.rs:19`: free function `tooltip(resp, text: &str)`; egui's tooltip text parameter is `impl Into<WidgetText>` (`on_hover_text`, egui-idioms.md#Text) and there is no builder or closure form (egui `Response::on_hover_ui`, egui `Tooltip::show`).
- `tooltip.rs:46`: `Tooltip::for_enabled` gives disabled triggers no tooltip; the upstream `tooltip-disabled` demo shows one.
- `tooltip.rs:62`: side fixed to `RectAlign::TOP`, no setter; the `tooltip-sides` demo uses four sides. The arrow code (`tooltip.rs:75-77`) handles only above/below.
- `tooltip.rs:49-50,62`: gap is the arrow's reach (≈7.07 px for `size-2.5`); bits offsets by `sideOffset + arrowHeight` (`use-floating-layer.svelte.ts:157`), with the arrow box 10 px tall (unsure; check against the docs page).
- `tooltip.rs:63-71`: content is one label; no way to add a Kbd or icon (`tooltip-keyboard` demo).
- `tooltip.rs:46`: delay is egui's global `tooltip_delay` 0.5 s (`style.rs:1472`); shadcn's Provider sets `delayDuration = 0` (`ui/tooltip/tooltip-provider.svelte`). Not mentioned in the module doc.
- `tooltip.rs:46`: no opening on keyboard focus (bits `#onfocus`), since egui's `should_show_tooltip` requires hover.
- `tooltip.rs:1-7`: no `widget_info`/`labelled_by` link between trigger and tooltip text (bits `aria-describedby`); egui offers no describedby, so this is at most a doc note.

#### dialog (report 11)

- `crates/shadcn-egui/src/components/dialog.rs:23`: `Dialog` has no `#[must_use = "You should call .show()"]` (egui container idiom).
- `dialog.rs:33`: `new(id_salt, title: &'a str)`; titles in egui take `impl Into<WidgetText>` (`egui-idioms.md#Text (WidgetText, RichText)`).
- `dialog.rs:43`: `description(&'a str)`, same.
- `dialog.rs:69-74`: `show` returns `Option<R>`; no panel `Response` (Window returns `Option<InnerResponse<..>>`), so the caller cannot read the panel rect or attach hover text.
- `dialog.rs:154`: `backdrop_response.clicked() || should_close()`: `should_close()` already includes the backdrop click (`containers/modal.rs:158-160`); redundant, not wrong.
- `dialog.rs:126-145`: no initial focus on open and no focus return to the trigger on close, unlike bits-ui `FocusScope` (`focus-scope.svelte.ts:77-115`).
- `dialog.rs` (whole file): no `widget_info` / AccessKit role for the panel, title or description (bits-ui sets `role="dialog"`, `aria-labelledby`, `aria-describedby`).
- `dialog.rs:247`: `footer` returns `()`; egui layout helpers return `InnerResponse<R>`.
- `dialog.rs:247`: `footer` has no counterpart for `dialog-footer.svelte`'s `showCloseButton` prop.
- `dialog.rs:164`: `header` is private; Sheet, Drawer and AlertDialog need the same header logic with their own recipes. Extract it into the shared shell.

#### tabs (report 12)

- `tabs.rs:52`: `new(&mut usize, &[&str])`; sketch uses generic `&mut V` and `.tab(value, atoms)`.
- `tabs.rs:44,55,134`: labels are owned `String`s, laid out as plain text; no icon/atom content (upstream `tabs-icons.svelte`).
- `tabs.rs:27-34,103`: `TabsOrientation` has only `Horizontal`; upstream `tabs-vertical.svelte` and the `group-data-vertical/tabs` recipes are unused.
- `tabs.rs:70-74`: `.enabled(bool)` on a leaf widget; `egui-idioms.md#Enabled state (add_enabled, Ui::disable)` says leaf widgets are disabled through the `Ui`. No per-trigger disabled (upstream `tabs-disabled.svelte`).
- `tabs.rs:88-91`: `content_gap(ui)` is an associated getter the caller must thread into `layout::vstack` (`reference.rs:77,764`); no `show` container.
- `tabs.rs:171-172`: id comes from the auto id of `allocate_exact_size`; no `id_salt` (`egui-idioms.md#IDs (id_salt, Id::new, push_id)`).
- `tabs.rs:381-383`: `tab_bar(..) -> bool`; egui `Ui` shortcuts return `Response`.
- `tabs.rs:258-265`: role `SelectableLabel`; no tablist/tabpanel information.
- `style/generated.rs:157`: `tabs_content` recipe (`text-sm`, `text-xs/relaxed`) is generated for every style but no code reads it.
- **bug** `tabs.rs:193-199`: with `*active` out of range (`active >= labels.len()`), no trigger gets a focusable sense, so the list is not reachable with Tab. bits-ui makes every trigger a Tab stop when nothing is selected, and `port-component/SKILL.md` ("the selected item, else the first") requires the first trigger.

#### pagination (report 13)

n/a (not in the crate). Related cross-cluster note: the crate shortcuts `tab_bar` (`components/tabs.rs:381`) and `select` (`components/select.rs:567`) return `bool`, while egui's `Ui` shortcuts return `Response` (`egui-idioms.md#Widget builders and impl Widget`). CP-13-23 inherits that split.

#### sidebar (report 14)

n/a (not in the crate). Dependencies it touches:
`tooltip` has no side (`crates/shadcn-egui/src/components/tooltip.rs:19`); `separator` has
no way to add margin or stack a recipe (`crates/shadcn-egui/src/components/separator.rs:30`).

#### scroll-area (report 14)

n/a (not in the crate). The theme leaves `style.spacing.scroll` at egui's
default (`crates/shadcn-egui/src/theme.rs:895-927` sets no `ScrollStyle`), so the Select list
(`crates/shadcn-egui/src/components/select.rs:442`) and any app `egui::ScrollArea` show egui's
bar: track colour `extreme_bg_color` = `input`, handle from `widgets.inactive`
= `secondary`, not shadcn's transparent track and `bg-border` thumb.

#### chart (report 17)

n/a (not in the crate). Related crate API: `Palette::chart: [Color32; 5]`
(`theme.rs:30-31`) already holds the five chart colours and the preset
already carries a chart colour (`reference.rs:513`), so the colour side
needs no new theme field.

## Topics

One table per topic, from the cross-checks: the answer each report's sketch gives, and the CPs tagged with the topic.

### T1 · Selection

| # | Component | Report | Answer (topic 1) | CPs tagged 1 |
|---|---|---|---|---|
| 1 | button | 01 | n/a | — |
| 2 | button-group | 01 | n/a | — |
| 3 | toggle | 01 | `&mut bool`; `Response`, `changed()` | — |
| 4 | toggle-group | 01 | generic `T`; single `&mut Option<T>` (deselect), multiple `&mut Vec<T>`; `ToggleItem<T>` iterator; group `Response` | CP-01-20, CP-01-21, CP-01-22, CP-01-24 |
| 5 | badge | 02 | n/a | — |
| 6 | kbd | 02 | n/a | — |
| 7 | spinner | 02 | n/a | — |
| 8 | skeleton | 02 | n/a | — |
| 9 | card | 03 | n/a | — |
| 10 | item | 03 | n/a | — |
| 11 | empty | 03 | n/a | — |
| 12 | aspect-ratio | 03 | n/a | — |
| 13 | alert | 04 | n/a | — |
| 14 | progress | 04 | n/a (display only, `f32` value) | CP-04-9, CP-04-10 |
| 15 | separator | 04 | n/a | — |
| 16 | avatar | 04 | n/a | — |
| 17 | checkbox | 05 | `&mut bool` + display `indeterminate(bool)`; `Response`, `changed()` | CP-05-4, CP-05-7 |
| 18 | radio-group | 05 | `&mut V` per item, single; items one call each in `show`; caller `Option<T>` for empty; `InnerResponse<R>` | CP-05-9, CP-05-10, CP-05-16, CP-05-17 |
| 19 | switch | 05 | `&mut bool`; `Response`, `changed()` | — |
| 20 | slider | 05 | `&mut f32` single, `&mut [f32]` multiple later; `Response` | CP-05-25 … CP-05-34 |
| 21 | input | 06 | n/a (`&mut String`, `Response`) | CP-06-2, CP-06-6 |
| 22 | textarea | 06 | n/a (`&mut String`) | CP-06-12 |
| 23 | input-otp | 06 | `&mut String` of at most `len` chars; `Response`; `show` adds `completed` | CP-06-13, CP-06-15, CP-06-21 |
| 24 | input-group | 06 | n/a (the control's value and `Response`) | — |
| 25 | label | 07 | n/a | CP-07-4 |
| 26 | field | 07 | n/a (FieldCard reports a click; caller owns the radio value) | — |
| 27 | form | 07 | n/a | — |
| 28 | native-select | 07 | `&mut T`, single; `option` calls in `show_ui` or `&[(T, &str)]` shortcut; `InnerResponse<Option<R>>` / `Response` | CP-07-24 |
| 29 | select | 08 | generic `&mut T`, single; data slice `SelectItem`/`SelectGroup`; `Response`; shortcut keeps `&mut usize` + `&[&str]` | CP-08-1, CP-08-2, CP-08-4, CP-08-5, CP-08-6, CP-08-14, CP-08-16, CP-08-17, CP-08-36 |
| 30 | command | 08 | no value; items are actions, per-item `clicked()`; highlight in memory; closure items | CP-08-20, CP-08-21, CP-08-22, CP-08-31 |
| 31 | dropdown-menu | 09 | no value; `clicked()`; checkbox item `&mut bool`; radio item `&mut T` (`_value` form); closure items | CP-09-7, CP-09-10, CP-09-11, CP-09-12, CP-09-13, CP-09-35 |
| 32 | context-menu | 09 | as dropdown-menu | CP-09-26 |
| 33 | menubar | 09 | as dropdown-menu, per menu | — |
| 34 | popover | 10 | n/a | — |
| 35 | hover-card | 10 | n/a | — |
| 36 | tooltip | 10 | n/a | — |
| 37 | dialog | 11 | n/a | — |
| 38 | alert-dialog | 11 | n/a (choice = which button the caller sees clicked) | CP-11-17 |
| 39 | sheet | 11 | n/a | CP-11-25 |
| 40 | drawer | 11 | n/a (snap point would be one, omitted) | CP-11-26, CP-11-29 |
| 41 | tabs | 12 | generic `&mut V`, single; chained `.tab(value, atoms)`; `Response` / `show` `InnerResponse<R>` | CP-12-1, CP-12-8, CP-12-9 |
| 42 | accordion | 12 | generic `V`; single `&mut Option<V>`, multiple `&mut Vec<V>`; `acc.item(..)` in closure; `InnerResponse<R>` | CP-12-15, CP-12-16, CP-12-26 |
| 43 | collapsible | 12 | n/a | — |
| 44 | navigation-menu | 13 | no value; links return `Response`, `.active(bool)`; open item in memory | CP-13-9 |
| 45 | breadcrumb | 13 | none; link `Response`; shortcut returns `Option<usize>` clicked index | CP-13-18 |
| 46 | pagination | 13 | `&mut usize` page, 1-based, single; items computed; `Response` (shortcut `bool`) | CP-13-21, CP-13-22, CP-13-30 |
| 47 | sidebar | 14 | caller's page; `.active(bool)` + `clicked()`, and `menu_value(&mut V, V, ..)`; single | CP-14-8 |
| 48 | scroll-area | 14 | n/a | — |
| 49 | resizable | 14 | n/a | CP-14-34 |
| 50 | calendar | 15 | crate `Date`; single `Option<Date>`, multiple `Vec<Date>`, range; generated dates; `Response` (shortcut `bool`) | CP-15-1, CP-15-2, CP-15-3, CP-15-4, CP-15-5, CP-15-22, CP-15-29 |
| 51 | range-calendar | 15 | `&mut DateRange` with optional ends; `Response`, `changed()` | CP-15-30, CP-15-31 |
| 52 | table | 16 | none built in; caller passes `selected` per row; row `Response` | CP-16-5, CP-16-12 |
| 53 | carousel | 16 | `&mut usize` slide index (0-based), single; count + per-index closure; `Response` | CP-16-13 |
| 54 | sonner | 16 | n/a | — |
| 55 | chart | 17 | n/a for plot; legend toggles hidden keys `HashSet<&'static str>` or `&mut [bool]`; `Response` | CP-17-3, CP-17-8, CP-17-9 |
| 56 | attachment | 18 | n/a | — |
| 57 | bubble | 18 | n/a | — |
| 58 | marker | 18 | n/a | — |
| 59 | message | 19 | n/a | — |
| 60 | message-scroller | 19 | n/a | CP-19-11 |
| 61 | questionnaire | 19 | choice index `usize`, single or multiple in `Answer.selected: Vec<usize>`; `&[Question]`; `QuestionnaireOutput` | CP-19-22, CP-19-23, CP-19-29 |

### T2 · Open state

| # | component | report | answer (under 20 words) | CPs tagged 2 |
|---|---|---|---|---|
| 1 | button | 01 | n/a; `expanded` is a caller flag for a trigger | — |
| 2 | button-group | 01 | n/a | — |
| 3 | toggle | 01 | n/a | — |
| 4 | toggle-group | 01 | n/a | — |
| 5 | badge | 02 | n/a | — |
| 6 | kbd | 02 | n/a | — |
| 7 | spinner | 02 | n/a | — |
| 8 | skeleton | 02 | n/a | — |
| 9 | card | 03 | n/a | — |
| 10 | item | 03 | n/a | — |
| 11 | empty | 03 | n/a | — |
| 12 | aspect-ratio | 03 | n/a | — |
| 13 | alert | 04 | none; the caller decides whether to show it | — |
| 14 | progress | 04 | n/a | — |
| 15 | separator | 04 | n/a | — |
| 16 | avatar | 04 | n/a; loading state is egui's image cache | CP-04-23 |
| 17 | checkbox | 05 | n/a | — |
| 18 | radio-group | 05 | n/a | — |
| 19 | switch | 05 | n/a | — |
| 20 | slider | 05 | n/a | — |
| 21 | input | 06 | n/a | — |
| 22 | textarea | 06 | n/a | — |
| 23 | input-otp | 06 | n/a | — |
| 24 | input-group | 06 | n/a; popups hang off addon button responses, caller-owned | — |
| 25 | label | 07 | n/a | — |
| 26 | field | 07 | n/a | — |
| 27 | form | 07 | n/a | — |
| 28 | native-select | 07 | egui memory via `Popup` (`Popup::is_id_open`), as `ComboBox` and crate Select | — |
| 29 | select | 08 | egui memory via popup id, as today; no caller binding | CP-08-7 |
| 30 | command | 08 | inline: none; `CommandDialog::show(ctx, &mut bool, …)`; query text in memory | CP-08-23, CP-08-24 |
| 31 | dropdown-menu | 09 | egui memory via popup id; sub-menus in egui `MenuState`; memory only | CP-09-4, CP-09-19 |
| 32 | context-menu | 09 | egui memory, `Popup::context_menu` semantics; CP-09-4 applies | CP-09-25 |
| 33 | menubar | 09 | one "which menu is open" value per bar in memory, keyed by bar id | CP-09-29 |
| 34 | popover | 10 | egui memory by default; optional caller `&mut bool` via `.open_bool`; static `is_open` | CP-10-2, CP-10-24, CP-10-25 |
| 35 | hover-card | 10 | egui memory keyed by trigger id (flag + timer); no caller binding | CP-10-15 |
| 36 | tooltip | 10 | none of the caller's; egui decides per frame, plus keyboard focus | — |
| 37 | dialog | 11 | caller-owned `&mut bool` argument to `show`, cleared on dismissal | CP-11-1, CP-11-35 |
| 38 | alert-dialog | 11 | caller-owned `&mut bool`, like Dialog | CP-11-12 |
| 39 | sheet | 11 | caller-owned `&mut bool` | — |
| 40 | drawer | 11 | caller-owned `&mut bool`; drag offset in memory under drawer id | — |
| 41 | tabs | 12 | n/a; selection is caller-owned `&mut V` | — |
| 42 | accordion | 12 | caller-owned `&mut Option<V>` / `&mut Vec<V>` | CP-12-14 |
| 43 | collapsible | 12 | caller-owned `&mut bool` | CP-12-28 |
| 44 | navigation-menu | 13 | egui memory keyed by menu id, holds open item (index or salt) | CP-13-2, CP-13-14 |
| 45 | breadcrumb | 13 | n/a; demo dropdowns own their state | — |
| 46 | pagination | 13 | n/a | — |
| 47 | sidebar | 14 | memory (`default_open`, persisted) or caller `.open(&mut bool)`; static `is_open`/`toggle` | CP-14-1, CP-14-2, CP-14-3, CP-14-11, CP-14-43 |
| 48 | scroll-area | 14 | n/a; bar visibility timer in memory under area id | CP-14-28 |
| 49 | resizable | 14 | layout in memory (typed `State`) or caller `.layout(&mut Vec<f32>)`; persist opt-in | CP-14-33, CP-14-38 |
| 50 | calendar | 15 | n/a; view month and focused date in memory, optional `.view_month(&mut Date)` | CP-15-6, CP-15-7 |
| 51 | range-calendar | 15 | n/a; view month and hovered date in memory (CP-15-6) | — |
| 52 | table | 16 | n/a | — |
| 53 | carousel | 16 | n/a; scroll offset and drag velocity in memory; slide index caller `&mut usize` | — |
| 54 | sonner | 16 | queue in `ctx` memory; toast life is its timer; app holds `ToastId`s | CP-16-25 |
| 55 | chart | 17 | tooltip hover-only, no state; legend selection caller `&mut` | — |
| 56 | attachment | 18 | n/a; the trigger opens whatever the caller attaches | — |
| 57 | bubble | 18 | n/a | — |
| 58 | marker | 18 | n/a | — |
| 59 | message | 19 | n/a | — |
| 60 | message-scroller | 19 | n/a; scroll mode and commands in memory (typed `State`), associated fns | CP-19-10 |
| 61 | questionnaire | 19 | active item index caller-owned in `&mut QuestionnaireState` | CP-19-24 |

### T3 · Overlay

| component | report | answer (topic 3) | CPs tagged 3 |
|---|---|---|---|
| button | 01 | n/a | — |
| button-group | 01 | n/a (children may open their own) | — |
| toggle | 01 | n/a | — |
| toggle-group | 01 | n/a | — |
| badge | 02 | n/a | — |
| kbd | 02 | n/a (inside a tooltip is context, not an overlay) | — |
| spinner | 02 | n/a | — |
| skeleton | 02 | n/a | — |
| card | 03 | n/a | — |
| item | 03 | n/a | — |
| empty | 03 | n/a | — |
| aspect-ratio | 03 | n/a | — |
| alert | 04 | n/a | — |
| progress | 04 | n/a | — |
| separator | 04 | n/a | — |
| avatar | 04 | n/a (badge painted on the corner, no popup) | — |
| checkbox | 05 | n/a | — |
| radio-group | 05 | n/a (CP is mis-tagged: checked indicator painting) | CP-05-18 |
| switch | 05 | n/a | — |
| slider | 05 | n/a | — |
| input | 06 | n/a | — |
| textarea | 06 | n/a | — |
| input-otp | 06 | n/a | — |
| input-group | 06 | n/a | — |
| label | 07 | n/a | — |
| field | 07 | n/a | — |
| form | 07 | n/a | — |
| native-select | 07 | trigger, fixed `BOTTOM_START`, flips; outside click, Esc, choosing closes | CP-07-25 |
| select | 08 | trigger, fixed `BOTTOM_START` + `side_offset` 4, egui flip; Esc, outside click, Tab, trigger | CP-08-8, CP-08-9, CP-08-15 |
| command | 08 | none; CommandDialog modal at top third; Esc, backdrop; combobox = Popover + Command | CP-08-25 |
| dropdown-menu | 09 | trigger, `.align(RectAlign)` `BOTTOM_START`, gap 4, egui alternatives; subs `RIGHT_START`; Esc, outside, select | CP-09-1, CP-09-5, CP-09-20, CP-09-21, CP-09-22 |
| context-menu | 09 | pointer at open (fixed), `RIGHT_START`, gap 2, flips; Esc, outside, select, right click | CP-09-24 |
| menubar | 09 | below trigger, fixed `BOTTOM_START`, gap 8, align offset −4; as dropdown + menu switch | — |
| popover | 10 | trigger, `.align(RectAlign)` `BOTTOM`, gap 4, opposite-side flip; Esc, outside, `ui.close()`, trigger | CP-10-1, CP-10-7, CP-10-20 |
| hover-card | 10 | trigger, `.align(RectAlign)` `BOTTOM`, gap 4; pointer leave (delay), Esc, outside, blur | CP-10-19 |
| tooltip | 10 | trigger, `.align(RectAlign)` `TOP`, gap offset + arrow; pointer leave, click, Esc, scroll | CP-10-12, CP-10-13 |
| dialog | 11 | centred, no anchor; Esc, backdrop, close button, `ui.close()` | — |
| alert-dialog | 11 | centred; Esc, Cancel, `ui.close()`; backdrop ignored | CP-11-13 |
| sheet | 11 | window edge by `Side` (Right); Esc, backdrop, close button, `ui.close()` | CP-11-24 |
| drawer | 11 | window edge by `Side` (Bottom); Esc, backdrop, drag, `ui.close()`; none if not dismissible | CP-11-27, CP-11-28, CP-11-30 |
| tabs | 12 | n/a | — |
| accordion | 12 | n/a | — |
| collapsible | 12 | n/a | — |
| navigation-menu | 13 | root rect (viewport) or item rect, fixed `BOTTOM_START` gap 6; Esc, link, leave, blur, outside | CP-13-3, CP-13-4, CP-13-10, CP-13-11, CP-13-34 |
| breadcrumb | 13 | n/a | — |
| pagination | 13 | n/a | — |
| sidebar | 14 | desktop none; mobile Sheet (deferred); collapsed tooltip `RIGHT`, centred | CP-14-12, CP-14-23 |
| scroll-area | 14 | n/a (bars float over the viewport edge) | — |
| resizable | 14 | n/a | — |
| calendar | 15 | none; caption Select popup `BOTTOM_START`, outside click; date picker out of scope | CP-15-8 |
| range-calendar | 15 | as calendar | — |
| table | 16 | n/a | — |
| carousel | 16 | n/a; prev/next 48 px outside the rect | CP-16-16 |
| sonner | 16 | `Area` at corner/edge (`ToastPosition`, bottom-right, 24 px); timer, swipe, close, action, `dismiss` | CP-16-31, CP-16-32 |
| chart | 17 | tooltip at pointer, offset 10, flips inside chart rect upstream; pointer leave | CP-17-5 |
| attachment | 18 | n/a; vertical actions painted top-right inside the card | CP-18-9 |
| bubble | 18 | reactions in place on bubble edge (two enums side/align); no popup, no dismissal | CP-18-18, CP-18-23 |
| marker | 18 | n/a | — |
| message | 19 | n/a | — |
| message-scroller | 19 | scroll button painted bottom/top centre inside the scroller rect | — |
| questionnaire | 19 | n/a (dialog demo wraps it in Dialog) | — |

### T4 · Sub-parts

| # | component | report | answer (topic 4) | topic-4 CPs |
|---|---|---|---|---|
| 1 | button | 01 | One part; icons through `.icon` / `.icon_end`. | CP-01-2, CP-01-3, CP-01-32, CP-01-6 |
| 2 | button-group | 01 | `ButtonGroup::show`; `button_group_text`, `button_group_separator` free fns; children are crate widgets found by tag. | CP-01-7, CP-01-8, CP-01-10, CP-01-11, CP-01-13, CP-01-14 |
| 3 | toggle | 01 | One part; icon(s) plus text. | CP-01-16 |
| 4 | toggle-group | 01 | `ToggleGroup` with `ToggleItem` data list; per-item variant/size overrides dropped. | CP-01-23, CP-01-26 |
| 5 | badge | 02 | No parts; inline icons become `.icon` / `.icon_end` slots. | CP-02-1, CP-02-2, CP-02-8, CP-02-28 |
| 6 | kbd | 02 | `Kbd` widget; `Kbd.Group` becomes closure helper `kbd_group(ui, \|ui\| …)`. | CP-02-10, CP-02-12, CP-02-14 |
| 7 | spinner | 02 | No parts; hosts embed it in their glyph slot. | CP-02-18 |
| 8 | skeleton | 02 | No parts. | — |
| 9 | card | 03 | Header/Title/Description/Action builder setters; Content = `show` closure; Footer closure; free `card_title`/`card_description` kept. | CP-03-2, CP-03-3, CP-03-6, CP-03-9, CP-03-10 |
| 10 | item | 03 | Title/Description/Media builder data; Actions = `show` closure; Header/Footer closures; `ItemGroup`, `item_separator`, free title/description. | CP-03-11, CP-03-14, CP-03-15, CP-03-16, CP-03-18, CP-03-21, CP-03-22 |
| 11 | empty | 03 | Header implicit; Title/Description/Media builder data; Content = `show` closure. | CP-03-23, CP-03-24, CP-03-27, CP-03-28, CP-03-34 |
| 12 | aspect-ratio | 03 | One part. | CP-03-32, CP-03-33 |
| 13 | alert | 04 | Title/Description/Action/icon builder fields; rich description through `.show(ui, \|ui\| …)`. | CP-04-1, CP-04-3, CP-04-4, CP-04-7, CP-04-31 |
| 14 | progress | 04 | Indicator painted; no label/value part. | CP-04-12 |
| 15 | separator | 04 | No parts. | — |
| 16 | avatar | 04 | `new(fallback)` + `.image(src)`; `.badge(AvatarBadge)`; `AvatarGroup::new(avatars)` data list with `.count`. | CP-04-21, CP-04-22, CP-04-24, CP-04-26, CP-04-28, CP-04-32 |
| 17 | checkbox | 05 | Root + indicator → one widget; icon painted inside; no own label. | CP-05-2, CP-05-6 |
| 18 | radio-group | 05 | `RadioGroup::show` container; `RadioGroupItem` leaf found by `UiStack` tag; dot painted. | CP-05-8, CP-05-11, CP-05-20, CP-05-21 |
| 19 | switch | 05 | Root + Thumb → one widget. | — |
| 20 | slider | 05 | Root/Track/Range/Thumb → one widget. | CP-05-35 |
| 21 | input | 06 | No parts. | — |
| 22 | textarea | 06 | No parts; separate `Textarea` type. | CP-06-9 |
| 23 | input-otp | 06 | Group/Separator → `.groups(&[usize])`; slots and caret painted. | CP-06-14, CP-06-19 |
| 24 | input-group | 06 | `InputGroup` builder; control argument; `.addon(align, closure)`; plain labels in addon; `InputGroupButton` type. | CP-06-24, CP-06-25, CP-06-26, CP-06-27, CP-06-28, CP-06-32, CP-06-36, CP-06-37 |
| 25 | label | 07 | No parts. | CP-07-2 |
| 26 | field | 07 | Field/FieldSet/FieldGroup/FieldCard containers; label/title/description/error slots; free `field::description/error/separator`. | CP-07-5, CP-07-7, CP-07-12, CP-07-15, CP-07-17, CP-07-19 |
| 27 | form | 07 | Parts fold into Field/FieldSet slots; Form.Button → Button; array fields → caller loop. | CP-07-20 |
| 28 | native-select | 07 | `NativeSelect`; `option(..)`; `opt_group(..)` heading followed by its options. | CP-07-22, CP-07-28 |
| 29 | select | 08 | One widget; `SelectGroup` with `.label`; `SelectItem`; `.group_separators(true)`; ScrollArea for scroll buttons. | CP-08-3, CP-08-18 |
| 30 | command | 08 | `Command` draws input; `cmd.group(heading, …)`, `cmd.separator()`, `CommandItem` with `.shortcut`; `CommandDialog`. | CP-08-19, CP-08-26, CP-08-27, CP-08-28, CP-08-33 |
| 31 | dropdown-menu | 09 | `DropdownMenu::new(&resp).show`; shared item widgets; `menu_label`, `menu_separator`; `MenuSub::show`; no group container. | CP-09-2, CP-09-8, CP-09-15, CP-09-16, CP-09-17 |
| 32 | context-menu | 09 | `ContextMenu::new(&resp).show` on any `Response`; rest as dropdown-menu. | — |
| 33 | menubar | 09 | `Menubar::show`; `MenubarMenu::new(text).show` / `menubar_menu`; items as dropdown-menu. | CP-09-27, CP-09-34 |
| 34 | popover | 10 | One `Popover` builder on trigger `Response`; header/title/description free fns; Close → `ui.close()`. | CP-10-4, CP-10-5 |
| 35 | hover-card | 10 | One `HoverCard` builder on trigger `Response`; free-form content. | — |
| 36 | tooltip | 10 | Provider → egui style; `Tooltip::new(&resp).show` or `tooltip(resp, text)`; arrow painted. | CP-10-9, CP-10-26 |
| 37 | dialog | 11 | Folds into `Dialog::show`; `new(title)` + `.description()`; corner close; `dialog::footer(ui, ..)`. | CP-11-4, CP-11-5, CP-11-6, CP-11-9 |
| 38 | alert-dialog | 11 | `new(title)`, `.description()`, `.media()`; `show` closure is the footer; `alert_dialog::cancel/action` fns. | CP-11-10, CP-11-11, CP-11-14 |
| 39 | sheet | 11 | As Dialog; header padded, body unpadded; `sheet::footer` pinned to bottom. | CP-11-21, CP-11-22, CP-11-23 |
| 40 | drawer | 11 | Folds into `Drawer::show`; handle bar drawn; header as Sheet; `drawer::footer`. | CP-11-32 |
| 41 | tabs | 12 | Root+List → `Tabs`; Trigger → `.tab(..)`; Content → `show` closure or caller `match`. | CP-12-2, CP-12-3, CP-12-4, CP-12-13 |
| 42 | accordion | 12 | `Accordion::single/multiple(..).show`; `acc.item(value, text).show(\|ui\| ..)`; chevrons drawn. | CP-12-17, CP-12-18, CP-12-20, CP-12-21, CP-12-23, CP-12-24, CP-12-33 |
| 43 | collapsible | 12 | Root → caller `bool`; `collapsible_trigger(response, &mut open)`; Content → `if open {}`. | CP-12-27, CP-12-30 |
| 44 | navigation-menu | 13 | `NavigationMenu`; `nav.item(trigger, \|ui\| content)`; `nav.link`; `NavigationMenuLink` widget; Indicator not ported. | CP-13-1, CP-13-5, CP-13-13 |
| 45 | breadcrumb | 13 | `Breadcrumb::show`; `BreadcrumbUi` methods for item/link/page/ellipsis; separators automatic. | CP-13-15, CP-13-16, CP-13-33, CP-13-31, CP-13-19, CP-13-20 |
| 46 | pagination | 13 | One widget with `numbers`/`prev_next` switches; public `page_items` for custom layouts. | CP-13-24, CP-13-27, CP-13-35 |
| 47 | sidebar | 14 | One `Sidebar` builder; header/footer closures; `sidebar::Group`; `MenuButton`; `menu_sub`; `sidebar::trigger/inset/input/separator` fns. | CP-14-4, CP-14-5, CP-14-6, CP-14-7, CP-14-9, CP-14-10, CP-14-17, CP-14-19, CP-14-21, CP-14-22, CP-14-45, CP-14-46, CP-14-47 |
| 48 | scroll-area | 14 | Root/Viewport/Scrollbar/Thumb/Corner → one `ScrollArea`; orientation → constructor. | CP-14-27, CP-14-30 |
| 49 | resizable | 14 | `ResizablePanes`; `Pane` spec + index in closure; handles drawn between every pair. | CP-14-32, CP-14-35 |
| 50 | calendar | 15 | 17 parts collapse into one widget; `day` snippet → `day_note`; `buttonVariant` not exposed. | CP-15-9, CP-15-10, CP-15-11, CP-15-36 |
| 51 | range-calendar | 15 | As calendar; the same 17 parts. | — |
| 52 | table | 16 | One `Table` builder; `header`/`row`/`footer` on body; `text`/`cell` on row; `.caption(..)`. | CP-16-1, CP-16-6, CP-16-7, CP-16-8 |
| 53 | carousel | 16 | `Carousel::show` with per-slide closure; built-in prev/next by `.buttons(bool)`; `&mut usize` + count. | CP-16-14, CP-16-15, CP-16-20, CP-16-41 |
| 54 | sonner | 16 | `Toaster::show(ctx)`; `Toast` builder + free fns; promise → loading toast by id; custom toasts not offered. | CP-16-24, CP-16-26, CP-16-27, CP-16-28, CP-16-36, CP-16-38, CP-16-44 |
| 55 | chart | 17 | `chart_plot(..)` + `ChartConfig`; `ChartTooltip`, `ChartLegend` widgets; series → `egui_plot` items. | CP-17-1, CP-17-2, CP-17-4, CP-17-6, CP-17-7, CP-17-10, CP-17-12, CP-17-14 |
| 56 | attachment | 18 | `Attachment` builder; media `.icon/.image/.spinner`; Actions = `show` closure; `attachment_action()`, `attachment_group()`. | CP-18-1, CP-18-2, CP-18-3, CP-18-4, CP-18-13 |
| 57 | bubble | 18 | Root+Content → one `Bubble`; `.reactions(closure)`; `bubble_group()`. | CP-18-14, CP-18-17, CP-18-21 |
| 58 | marker | 18 | `Marker`; Icon → `.icon()` / `.spinner()`; Content → `new(text)`. | CP-18-24, CP-18-26, CP-18-33, CP-18-35 |
| 59 | message | 19 | `Message` builder; avatar/header/footer setters; Content = `show` closure; `message_group` free fn. | CP-19-1, CP-19-2, CP-19-3, CP-19-4 |
| 60 | message-scroller | 19 | Provider/Root/Viewport/Content → one `MessageScroller`; `s.item(..)`; `.scroll_button(..)`; hooks → assoc fns. | CP-19-9, CP-19-12, CP-19-13, CP-19-14, CP-19-39 |
| 61 | questionnaire | 19 | All parts become data on `Question`/`Choice`; component lays out everything; custom layouts limited. | CP-19-21, CP-19-25, CP-19-26, CP-19-33, CP-19-42 |

### T5 · Container

| # | Component | Report | Answer (topic 5) | CPs tagged 5 |
|---|---|---|---|---|
| 1 | button | 01 | `ui.add` → `Response` | — |
| 2 | button-group | 01 | `show(ui, \|ui\| …) -> InnerResponse<R>` | — |
| 3 | toggle | 01 | `ui.add` → `Response` | — |
| 4 | toggle-group | 01 | `ui.add` → `Response` (data list of items) | — |
| 5 | badge | 02 | `ui.add` → `Response` | — |
| 6 | kbd | 02 | `Kbd` `ui.add` → `Response`; group: closure helper → `InnerResponse<R>` | — |
| 7 | spinner | 02 | `ui.add` → `Response`; `paint_at` for host widgets | — |
| 8 | skeleton | 02 | `ui.add` → `Response` | — |
| 9 | card | 03 | `show(ui, \|ui\| ..) -> InnerResponse<R>` (today `R`) | CP-03-1 |
| 10 | item | 03 | `ui.add(Item)` without actions; `show(ui, actions) -> InnerResponse<R>` with them | CP-03-12 |
| 11 | empty | 03 | `show(ui, content) -> InnerResponse<R>`; `ui.add(Empty)` without content | — |
| 12 | aspect-ratio | 03 | `show(ui, add) -> InnerResponse<R>`; child `max_rect` is the ratio box | CP-03-29 |
| 13 | alert | 04 | `ui.add` → `Response` text-only; `show` → `InnerResponse<R>` custom body | CP-04-2 |
| 14 | progress | 04 | `ui.add` → `Response` | — |
| 15 | separator | 04 | `ui.add` → `Response`; struct plus two free shortcuts | CP-04-15 |
| 16 | avatar | 04 | `ui.add` → `Response` for `Avatar` and `AvatarGroup` | CP-04-29 (mis-tagged) |
| 17 | checkbox | 05 | `ui.add` → `Response` | — |
| 18 | radio-group | 05 | group `show(ui, \|ui\| …) -> InnerResponse<R>`; items `ui.add` → `Response` | CP-05-8 |
| 19 | switch | 05 | `ui.add` → `Response` | — |
| 20 | slider | 05 | `ui.add` → `Response` | — |
| 21 | input | 06 | `ui.add` → `Response`; plus `show(ui) -> TextEditOutput` | CP-06-3 |
| 22 | textarea | 06 | `ui.add` → `Response`; plus `show(ui) -> TextEditOutput` | — |
| 23 | input-otp | 06 | `ui.add` → `Response`; plus `show(ui) -> InputOtpOutput` (completion) | CP-06-16 |
| 24 | input-group | 06 | `ui.add` → the control's `Response` (stored addon closures) | CP-06-29 |
| 25 | label | 07 | `ui.add` → `Response` | — |
| 26 | field | 07 | `show(ui, \|ui\| …) -> InnerResponse<R>`; `Field::add(ui, widget) -> Response` | CP-07-6 |
| 27 | form | 07 | n/a | — |
| 28 | native-select | 07 | `show_ui(ui, closure) -> InnerResponse<Option<R>>` (egui `ComboBox` form) | — |
| 29 | select | 08 | `ui.add(Select)` → `Response` (data slice) | — |
| 30 | command | 08 | `show(ui, \|cmd\| …) -> InnerResponse<R>`; `CommandDialog::show(ctx, &mut open, …) -> Option<R>` | CP-08-29 |
| 31 | dropdown-menu | 09 | `show(add) -> Option<R>`; shortcut `InnerResponse<Option<R>>`; items `ui.add` | CP-09-3 |
| 32 | context-menu | 09 | `show(add) -> Option<R>`; shortcut `context_menu(&resp, add) -> Option<R>` | CP-09-23 |
| 33 | menubar | 09 | bar `show(ui, add) -> InnerResponse<R>`; menu `show -> InnerResponse<Option<R>>` | CP-09-28 |
| 34 | popover | 10 | `.show(\|ui\| …)` → `Option<InnerResponse<R>>` (today `Option<R>`) | CP-10-3, CP-10-25 |
| 35 | hover-card | 10 | `.show(\|ui\| …)` → `Option<InnerResponse<R>>` | CP-10-14 |
| 36 | tooltip | 10 | builder `.show` → `Option<InnerResponse<R>>`; shortcut returns the trigger `Response` | — |
| 37 | dialog | 11 | `show(ctx, &mut open, \|ui\| ..) -> Option<InnerResponse<R>>` (today `Option<R>`) | CP-11-2, CP-11-3 |
| 38 | alert-dialog | 11 | `show(ctx, &mut open, \|ui\| ..) -> Option<InnerResponse<R>>` | — |
| 39 | sheet | 11 | `show(ctx, &mut open, \|ui\| ..) -> Option<InnerResponse<R>>` | — |
| 40 | drawer | 11 | `show(ctx, &mut open, \|ui\| ..) -> Option<InnerResponse<R>>` | — |
| 41 | tabs | 12 | both: `ui.add` list alone → `Response`; `.show(ui, ..)` list + content → `InnerResponse<R>` | CP-12-4 |
| 42 | accordion | 12 | `show` → `InnerResponse<R>`; item `show` → `CollapsingResponse<R>` | CP-12-19 |
| 43 | collapsible | 12 | no container; helper takes and returns a `Response` | — |
| 44 | navigation-menu | 13 | `show(ui, \|nav\| …) -> InnerResponse<R>`; `item` → `InnerResponse<Option<R>>` | CP-13-32 |
| 45 | breadcrumb | 13 | `show(ui, \|bc\| …) -> InnerResponse<R>` | — |
| 46 | pagination | 13 | `ui.add(Pagination)` → `Response` | CP-13-29 (mis-tagged) |
| 47 | sidebar | 14 | `show_inside(ui, ..) -> InnerResponse<R>`; parts `ui.add`, `show`, `GroupOutput<R>`, `MenuButtonOutput` | — |
| 48 | scroll-area | 14 | `show(ui, ..) -> ScrollAreaOutput<R>` (egui's type) | CP-14-26 |
| 49 | resizable | 14 | `show(ui, \|ui, i\| ..) -> ResizableOutput<R>` | CP-14-40 |
| 50 | calendar | 15 | `ui.add(Calendar…)` → `Response` | — |
| 51 | range-calendar | 15 | `ui.add` → `Response` | — |
| 52 | table | 16 | `show(ui, \|t\| ..) -> InnerResponse<R>` | — |
| 53 | carousel | 16 | `show(ui, \|ui, i\| ..)` (`FnMut`) → `Response` with `changed()` | — |
| 54 | sonner | 16 | `Toaster::show(&Context) -> Vec<ToastEvent>`; `Toast::show(&Context) -> ToastId` | — |
| 55 | chart | 17 | tooltip, legend `ui.add` → `Response`; plot `show(ui, \|plot_ui\| …)` → `PlotResponse<R>` | — |
| 56 | attachment | 18 | `ui.add` without actions; `.show(ui, \|ui\| actions) -> InnerResponse<R>` with them | CP-18-10 |
| 57 | bubble | 18 | `show(ui, \|ui\| …) -> InnerResponse<R>`; free `bubble(ui, text, variant) -> Response` | CP-18-14 |
| 58 | marker | 18 | `ui.add(Marker…)` → `Response` | — |
| 59 | message | 19 | `show(ui, \|ui\| ..) -> InnerResponse<R>`; group as free fn | CP-19-7 |
| 60 | message-scroller | 19 | `show(ui, \|s\| ..) -> MessageScrollerOutput<R>` (like `ScrollAreaOutput`) | CP-19-17 |
| 61 | questionnaire | 19 | `impl Widget` → `Response`; plus `show(ui) -> QuestionnaireOutput` | CP-19-28 |

### T6 · Identity

| # | Component | Report | Answer (under 20 words) | Topic-6 CPs |
|---|---|---|---|---|
| 1 | button | 01 | egui auto id from `allocate_exact_size` | — |
| 2 | button-group | 01 | `ui.next_auto_id()` or optional `.id_salt`; keys the stored child count | — |
| 3 | toggle | 01 | auto id | — |
| 4 | toggle-group | 01 | allocation auto id or optional `.id_salt`; items `id.with(("item", i))`; tab stop in memory | — |
| 5 | badge | 02 | auto id; no state | — |
| 6 | kbd | 02 | auto id; no state | — |
| 7 | spinner | 02 | auto id; no state (angle from time) | — |
| 8 | skeleton | 02 | auto id; no state | — |
| 9 | card | 03 | none needed (passive); size context reaches free parts through the Ui | CP-03-7 |
| 10 | item | 03 | auto id of the rect; clickable item uses the `UiBuilder::sense` scope id | — |
| 11 | empty | 03 | none needed | — |
| 12 | aspect-ratio | 03 | none needed | — |
| 13 | alert | 04 | no state; `ui.next_auto_id()` only for AccessKit | — |
| 14 | progress | 04 | none needed; `next_auto_id` only if animated | — |
| 15 | separator | 04 | none | — |
| 16 | avatar | 04 | none of its own; image cache keys by URI | — |
| 17 | checkbox | 05 | auto id; hit area `id.with("hit")` | — |
| 18 | radio-group | 05 | required `id_salt` in `new` (roving state in memory); items auto ids | CP-05-12 |
| 19 | switch | 05 | auto id, hit `id.with("hit")`; animation keyed on base id | — |
| 20 | slider | 05 | auto id; per thumb `id.with(index)` | — |
| 21 | input | 06 | `TextEdit` auto id or optional `id_salt` hashed in the showing Ui; drop `id_source` | CP-06-8 |
| 22 | textarea | 06 | as Input | — |
| 23 | input-otp | 06 | auto id or optional `id_salt`; one focusable id for the widget | — |
| 24 | input-group | 06 | the control's id (Input/Textarea `id_salt`); addons auto ids | — |
| 25 | label | 07 | auto id; no state | — |
| 26 | field | 07 | no state; auto ids; `id_salt` only when a caller needs it | — |
| 27 | form | 07 | n/a (egui uses `labelled_by`) | — |
| 28 | native-select | 07 | required: `from_id_salt(salt)` → popup id, like `ComboBox` | CP-07-32 |
| 29 | select | 08 | required `id_salt: impl Hash` in `new`, then `ui.make_persistent_id` | CP-08-10 |
| 30 | command | 08 | required `id_salt` in `new`; items keyed by `value` within the command id | (CP-08-10) |
| 31 | dropdown-menu | 09 | popup id from trigger `Response`; sub-menu from sub-trigger id; highlight under popup id | — |
| 32 | context-menu | 09 | popup id from the target `Response` id | — |
| 33 | menubar | 09 | bar: optional `id_salt`, else Ui auto id; menus from trigger response id | CP-09-30 |
| 34 | popover | 10 | `trigger.id.with("popup")` (egui default); optional `.id_salt` | CP-10-17 |
| 35 | hover-card | 10 | `trigger.id.with("hover_card")`; optional `.id_salt` | CP-10-17 |
| 36 | tooltip | 10 | egui `Tooltip::next_tooltip_id(ctx, trigger.id)` | — |
| 37 | dialog | 11 | required `id_salt` in `new`; `Id::new(id_salt)` global, to `Modal::new` | CP-11-8 |
| 38 | alert-dialog | 11 | `Id::new(id_salt)` (global) | CP-11-8 |
| 39 | sheet | 11 | `Id::new(id_salt)` (global) | CP-11-8 |
| 40 | drawer | 11 | `Id::new(id_salt)` (global); drag state under the same id | CP-11-8 |
| 41 | tabs | 12 | optional `id_salt`, else allocation auto id; triggers `id.with(("tab", i))` | CP-12-11 |
| 42 | accordion | 12 | optional `id_salt`, else `next_auto_id`; item id = root `.with(index)`, not value | CP-12-11 |
| 43 | collapsible | 12 | none needed (caller's bool) | — |
| 44 | navigation-menu | 13 | required `NavigationMenu::new(id_salt)`; item id `menu_id.with(index)` | — |
| 45 | breadcrumb | 13 | no state, no id; items auto ids | — |
| 46 | pagination | 13 | auto id or optional `id_salt`, for focus movement; no stored state | — |
| 47 | sidebar | 14 | required `Sidebar::left(id: impl Into<Id>)` like `Panel::left`; menu buttons auto id | — |
| 48 | scroll-area | 14 | optional `id_salt`, as egui | — |
| 49 | resizable | 14 | required `id_salt` in constructor (`make_persistent_id`); handles `id.with(("handle", i))` | — |
| 50 | calendar | 15 | optional `id_salt` (default `next_auto_id`); memory under `ui.id().with(salt)` | CP-15-12 |
| 51 | range-calendar | 15 | as calendar | (CP-15-12) |
| 52 | table | 16 | required `id_salt` in `new`; column widths in memory under it | CP-16-10 (mis-tagged) |
| 53 | carousel | 16 | required `id_salt` in `new` | CP-16-23 (mis-tagged) |
| 54 | sonner | 16 | `ToastId` newtype, auto-incrementing or caller-given via `.id(ToastId)`; fixed toaster `Area` id | — |
| 55 | chart | 17 | required plot `id_salt` (`chart_plot(id_salt)`); tooltip from plot response; legend `next_auto_id` | — |
| 56 | attachment | 18 | `next_auto_id` default, optional `.id_salt()` for reordering lists | CP-18-11 |
| 57 | bubble | 18 | auto id, only with `sense`; no state | — |
| 58 | marker | 18 | auto id, only with `sense`/`link` | — |
| 59 | message | 19 | none needed (no state) | — |
| 60 | message-scroller | 19 | required `id_salt` (`make_persistent_id`); items keyed by `message: impl Hash` | (CP-19-11, tagged 1) |
| 61 | questionnaire | 19 | optional `id_salt` (default `next_auto_id`); choice focus `id.with((q, c))` | — |

### T7 · Orientation and direction

| Component | Report | Answer (under 20 words) | CPs tagged 7 |
|---|---|---|---|
| button | 01 | n/a | — |
| button-group | 01 | `ButtonGroupOrientation` + `.orientation()` + `.vertical()`; RTL ignored | CP-01-9 |
| toggle | 01 | n/a | — |
| toggle-group | 01 | horizontal default, `.vertical()` only (private `ToggleGroupOrientation` field); Up/Down when vertical; RTL ignored | CP-01-29, CP-01-30, CP-01-31 (crate-wide) |
| badge | 02 | n/a (LTR only) | — |
| kbd | 02 | horizontal only | — |
| spinner | 02 | n/a | — |
| skeleton | 02 | n/a | — |
| card | 03 | n/a; footer is a fixed row | CP-03-3 |
| item | 03 | n/a; row, header/footer full-width rows | — |
| empty | 03 | n/a; centred column | — |
| aspect-ratio | 03 | n/a | — |
| alert | 04 | n/a | — |
| progress | 04 | horizontal only (upstream has no orientation) | — |
| separator | 04 | `.horizontal()` / `.vertical()`, no public enum; default inferred from `Ui` layout | CP-04-17, CP-04-20, CP-04-33 |
| avatar | 04 | group left-to-right with −8px overlap; otherwise n/a | — |
| checkbox | 05 | n/a | — |
| radio-group | 05 | no orientation; arrows take both axes; caller lays items out; RTL ignored | CP-05-13 |
| switch | 05 | n/a (RTL ignored) | — |
| slider | 05 | shared `Orientation` + `.vertical()`; vertical min at bottom; `dir` not offered | CP-05-37 |
| input | 06 | n/a | — |
| textarea | 06 | n/a | — |
| input-otp | 06 | horizontal only; RTL ignored | CP-06-23 (mis-tagged) |
| input-group | 06 | `AddonAlign::{InlineStart, InlineEnd, BlockStart, BlockEnd}` (logical names) | CP-06-31 (mis-tagged) |
| label | 07 | n/a | — |
| field | 07 | `FieldOrientation::{Vertical, Horizontal, Responsive}` + `.horizontal()`; Responsive at 448 pt | CP-07-7, CP-07-14, CP-07-29 |
| form | 07 | n/a | — |
| native-select | 07 | n/a; left to right | — |
| select | 08 | n/a (vertical list) | — |
| command | 08 | vertical list only; grid `columns` mode omitted | CP-08-30 |
| dropdown-menu | 09 | vertical list; LTR only (`cn-rtl-flip` not ported) | — |
| context-menu | 09 | vertical; LTR only | — |
| menubar | 09 | bar horizontal, menus vertical; LTR only | — |
| popover | 10 | n/a (side lives in `RectAlign`) | — |
| hover-card | 10 | n/a | — |
| tooltip | 10 | n/a (side is in `align`); arrow on all four sides | CP-10-23 |
| dialog | 11 | n/a; footer row fixed, right to left; `sm:` always true | — |
| alert-dialog | 11 | footer row right-aligned (`sm:` always true); size `sm`: 2-column grid | — |
| sheet | 11 | `.side(Side)`, default Right; footer vertical column | CP-11-18, CP-11-19 |
| drawer | 11 | `.side(Side)`, default Bottom; drag axis follows side | CP-11-34 |
| tabs | 12 | `TabsOrientation` + `.orientation()` + `.vertical()`; vertical uses Up/Down, content right | CP-12-5, CP-12-31 |
| accordion | 12 | vertical only; Up/Down navigation | — |
| collapsible | 12 | n/a | — |
| navigation-menu | 13 | horizontal and LTR only; bits vertical not offered | CP-13-6 |
| breadcrumb | 13 | horizontal, wrapping; LTR | — |
| pagination | 13 | horizontal and LTR; bits vertical (arrow axis) omitted | CP-13-25 |
| sidebar | 14 | `Sidebar::left` / `right` constructors; vertical only; LTR | CP-14-13 |
| scroll-area | 14 | `ScrollArea::vertical()` / `horizontal()` / `both()` constructors (egui form) | — |
| resizable | 14 | `ResizablePanes::horizontal()` / `vertical()` constructors; nesting by `show` in a pane | CP-14-36 |
| calendar | 15 | months side by side (`md:` always); `rtl` ignored; `week_start` | CP-15-13, CP-15-14 |
| range-calendar | 15 | as calendar; two months side by side | — |
| table | 16 | n/a (LTR; `text-end` = `align_end()`) | — |
| carousel | 16 | `CarouselOrientation` + `.orientation()` + `.vertical()`; Left/Right keys in both orientations | CP-16-17, CP-16-40 |
| sonner | 16 | n/a; stacking follows y position (top grows down, bottom grows up) | — |
| chart | 17 | upstream bar orientation and `seriesLayout`; not in sketch; legend horizontal | — |
| attachment | 18 | `AttachmentOrientation` + `.orientation()` + `.vertical()` | CP-18-5 |
| bubble | 18 | `BubbleAlign::{Start, End}`, inherited from enclosing `Message`; reactions side/align enums | CP-18-15, CP-18-16 |
| marker | 18 | none; stacked `flex-col` is demo-only, left out | CP-18-28, CP-18-34 (crate-wide) |
| message | 19 | `MessageAlign::{Start, End}` + `.end()`; End mirrors row; `rtl` not ported | CP-19-6, CP-19-8 |
| message-scroller | 19 | vertical only; scroll button edge `ScrollEdge::{Start, End}` | — |
| questionnaire | 19 | vertical only; actions row Previous left, Skip/Next right; `rtl` not ported | CP-19-36 (mis-tagged) |

### T8 · Sizing

| Component | Report | Answer (topic 8) | CPs tagged 8 |
|---|---|---|---|
| button | 01 | `ButtonSize` (8 variants incl. `Icon*`); sketch adds `.min_width(f32)` for stretching | CP-01-5 |
| button-group | 01 | content width; vertical stretches children to widest (previous-pass width) | CP-01-12 |
| toggle | 01 | `ToggleSize::{Sm, Default, Lg}`; width from content, min width = height | — |
| toggle-group | 01 | `ToggleSize` on group; `spacing` gap in points; vertical stretches to widest | CP-01-25 |
| badge | 02 | content-sized (`w-fit`, `h-5`); no variants | CP-02-7 (colours) |
| kbd | 02 | content-sized, min width/height from recipe; no variants | — |
| spinner | 02 | `.size(f32)` in px, default from recipe | CP-02-16 |
| skeleton | 02 | `new(impl Into<Vec2>)`; `f32::INFINITY` width = fill; no variants | CP-02-23, CP-02-24, CP-02-25 |
| card | 03 | fills width, `.max_width(f32)`; `CardSize::{Default, Sm}` | CP-03-4, CP-03-5 |
| item | 03 | fills width; `ItemSize::{Default, Sm, Xs}`; title/description line clamp | CP-03-17, CP-03-20 |
| empty | 03 | fills width, header/content capped `max-w-sm`; height = content | CP-03-25, CP-03-26 |
| aspect-ratio | 03 | width = available (or `max_width`), height = width / ratio | CP-03-30 |
| alert | 04 | fills width; `.max_width(f32)` (CP) but sketch lacks it; no variants | CP-04-5 |
| progress | 04 | fills width by default, `.width(f32)`; height from recipe; no variants | CP-04-11 |
| separator | 04 | full width / row height; `.length(f32)` not offered; no variants | CP-04-18 |
| avatar | 04 | `AvatarSize::{Sm, Default, Lg}`; group count follows group avatar size | CP-04-27 |
| checkbox | 05 | fixed by recipe (`size-4`); no variants | — |
| radio-group | 05 | item fixed by recipe; group fills width with recipe gap | — |
| switch | 05 | `SwitchSize::{Default, Sm}`; fixed by recipe | CP-05-24 |
| slider | 05 | horizontal fills width unless `.width(f32)`; vertical `min-h-40` from recipe | CP-05-36 |
| input | 06 | height from recipe; width up to `max_width` (default `Metrics::field_max_width`) | CP-06-4 |
| textarea | 06 | grows from `min-h-16`, `rows` raises minimum; width up to `max_width` | CP-06-10, CP-06-11 |
| input-otp | 06 | slot size from recipe, `slot_size(Vec2)` override; gap from recipe | CP-06-22 |
| input-group | 06 | group owns `max_width` (default field max); height from recipe; `InputGroupButtonSize` | CP-06-33, CP-06-35 |
| label | 07 | content width; wrap at available width (CP) | CP-07-3 |
| field | 07 | fills width; no width setter, caller limits via layout | CP-07-11 |
| form | 07 | n/a | — |
| native-select | 07 | fills width, `.width(f32)`; size `Default`/`Sm` (re-export `SelectSize`) | CP-07-23, CP-07-26 (placeholder) |
| select | 08 | `.width` (fixed) and `.max_width` (cap); default widest option; `SelectSize::{Default, Sm}` | CP-08-11 |
| command | 08 | fills parent, optional `max_width`; list `max-h-72` from recipe; no variants | — |
| dropdown-menu | 09 | `.width(f32)`; default max(trigger width, recipe min-w); height capped by ScrollArea | CP-09-6 |
| context-menu | 09 | content width (recipe min-w), `.width()`; height capped by ScrollArea | — |
| menubar | 09 | bar at content width, `.width()`; height from recipe | CP-09-31 |
| popover | 10 | `.width(f32)` over recipe `w-72`; no `max_width`; no variants | CP-10-18 |
| hover-card | 10 | `.width(f32)` over recipe `w-64`/`w-72` | CP-10-18 |
| tooltip | 10 | recipe `w-fit max-w-xs`; no setter | CP-10-18 |
| dialog | 11 | `width(px)` replaces `sm:max-w-*`; capped to window minus 2rem; no variants | — |
| alert-dialog | 11 | `AlertDialogSize::{Default, Sm}` mapping to recipe max widths | CP-11-15 |
| sheet | 11 | one `size(px)`: width (Left/Right) or height (Top/Bottom) | CP-11-20 |
| drawer | 11 | `size(px)`; defaults from content classes (`max-h-[80vh]`, `w-3/4`) | — (follows CP-11-20) |
| tabs | 12 | `.width(f32)` for horizontal list; vertical list = widest trigger; no variants | — |
| accordion | 12 | fills width; trigger text wraps; no variants | — |
| collapsible | 12 | n/a (trigger widget sizes itself) | — |
| navigation-menu | 13 | root content width; content width is caller's; viewport = content + 1rem | CP-13-7 |
| breadcrumb | 13 | list wraps at available width; item truncation left to caller | CP-13-17 |
| pagination | 13 | page link takes `ButtonSize` (default `Icon`); prev/next `Default`; never centres | CP-13-26, CP-13-35 |
| sidebar | 14 | `.width` (16rem), `.icon_width` (3rem); `MenuButtonSize` `Default/Sm/Lg`; sub button `Sm/Md` | CP-14-14, CP-14-15, CP-14-25 (colours) |
| scroll-area | 14 | `max_height`, `max_width`, `auto_shrink` (egui's); bar thickness from recipe | CP-14-31 |
| resizable | 14 | fills available rect; horizontal group needs height from parent or `.height` | CP-14-41 |
| calendar | 15 | no width; size = 7 × `cell_size` + padding; `cell_size(px)` override | CP-15-15, CP-15-16 |
| range-calendar | 15 | as calendar; demo overrides cell size 40/48 → `cell_size` | — (CP-15-15, CP-15-16 cover it) |
| table | 16 | fills width; per-column `Column::exact/auto/remainder`; horizontal scroll | CP-16-2, CP-16-3, CP-16-4 |
| carousel | 16 | width from parent; slide size from `per_view` and `spacing(px)`; vertical needs parent height | CP-16-18, CP-16-19, CP-16-21 |
| sonner | 16 | width 356; narrow windows full width minus 2×16; height from content | CP-16-29, CP-16-30, CP-16-35, CP-16-37 |
| chart | 17 | fills width; `.height(..)` and `.aspect(16/9)`; tooltip `min-w-32` | — |
| attachment | 18 | `AttachmentSize::{Default, Sm, Xs}`; `.width(f32)`; min widths from recipe | CP-18-6 |
| bubble | 18 | `w-fit` up to 80 % of parent (ghost 100 %); no variants | CP-18-20 |
| marker | 18 | full width (generator ignores `w-full`, widget claims width); no variants | — |
| message | 19 | fills width; no variants | — |
| message-scroller | 19 | fills available rect; no variants; button `IconSm` | CP-19-19 (timing) |
| questionnaire | 19 | fills width; caller limits with parent `max_width`; nav buttons `default` | — |

### T9 · State flags

| # | Component | Report | Answer (topic 9) | Topic-9 CPs |
|---|---|---|---|---|
| 1 | button | 01 | Disabled per CP-01-1 (sketch: via `Ui`); `invalid`, `expanded`, `has_popup` as `bool` setters; group position from enclosing group. | CP-01-1, (CP-01-31) |
| 2 | button-group | 01 | None on the group; children get `IN_BUTTON_GROUP` + position from context. | (CP-01-31) |
| 3 | toggle | 01 | `invalid`; pressed from `&mut bool`; disabled per CP-01-1. | CP-01-17, CP-01-18, CP-01-19, (CP-01-1, CP-01-31) |
| 4 | toggle-group | 01 | Per-item `ToggleItem::enabled(bool)`; group disabled per CP-01-1; no group `invalid`. | CP-01-27, CP-01-28, CP-01-31, (CP-01-1 per-item) |
| 5 | badge | 02 | `.invalid(bool)`, `.link(bool)`; disabled via `ui.add_enabled` only. | CP-02-3, CP-02-4, CP-02-5, CP-02-9, CP-02-29 |
| 6 | kbd | 02 | None; context flags `IN_TOOLTIP`, `IN_INPUT_GROUP` detected from `ui.stack()`. | CP-02-13 |
| 7 | spinner | 02 | None; disabled via host `Ui`. | CP-02-17, CP-02-19, CP-02-20 |
| 8 | skeleton | 02 | None. | CP-02-26, CP-02-27 |
| 9 | card | 03 | None. | — |
| 10 | item | 03 | None upstream; clickable through `.sense(Sense)`; `HOVER`/`FOCUS_VISIBLE` only when sensed. | CP-03-13 |
| 11 | empty | 03 | None (`.outline(bool)` is a variant flag). | CP-03-35 |
| 12 | aspect-ratio | 03 | None. | CP-03-31 |
| 13 | alert | 04 | `variant` only; no `enabled`, no `invalid`. | CP-04-6 |
| 14 | progress | 04 | None. | CP-04-13 |
| 15 | separator | 04 | None; `decorative` affects accessibility only. | CP-04-19 |
| 16 | avatar | 04 | None; image loading status internal. | CP-04-25 |
| 17 | checkbox | 05 | `.enabled(bool)` kept, `.invalid(bool)`, `.indeterminate(bool)`. | CP-05-1, CP-05-3 |
| 18 | radio-group | 05 | Group `.enabled`, `.invalid`; item `.invalid`; item disabled via `ui.add_enabled`. | CP-05-14, CP-05-15 |
| 19 | switch | 05 | `.enabled(bool)`, `.invalid(bool)`. | CP-05-23, (CP-05-3) |
| 20 | slider | 05 | `.enabled(bool)`; no `invalid` (no style rule). | (CP-05-3) |
| 21 | input | 06 | Disabled via `ui.add_enabled`; `.invalid(bool)`, `.password(bool)`. | CP-06-1, CP-06-7 |
| 22 | textarea | 06 | Disabled via `Ui`; `.invalid(bool)`. | (CP-06-1, CP-06-38) |
| 23 | input-otp | 06 | Disabled via `Ui`; `.invalid(bool)` for whole widget (all slots + group). | CP-06-17, CP-06-18, CP-06-20 |
| 24 | input-group | 06 | Disabled via `Ui` (fades addons); invalid read from the inner control. | CP-06-30, CP-06-34, CP-06-38 |
| 25 | label | 07 | `.enabled(bool)` kept as "peer disabled" look, plus `Ui`; no `invalid` (comes from Field). | CP-07-1 |
| 26 | field | 07 | `invalid`, `enabled` (disables control `Ui`), FieldCard `selected`; errors imply invalid. | CP-07-8, CP-07-9, CP-07-10, CP-07-16, CP-07-18, CP-07-30 |
| 27 | form | 07 | Errors → invalid, folded into Field; `required` not offered. | CP-07-21 |
| 28 | native-select | 07 | `.enabled(bool)` kept, `.invalid(bool)`, `size`; per-option disabled via `add_enabled_ui`. | CP-07-27, CP-07-31 |
| 29 | select | 08 | `.enabled(bool)` kept, `.invalid(bool)`; per item `SelectItem::disabled(bool)`. | CP-08-12 |
| 30 | command | 08 | Per item `disabled(bool)`, `checked(bool)`; root none; input disabled only via `Ui`. | CP-08-32, CP-08-34, CP-08-37 |
| 31 | dropdown-menu | 09 | Item `.enabled(bool)` (and `Ui`), `.variant(Destructive)`, `.inset`, `.close_on_select`, `.indeterminate`; no `invalid`. | CP-09-9, CP-09-14, CP-09-18, CP-09-36 |
| 32 | context-menu | 09 | As dropdown-menu minus `indeterminate`; disabled target follows the target `Response`. | (CP-09-18, CP-09-36) |
| 33 | menubar | 09 | `MenubarMenu::enabled(bool)`; item flags as dropdown-menu. | CP-09-32, CP-09-33, (CP-09-18, CP-09-36) |
| 34 | popover | 10 | None on the popover; disabled lives on the trigger widget. | CP-10-6 |
| 35 | hover-card | 10 | `HoverCard::enabled(bool)` (bits root `disabled`). | CP-10-16, CP-10-22 |
| 36 | tooltip | 10 | None; shows for disabled triggers too. | CP-10-10, CP-10-11 |
| 37 | dialog | 11 | None (`enabled` n/a); `show_close_button(bool)`. | CP-11-7, CP-11-36 |
| 38 | alert-dialog | 11 | None. | CP-11-16, CP-11-38 |
| 39 | sheet | 11 | None. | CP-11-37, (CP-11-36) |
| 40 | drawer | 11 | `modal(bool)`, `dismissible(bool)`. | CP-11-31, CP-11-33, (CP-11-36) |
| 41 | tabs | 12 | Per trigger `tab_enabled(bool, v, atoms)`; whole list via `ui.add_enabled` (drop `.enabled`). | CP-12-6, CP-12-7, CP-12-12, CP-12-31 |
| 42 | accordion | 12 | `AccordionItem::enabled(bool)`; whole accordion via `ui.add_enabled_ui`. | CP-12-22, CP-12-25, CP-12-32, CP-12-34 |
| 43 | collapsible | 12 | Disabled through the trigger widget; helper ignores clicks on a disabled response. | — |
| 44 | navigation-menu | 13 | Per item via `add_enabled_ui` around `nav.item`; links `.active(bool)`; no `invalid`. | CP-13-8, CP-13-12, CP-13-34, CP-13-36 |
| 45 | breadcrumb | 13 | None upstream; `enabled` through `ui.add_enabled_ui`. | — |
| 46 | pagination | 13 | Disabled via `ui.add_enabled`; Previous/Next disable themselves at the ends. | CP-13-28 |
| 47 | sidebar | 14 | `MenuButton::active(bool)`; disabled via `ui.add_enabled`; collapsed/variant/side from context. | CP-14-16, CP-14-20, CP-14-24, CP-14-44, CP-14-48 |
| 48 | scroll-area | 14 | None (no disabled upstream). | CP-14-29, (CP-14-44) |
| 49 | resizable | 14 | Per-pane `collapsible`; disabled via `ui.add_enabled_ui`. | CP-14-37, CP-14-39, CP-14-42, (CP-14-44) |
| 50 | calendar | 15 | `.enabled(bool)` (also honours `ui.is_enabled()`), `.read_only(bool)`; no `invalid`. | CP-15-17, CP-15-18, CP-15-19, CP-15-20, CP-15-27, CP-15-28, CP-15-37, CP-15-38 |
| 51 | range-calendar | 15 | As calendar, plus `min_days`, `max_days`, `exclude_disabled`. | CP-15-32, CP-15-34, CP-15-35 |
| 52 | table | 16 | Per row `selected`, `expanded` (positional bools); disabled via `ui.add_enabled_ui`. | CP-16-9, CP-16-39 |
| 53 | carousel | 16 | `looping(bool)`, `buttons(bool)`; disabled via `ui.add_enabled_ui`. | CP-16-22, CP-16-42 |
| 54 | sonner | 16 | Toast `dismissible`, `close_button`, `important`; toaster `expand`, `close_button`, `rich_colors`; no `enabled`. | CP-16-33, CP-16-34, CP-16-43 |
| 55 | chart | 17 | None upstream; legend item `selected`. | CP-17-13, CP-17-15 |
| 56 | attachment | 18 | `AttachmentState` enum; enabled via the `Ui`; no `invalid`. | CP-18-4, CP-18-7, CP-18-8, (CP-18-30, CP-18-34, CP-18-37) |
| 57 | bubble | 18 | `.sense` (clickable); enabled via the `Ui`; no `invalid`. | CP-18-19, CP-18-22, CP-18-36, (CP-18-37) |
| 58 | marker | 18 | `.link(bool)`, `.sense`, `.shimmer(bool)`, `.status(bool)`; enabled via the `Ui`. | CP-18-25, CP-18-27, CP-18-29, CP-18-30, CP-18-31, CP-18-34, CP-18-37 |
| 59 | message | 19 | None upstream; `ghost(bool)` set by caller, mirrors the inner Bubble. | CP-19-5, CP-19-37 |
| 60 | message-scroller | 19 | `auto_scroll(bool)`, `preserve_on_prepend(bool)`; button active is derived. | CP-19-16, CP-19-18, CP-19-20, CP-19-40 |
| 61 | questionnaire | 19 | Question `required`, `multiple`, `disabled`, `error`; `Choice::disabled`; whole form via `Ui`. | CP-19-27, CP-19-30, CP-19-31, CP-19-34, CP-19-35, CP-19-38, CP-19-41 |

### T10 · Shortcut function

| Component | Report | Answer (shortcut, return) | Topic-10 CPs |
|---|---|---|---|
| button | 01 | `button(ui, text) -> Response`, new | CP-01-4 |
| button-group | 01 | none for root; `button_group_separator(ui)` `()`, `button_group_text(ui, text) -> Response` | — |
| toggle | 01 | `toggle_button(ui, &mut bool, text) -> Response`; name open | CP-01-15 |
| toggle-group | 01 | none, or `toggle_group(ui, &mut Option<T>, &[(T, &str)]) -> Response` | — |
| badge | 02 | `badge(ui, text) -> Response`; drops today's `variant` argument | CP-02-6 |
| kbd | 02 | `kbd(ui, text) -> Response`, `kbd_shortcut(ui, &KeyboardShortcut) -> Response` | CP-02-11 |
| spinner | 02 | `spinner(ui) -> Response` | CP-02-22 |
| skeleton | 02 | `skeleton(ui, size) -> Response` | — |
| card | 03 | none; `layout::card` removed | CP-03-8 |
| item | 03 | none; part helpers `item_title`/`item_description`/`item_separator -> Response` | CP-03-19 |
| empty | 03 | none | — |
| aspect-ratio | 03 | none, or `layout::aspect_ratio(ui, ratio, add) -> R` | CP-03-29 (5/10) |
| alert | 04 | `alert(ui, title, description) -> Response`, default variant | CP-04-8 |
| progress | 04 | `progress(ui, fraction) -> Response` | CP-04-14 |
| separator | 04 | `separator(ui)`, `separator_vertical(ui) -> Response` (today `()`) | CP-04-16 |
| avatar | 04 | `avatar(ui, fallback) -> Response` or none; sketch none | CP-04-30 |
| checkbox | 05 | `checkbox(ui, &mut bool) -> Response` (today), no label | CP-05-5 |
| radio-group | 05 | `radio_value(ui, &mut V, V) -> Response`, no label | CP-05-19 |
| switch | 05 | `toggle(ui, &mut bool) -> Response` (today) | CP-05-22 |
| slider | 05 | `slider(ui, &mut f32, range) -> Response`, new | CP-05-38 |
| input | 06 | `input(ui, &mut String) -> Response`, new | CP-06-5 |
| textarea | 06 | `textarea(ui, &mut String) -> Response`; not in sketch, no CP | — |
| input-otp | 06 | none | — |
| input-group | 06 | none | — |
| label | 07 | `label(ui, text) -> Response` (today) | — |
| field | 07 | part helpers `field::description/error/separator`; `field(ui, label, widget)` open | CP-07-13 |
| form | 07 | n/a | — |
| native-select | 07 | `native_select(ui, id_salt, &mut T, &[(T, &str)]) -> Response` | — |
| select | 08 | `select(ui, id_salt, &mut usize, &[&str]) -> Response` (today `-> bool`) | CP-08-13 (anchor) |
| command | 08 | none; possible `command_palette(ctx, &mut open, &[&str]) -> Option<usize>` | CP-08-35 |
| dropdown-menu | 09 | `dropdown_menu(ui, trigger: Button, add) -> InnerResponse<Option<R>>`; `menu_label`, `menu_separator` `()` | — |
| context-menu | 09 | `context_menu(&Response, add) -> Option<R>` | — |
| menubar | 09 | `menubar_menu(ui, &str, add) -> InnerResponse<Option<R>>` | — |
| popover | 10 | none (`popover(&trigger, ..)` rejected) | CP-10-21 |
| hover-card | 10 | none, or `Response` extension `on_hover_card` (CP-10-14, tagged 5) | — |
| tooltip | 10 | `tooltip(resp: Response, text) -> Response` (today) plus builder | CP-10-8 |
| dialog | 11 | none; builder is the entry point | — |
| alert-dialog | 11 | part helpers `alert_dialog::cancel/action(ui, IntoAtoms) -> Response` | — |
| sheet | 11 | none | — |
| drawer | 11 | none | — |
| tabs | 12 | `tabs(ui, &mut V, &[(V, &str)]) -> Response`, replaces `tab_bar -> bool` | CP-12-10 |
| accordion | 12 | none; n/a | — |
| collapsible | 12 | `collapsible_trigger(Response, &mut bool) -> Response` | CP-12-29 |
| navigation-menu | 13 | none; a container like `MenuBar` | — |
| breadcrumb | 13 | `breadcrumb(ui, &[&str]) -> Option<usize>` (CP-13-18, tagged 1) | — |
| pagination | 13 | `pagination(ui, &mut page, total_pages) -> bool`; follows CP-08-13 when decided | CP-13-23 |
| sidebar | 14 | `sidebar::trigger(ui, id) -> Response`, `sidebar::menu_value(..) -> Response` | CP-14-18 |
| scroll-area | 14 | none; `ScrollArea::vertical()` already short | — |
| resizable | 14 | none | — |
| calendar | 15 | `calendar(ui, &mut Option<Date>) -> bool`; follows CP-08-13 | CP-15-21 |
| range-calendar | 15 | `range_calendar(ui, &mut DateRange) -> bool`; follows CP-08-13 | CP-15-33 |
| table | 16 | `table(ui, headers: &[&str], rows: &[Vec<String>]) -> Response` | CP-16-11, CP-16-39 |
| carousel | 16 | none; builder already short | — |
| sonner | 16 | `toast(ctx, title) -> ToastId`, `success/info/warning/error/loading(ctx, title) -> ToastId` | — |
| chart | 17 | none; maybe `chart_tooltip(resp, label, rows) -> Response` | CP-17-11 |
| attachment | 18 | only `impl Widget`; `attachment_action -> Response`, `attachment_group -> ScrollAreaOutput<R>` | CP-18-12 |
| bubble | 18 | `bubble(ui, text, variant) -> Response`; `bubble_group(ui, add) -> InnerResponse<R>` | — |
| marker | 18 | `marker(ui, text) -> Response` | CP-18-32 |
| message | 19 | `message_group(ui, add) -> InnerResponse<R>`; none for one message | — |
| message-scroller | 19 | none; builder is one call | CP-19-15 (mis-tagged) |
| questionnaire | 19 | none; builder is the call | CP-19-32 (mis-tagged) |

### T11 · Cross-cutting conventions

Notation, in this order where present:
- **B** boolean setters: `n` no argument (`.vertical()`), `b` takes a
  `bool`, `pos` positional `bool` parameter; flags named in brackets.
- **I** icon parameter: `v` `Icon` by value, `&'a` borrowed, `&` reference
  copied, `G` `impl Into<Glyph>`.
- **T** text parameters: `W` `impl Into<WidgetText>`, `S` `impl
  Into<String>`, `s` `&str`, `A` `impl IntoAtoms`.
- **K** constructors: `new` / named ones; `id` = `id_salt` required in
  the constructor, `id?` = optional `.id_salt()` setter.
- **M** `#[must_use]` on the main builder (`M+` / `M−`); **In**
  `#[inline]` on setters (`In+` / `In−`).
- **Bind** binding setter; **F** new `State` flags; **Sl** icon slots;
  **X** name clash.

| Component | Report | Answer | CPs |
|---|---|---|---|
| button | 01 | B b(expanded, invalid, has_popup) · I v · T S · K new · M+ In+ | CP-01-2, CP-01-3, CP-01-32 |
| button-group | 01 | B n(vertical) · T S (`button_group_text`) · K new, id? · M+ In+ | CP-01-31, CP-01-9, CP-01-11 |
| toggle | 01 | B n(icon_only), b(invalid) · I v · T S · M+ In+ · F PRESSED · X `toggle` → `toggle_button` | CP-01-15, CP-01-16, CP-01-17, CP-01-31 |
| toggle-group | 01 | B n(vertical, icon_only), item b(enabled) · I v · T S · K single/multiple · item M− · In+ | CP-01-24, CP-01-30, CP-01-31 |
| badge | 02 | B b(link, invalid) · I G (by value inside) · T W · K new · M+ In+ | CP-02-1, CP-02-2, CP-02-8, CP-02-28, CP-02-29 |
| kbd | 02 | T W · I v · K `new(text)` plus `Kbd::icon(Icon)` content constructor · M+ | CP-02-14 |
| spinner | 02 | T W (label) · colour `impl Into<Color32>` · derive Default · M+ In+ · X `spinner(ui)` vs `ui.spinner()` | CP-02-18, CP-02-22 |
| skeleton | 02 | B n(circle) · K `new(impl Into<Vec2>)` · M+ In+ | CP-02-25, CP-02-29 |
| card | 03 | T W · K new · M+ (show) In− · free `card_title`/`card_description` | CP-03-9 |
| item | 03 | T W · I &'a · K new(title) · `.sense(Sense)` · M+ In− · `ItemGroup::new` | CP-03-13, CP-03-14, CP-03-34 |
| empty | 03 | B b(outline) · I &'a · T W · K new(title) · M+ In− | CP-03-34, CP-03-35 |
| aspect-ratio | 03 | K new(ratio) · M+ In− · no flags, no text | CP-03-29 |
| alert | 04 | I &'a · T W · K new(title) · M+ In− · `AlertVariant` no Default | CP-04-3, CP-04-31 |
| progress | 04 | K new(fraction) + `from_value` · M+ In− | CP-04-9 |
| separator | 04 | B n(horizontal, vertical) · K new, derive Default · M+ In− | CP-04-20, CP-04-33 |
| avatar | 04 | I &'a · T W (fallback, count), S (alt) · K new · `AvatarBadge` M− · `AvatarSize` no Default | CP-04-32 |
| checkbox | 05 | B b(indeterminate, invalid, enabled) · K new(&mut bool) · M+ In+ · Sl minus | CP-05-3, CP-05-6 |
| radio-group | 05 | B b(invalid, enabled) · K `RadioGroup::new(id_salt)` id · M+ In+ · dot painted | CP-05-12, CP-05-18, CP-05-19 |
| switch | 05 | B b(invalid, enabled) · K new(&mut bool) · M+ In+ · X shortcut `toggle` | CP-05-22 |
| slider | 05 | B n(vertical), b(enabled) · K new + `multiple` · M+ In+ | CP-05-25, CP-05-37 |
| input | 06 | B b(password, invalid) · T A (`hint_text`) · K new, id? · M+ In− | CP-06-2, CP-06-38 |
| textarea | 06 | B b(invalid) · T A (`hint_text`) · K new, id? · M+ In− | CP-06-9, CP-06-38 |
| input-otp | 06 | B b(invalid) · K new(&mut String, len), id? · M+ In− · Sl minus | CP-06-37, CP-06-38 |
| input-group | 06 | I v (`InputGroupButton::icon` constructor) · T S · `InputGroupButton` M− · In− | CP-06-27, CP-06-36 |
| label | 07 | B b(enabled) · T W · K new · M+ In− | CP-07-1, CP-07-4 |
| field | 07 | B n(horizontal, control_first, tight), b(invalid, enabled) · T W, S (error), s (separator) · M+ In− · X `field::separator`, `field::description` | CP-07-29, CP-07-30 |
| form | 07 | n/a (folded into Field) | — |
| native-select | 07 | B b(invalid, enabled) · T W, A (`option`), s (shortcut) · K `from_id_salt` id · M+ (show_ui) In− | CP-07-31, CP-07-32 |
| select | 08 | B b(invalid, enabled, group_separators), item b(disabled) · I v · T s (items), W (placeholder) · K new(id_salt,…) id + `grouped` · item M− | CP-08-10, CP-08-14, CP-08-32, CP-08-36 |
| command | 08 | B b(should_filter, loop_navigation, vim_bindings), item b(checked, disabled) · I v · T s, W · K new(id_salt) id · `CommandDialog` M− | CP-08-28, CP-08-32, CP-08-36, CP-08-37 |
| dropdown-menu | 09 | B b(inset, enabled, close_on_select, indeterminate) · I &'a · T S, s (`menu_label`) · K new(&Response) · sub-items M− · Sl minus | CP-09-7, CP-09-35, CP-09-36 |
| context-menu | 09 | shared items as dropdown-menu · K new(&Response) · X `context_menu` vs egui `Response::context_menu` | CP-09-23, CP-09-35, CP-09-36 |
| menubar | 09 | B b(enabled) · T S, s (`menubar_menu`) · K new, id? · M+ In− | CP-09-28, CP-09-30, CP-09-36 |
| popover | 10 | Bind `open_bool` · T W · K new(&Response), id? · M+ In− | CP-10-2, CP-10-4, CP-10-17 |
| hover-card | 10 | B b(enabled) · K new(&Response) · M+ In− | CP-10-14 |
| tooltip | 10 | T W · K new(&Response) · M+ In− · X `Tooltip` = egui `Tooltip` | CP-10-8, CP-10-26 |
| dialog | 11 | B b(show_close_button) · T W · K new(id_salt, title) id · M+ In− · free `dialog::footer` | CP-11-4, CP-11-8, CP-11-36 |
| alert-dialog | 11 | I v (media) · T W, A (`cancel`/`action`) · K new(id_salt, title) id · F HAS_MEDIA | CP-11-14, CP-11-38 |
| sheet | 11 | B b(show_close_button) · T W · K new(id_salt, title) id · enum `Side` · F SIDE_×4 | CP-11-19, CP-11-36, CP-11-37 |
| drawer | 11 | B b(modal, dismissible) · T W · K new(id_salt, title) id · F SIDE_×4 | CP-11-36, CP-11-37 |
| tabs | 12 | B n(vertical), pos `tab_enabled(bool, v, atoms)` · T A · K new(&mut V), id? · M+ In− | CP-12-3, CP-12-6, CP-12-31, CP-12-32 |
| accordion | 12 | B item b(enabled) · T W · K single/multiple, id? · `Accordion` M− · Sl chevron_up/down · F OPEN | CP-12-16, CP-12-18, CP-12-21, CP-12-33, CP-12-34 |
| collapsible | 12 | free `collapsible_trigger(Response, &mut bool)` · X `Collapsible` vs sidebar enum | CP-12-27, CP-12-28, CP-12-29 |
| navigation-menu | 13 | B b(viewport, active) · T A, W (description) · K new(id_salt) id · Sl nav chevron · F 2 new + EXPANDED | CP-13-8, CP-13-31, CP-13-34, CP-13-36 |
| breadcrumb | 13 | I &'a (separator) · T W · K new · Sl chevron_right, more_horizontal | CP-13-20, CP-13-31, CP-13-33 |
| pagination | 13 | B b(numbers, prev_next, prev_next_text, centered) · K new + `from_count`, id? · Sl chevron_left/right | CP-13-22, CP-13-27, CP-13-31, CP-13-35 |
| sidebar | 14 | B b(rail, active), pos (action, menu_skeleton) · I v · T S · K left/right(`Into<Id>`) · Bind `open` · In+ · F ~10 · Sl sidebar · X `Sidebar::toggle`, `sidebar::separator`/`input` | CP-14-13, CP-14-22, CP-14-43, CP-14-44, CP-14-45, CP-14-46, CP-14-47, CP-14-48 |
| scroll-area | 14 | B b(stick_to_bottom) · K vertical/horizontal/both, id? · M+ In+ · X `ScrollArea` = egui `ScrollArea` | CP-14-26, CP-14-44 |
| resizable | 14 | B b(with_handle) · K horizontal/vertical(id_salt, …) id · Bind `layout` · `Pane` M− · In+ | CP-14-36, CP-14-43, CP-14-44 |
| calendar | 15 | B b(7 options, enabled) · T W (day_note) · K new/multiple/range, id? · Bind `view_month` · In− · F ~8 · Sl chevron_left/right/down_plain | CP-15-2, CP-15-6, CP-15-12, CP-15-17, CP-15-36, CP-15-37, CP-15-38 |
| range-calendar | 15 | `Calendar::range` constructor · B b(exclude_disabled) · range-only setters on shared type | CP-15-31, CP-15-37 |
| table | 16 | B n(align_end), pos `row_with(selected, expanded)` · T W · K new(id_salt) id, `Column::auto/exact/remainder` · `Table` M− | CP-16-5, CP-16-39 |
| carousel | 16 | B n(vertical), b(looping, buttons) · K new(id_salt, &mut usize, count) id · M− · Sl chevron_left/right | CP-16-40, CP-16-41, CP-16-42 |
| sonner | 16 | B b(expand, close_button, dismissible) · T S · `Toast`/`Toaster` M− · X `error`, `info`, `toast` shortcuts · Sl 5 kinds | CP-16-36, CP-16-38, CP-16-43, CP-16-44 |
| chart | 17 | B n(hide_label, hide_indicator) · I v (`Option<Icon>` field) · T W, String · `ChartLegend` M− | CP-17-11, CP-17-15 |
| attachment | 18 | B n(vertical, spinner) · I &'a, `attachment_action(&Icon, &str)` · T W, s · K new, id? · F cluster ~12 | CP-18-5, CP-18-30, CP-18-33, CP-18-34, CP-18-35 |
| bubble | 18 | T W · K new · M+ In− · shortcut `bubble(ui, text, variant)` positional variant | CP-18-14, CP-18-15 |
| marker | 18 | B n(spinner), b(link, shimmer, status) · I &'a · T W · K new · M+ In− | CP-18-33, CP-18-34, CP-18-35 |
| message | 19 | B n(end, avatar_space), b(ghost) · T W · K new · M+ In− · F ALIGN_END, GHOST | CP-19-8, CP-19-37, CP-19-41 |
| message-scroller | 19 | B b(auto_scroll, preserve_on_prepend), pos (item anchor) · K new(id_salt) id · Sl arrow_down · F 2 | CP-19-37, CP-19-39, CP-19-40 |
| questionnaire | 19 | B b(multiple, required, disabled, progress) · T W, S (freeform) · K new, id? · `Choice`/`Question` M− · Sl check (reuse) · F 3 | CP-19-36, CP-19-37, CP-19-38, CP-19-41, CP-19-42 |

Rows: 61.

## Choice points

Every `CP-` ID of the 19 reports and every `CP-T` ID of the 11 cross-checks, grouped by the topic its question belongs to and, inside a topic, by the groups of the cross-check. Each line: ID (component) · question · options · sketch pick. A CP tagged with another topic in its report says so in italics; mis-tag groups of the cross-checks name where their CPs went. Where reports ask the same question under different IDs, the IDs stand side by side in one group.

Open review notes that touch CPs, kept for stage 2: batch-B N1 (CP-12-28 names the old enum, now CP-14-46), N2 (CP-14-12 also depends on CP-11-35), N3 (CP-11-19 lists the sidebar as a `Side` user while CP-14-13 picks constructors), N4 (CP-14-43 and CP-14-48 have no sketch pick), N5 (three spellings of the plain down-chevron slot, CP-T11-13), N6 (CP-18-30 also referenced by 14 and 16); batch-B Y12–Y16 (`data-open`, animation, slot content, batch-A components treated as absent, index base) map to CP-T11-14, CP-T3-10 / CP-T9-14, CP-T4-13 / G3 of T11, CP-T4-12 / CP-T5-10, CP-T1-10. CP-02-15, CP-02-21, CP-09-24 and CP-15-23…26 appear in no cross-check group; they are placed by their question and marked.

### T1 · Selection

**G1 · Value type of a choice (generic, index, domain type)**

- `CP-01-20` (toggle-group) · Value type: generic `T: PartialEq` (egui `selectable_value`) / index `usize` like `Tabs` and `Select` (`tabs.rs:52`, `select.rs:64`) / `&mut [bool]` for multiple · sketch: generic `T`
- `CP-05-9` (radio-group) · Value type? · generic `V: PartialEq + Clone` / `usize` index (`Tabs`, `Select`) / `String` (upstream) · generic `V`.
- `CP-08-1` (select) · value type · `&mut usize` index (today; `ComboBox::show_index` precedent) / generic `&mut T: PartialEq + Clone` / `&mut Option<T>` · sketch: generic `&mut T`, no-match shows the placeholder.
- `CP-12-1` (tabs) · selection value type · `&mut usize` index (today) / generic `&mut V: PartialEq` (like `selectable_value`) / `&mut Option<V>` (upstream `""` = nothing selected) · generic `&mut V`.
- `CP-19-22` (questionnaire) · choice value type · index `usize` (like `ComboBox::show_index`) / generic `T: PartialEq + Clone` per choice / `String` value as upstream form data · index
- `CP-15-1` (calendar) · date type · crate-owned `Date {year,month,day}` (no dependency, own arithmetic) / `jiff::civil::Date` (what `egui_extras` datepicker uses; optional feature) / `chrono::NaiveDate` / `time::Date` · pick: crate-owned `Date`, optional `jiff` feature for `From`/`Into`.
- `CP-05-26` (slider) · Value type? · `f32` (today) / generic `emath::Numeric` like `egui::Slider` (stored as f64 behind a closure, no type parameter) · `f32` now; generic stays possible without a break for `f32` callers.
- `CP-06-13` (input-otp) · Value type · `&mut String` (bits) / `&mut Vec<Option<char>>` / `&mut [char; N]` · pick: `&mut String`.
- `CP-17-3` (chart) · Data model · `ChartConfig` + caller passes `Vec<f64>` per series / generic `T` rows with accessor closures (layerchart `x="month"`) / egui_plot items directly · sketch: egui_plot items plus `ChartConfig` for colours and labels.
- `CP-04-9` (progress) · Value input · fraction `0..=1` (egui `ProgressBar::new`) vs `value` + `max` (+`min`) (shadcn/bits-ui) · sketch: `new(fraction)` plus `from_value(value, max)`.

**G2 · How "no selection" is expressed in single mode**
See also (placed in another group): CP-08-1, CP-12-1.

- `CP-01-21` (toggle-group) · Single-select value: `&mut Option<T>` (upstream allows clearing) / `&mut T` with no deselect (radio-like) / `&mut T` plus `.allow_empty` · sketch: `&mut Option<T>`
- `CP-05-10` (radio-group) · Empty selection? · built-in `&mut Option<V>` / caller uses `V = Option<T>` · caller's `Option<T>`.
- `CP-04-10` (progress) · Indeterminate (`value: null`) · `Option<f32>` drawn empty like shadcn; an animated indeterminate bar (not upstream); not offered · sketch: not offered.
- `CP-15-30` (range-calendar) · range value · `&mut DateRange` with optional ends (caller sees the half-picked start) / `&mut Option<(Date, Date)>` with the pending start in memory (bits-ui `value` semantics) · pick: `DateRange`.

**G3 · Deselect by a second click**
See also (placed in another group): CP-01-21.

- `CP-08-5` (select) · deselect (`allowDeselect`, default false) · omit / `.allow_deselect()` for `Option<T>` only · sketch: omit.
- `CP-15-4` (calendar) · second click on the selected date · clears it (bits-ui default) / keeps it · pick: clears, `prevent_deselect(true)` keeps.

**G4 · Multiple-mode container**

- `CP-01-22` (toggle-group) · Multiple-select value: `&mut Vec<T>` (upstream array, append order) / `&mut HashSet<T>` / `&mut [bool]` per item · sketch: `&mut Vec<T>`
- `CP-12-15` (accordion) · multiple-mode container · `&mut Vec<V>` / `&mut HashSet<V>` (needs `Hash + Eq`) / `&mut [bool]` by index · `&mut Vec<V>`.
- `CP-17-8` (chart) · Legend selection storage · caller `&mut HashSet<key>` / `&mut [bool]` by series index / egui memory · sketch: caller `&mut HashSet`.
- `CP-09-11` (dropdown-menu) · CheckboxGroup (`string[]`, dropdown only) · port as `MenuCheckboxItem::in_set(&mut HashSet<T>, value)` / not ported (caller keeps bools) · not ported.
- `CP-19-23` (questionnaire) · answer storage · one `Answer { selected, text, status }` per question in the state / caller's own typed fields bound per question (`&mut Option<usize>`, `&mut Vec<usize>`, `&mut String`) · state struct
- `CP-05-25` (slider) · Several thumbs? · `Slider::multiple(&mut [f32], range)` on the same type / a separate `RangeSlider` type / not offered · `multiple` on the same type, later.
- `CP-08-4` (select) · multiple · leave out / `Select::multiple(id, &mut Vec<T>, items)` / own `MultiSelect` type · sketch: leave out (no shadcn demo uses `type="multiple"`), shape keeps room.

**G5 · How single vs multiple (vs range) is spelled**
See also (placed in another group): CP-08-4, CP-05-25.

- `CP-01-24` (toggle-group) · Single vs multiple: two constructors `single`/`multiple` / two types `ToggleGroup` and `ToggleGroupMulti` / one type with an enum · sketch: two constructors
- `CP-12-16` (accordion) · single vs multiple as · two constructors `single`/`multiple` / one constructor + `.multiple(bool)` / two types · two constructors.
- `CP-15-2` (calendar) · selection modes · one `Calendar` with `new`/`multiple`/`range` constructors / separate `Calendar`, `MultiCalendar`, `RangeCalendar` structs / single only (no shadcn demo uses multiple) · pick: one type, three constructors.
- `CP-15-31` (range-calendar) · range-only options on the shared type · setters ignored outside range mode / type-state (`Calendar<Range>`) / separate `RangeCalendar` struct · pick: setters ignored outside range mode.

**G6 · How items are supplied and where the binding lives**
See also (placed in another group): CP-17-3, CP-01-23, CP-05-8, CP-05-11, CP-05-20, CP-12-2, CP-16-14, CP-19-12, CP-19-21.

- `CP-08-2` (select) · how items are supplied · `&[&str]` (today) / data slice of `SelectItem<T>` / collector closure (`|list| list.item(v, "A")`, run every frame, paints nothing) / ComboBox-style `show_ui` closure + caller `selected_text` · sketch: data slice.
- `CP-07-24` (native-select) · how native-select items and values are supplied · `ComboBox`-style `show_ui` closure with `&mut T` / Select's `&mut usize` + `&[&str]` / both · sketch: both (closure builder, slice shortcut); conflicts with Select today
- `CP-09-13` (dropdown-menu) · Radio items · `MenuRadioItem::new(&mut cur, v, text)` (`radio_value` form, no group struct) / a `menu_radio_group(ui, &mut v, |g| ..)` container (bits RadioGroup) · `_value` form.
- `CP-14-8` (sidebar) · Active item idiom · (a) `.active(bool)` + `clicked()` (b) `menu_value(&mut V, V, ..)` that marks changed (c) both · sketch: (c)
- `CP-16-5` (table) · Row selected/expanded flags · (a) `row_with(selected, expanded, ..)`; (b) chained on a returned row builder; (c) not offered, only hover · (a)

**G7 · Item content / label type**
See also (placed in another group): CP-08-14, CP-09-7, CP-07-4, CP-06-2, CP-12-3, CP-01-2, CP-05-20.


**G8 · Item icon parameter form (follows CP-01-3)**
See also (placed in another group): CP-01-3, CP-08-36, CP-09-35.


**G9 · What the call returns**
See also (placed in another group): CP-08-13, CP-05-5, CP-12-10, CP-13-23, CP-15-21, CP-15-33.

- `CP-08-6` (select) · what the call returns · `Response` + `changed()` / `InnerResponse<Option<T>>` · sketch: `Response`.
- `CP-15-3` (calendar) · return value · `Response` with `changed()` / `Option<Date>` newly picked / `InnerResponse` · pick: `Response`.
- `CP-08-22` (command) · how activation is reported · per-item `Response::clicked()` (Enter synthesises it) / `show` returns `Option<value>` of the activated item · sketch: per-item `Response`.
- `CP-13-18` (breadcrumb) · breadcrumb shortcut return · `Option<usize>` clicked index / `bool` / `Response` (CP-08-13) · sketch: `Option<usize>`, an exception to CP-08-13

**G10 · Who owns the selected value**
See also (placed in another group): CP-19-23, CP-12-14, CP-19-24.

- `CP-16-13` (carousel) · Who owns the slide index · (a) caller `&mut usize`; (b) egui memory with a `CarouselState::load/store`; (c) both (`.index(&mut)` optional) · (a)

**G11 · Index base**

- `CP-13-21` (pagination) · Page base · 1-based (bits-ui, matches the label) / 0-based (Rust index, like `Tabs` `&mut usize`) · 1-based.
- `CP-13-22` (pagination) · Constructor input · `total_pages` / bits-style `count` + `per_page` (shadcn default `perPage = 10`, bits default 1) / both · both (`new` and `from_count`); `from_count` has no default `per_page`.

**G12 · Checkbox-like values (`&mut bool`, indeterminate)**

- `CP-05-7` (checkbox) · `checked && indeterminate` both true? · indeterminate wins (bits-ui `data-state`, `aria-checked="mixed"`) / tick wins (shadcn snippet) · indeterminate wins (matches the accessibility value).
- `CP-09-10` (dropdown-menu) · Checkbox item value · `&mut bool` marking `changed()` (crate Checkbox, egui `checkbox`) / `bool` + `clicked()` · `&mut bool`.
- `CP-09-12` (dropdown-menu) · Indeterminate checkbox item · display flag `.indeterminate(bool)` (caller clears on `changed()`) / `&mut bool` second binding (bits) / not supported (context-menu and menubar files ignore it) · display flag.
- `CP-09-26` (context-menu) · Indeterminate on context-menu checkbox items · not offered (upstream file ignores it) / offered as in dropdown-menu (shared widget) · offered, since the widget is shared (CP-09-2); upstream disagreement.

**G13 · Selection keyboard behaviour (tagged 1, behaviour questions)**

- `CP-05-4` (checkbox) · Does Enter toggle? · yes (egui's fake click) / no, Space only (bits-ui) · Space only.
- `CP-05-16` (radio-group) · Arrow keys when no item matches the value? · move focus only (bits-ui `hasValue`) / move and select · move focus only.
- `CP-05-17` (radio-group) · Wrap at the ends? · always (shadcn never sets `loop`) / `.wrap(bool)` option · always.
- `CP-12-8` (tabs) · activation mode · automatic only (today) / `.activation(Manual|Automatic)` · automatic only (crate focus only lands on the selected tab anyway).
- `CP-12-9` (tabs) · keyboard wrap (`loop`) · always wrap (today; bits component default) / `.wrap(bool)` · always wrap.
- `CP-12-26` (accordion) · focus model · every trigger a Tab stop plus arrow navigation (bits) / roving single Tab stop (like tabs) · bits model (every trigger a Tab stop).
- `CP-13-30` (pagination) · Keyboard: copy bits (all Tab stops, arrows move focus) / crate group rule (one Tab stop, arrows move focus, SKILL.md "roving focus") · copy bits: it is not a selection group, and arrows do not change the page.
- `CP-08-16` (select) · typeahead on the closed trigger · selects directly without opening (bits `:829`) / opens first / nothing (today) · sketch: bits.
- `CP-08-17` (select) · typeahead buffer · 1 s accumulated string (bits) / first letter cycling (today) · sketch: bits.
- `CP-08-31` (command) · keyboard extras · full bits set (vim Ctrl+N/J/P/K default on, Alt group jump, Meta first/last) / arrows, Home/End, Enter only · sketch: full set, `.vim_bindings(false)` to turn off.
- `CP-05-28` (slider) · PageUp/PageDown? · keep ten steps (today, WAI-ARIA slider pattern) / drop (bits-ui 2.19.2 has none) · keep, comment no longer credited to bits-ui.
- `CP-05-29` (slider) · Left/Right on a vertical slider? · accepted (today) / ignored (bits-ui `:883,894`) · ignored.
- `CP-05-30` (slider) · Meta+Arrow → ends? · add (bits-ui) / omit · add.
- `CP-06-12` (textarea) · Tab key · moves focus (browser) / inserts `\t` (`lock_focus(true)`) / option · pick: moves focus, no option.

**G14 · Value semantics inside one component (no cross-component counterpart)**
See also (placed in another group): CP-13-9.

- `CP-05-27` (slider) · Default step? · 0 = continuous (today, egui) / 1 (bits-ui) · 0 = continuous, documented as a crate choice.
- `CP-05-31` (slider) · Pointer mapping? · over the thumb travel, thumb centre under the pointer (today) / over the whole root as bits-ui (thumb lands slightly off the pointer) · today's.
- `CP-05-32` (slider) · End/drag when the step does not divide the range? · reach `max` (today `slider.rs:242-244`) / last grid value (bits-ui `normalizeSteps`) · last grid value.
- `CP-05-33` (slider) · Thumbs crossing (multiple)? · swap and move focus (bits-ui `autoSort`) / stop at the neighbour · swap.
- `CP-05-34` (slider) · Array `step` (allowed values)? · `.steps(&[f32])` / not offered (no shadcn demo uses it) · not offered.
- `CP-08-20` (command) · result ordering while searching · keep source order, hide non-matches (fits immediate mode) / re-sort items and groups by score like bits (`command.svelte.ts:190-268`; needs a collector) · sketch: keep source order.
- `CP-08-21` (command) · filter algorithm · port `computeCommandScore` (fuzzy, abbreviations, keywords) / case-insensitive substring / caller filter only · sketch: port, with `.filter(fn)` override and `.should_filter(false)`.
- `CP-15-5` (calendar) · unavailable dates · not selectable (bits-ui code) / selectable and marks invalid (bits-ui `types.ts` doc) · pick: not selectable.
- `CP-15-22` (calendar) · month and weekday names · English tables plus `text(&CalendarText)` override / format callbacks (`monthFormat` function form) / `icu` dependency · pick: English plus struct override.
- `CP-15-29` (calendar) · navigation options `months`, `paged_navigation`, `fixed_weeks`, `min`/`max`, `years` · all / only `months` + `min`/`max` · pick: all.
- `CP-06-6` (input) · Which TextEdit options pass through · none / `char_limit` only / also `return_key`, `horizontal_align`, `prefix`/`suffix` · pick: `char_limit`; prefix/suffix belong to InputGroup.
- `CP-06-15` (input-otp) · Pattern type · regex string (needs `regex` dependency) / `fn(char) -> bool` / enum `{Any, Digits, Letters, DigitsAndLetters}` · pick: `fn(char) -> bool`, default any.
- `CP-06-21` (input-otp) · Paste · reject whole paste on a bad char (bits) / keep the allowed chars / `paste_filter` transformer · pick: reject whole, no transformer.
- `CP-17-9` (chart) · Legend semantics · click toggles visibility (layerchart `selectedKeys.toggle`, unselected fade to 0.3) / click isolates one series · sketch: toggle.
- `CP-19-29` (questionnaire) · freeform input types (`email`, `number`, `date`, …, `QuestionnaireInputType`) · text only / text + `.password(bool)` / an input-type enum that only changes validation · text only
- `CP-14-34` (resizable) · Size unit · (a) percent 0–100 (paneforge, docs) (b) fraction 0–1 (c) pixels · sketch: (a)
- `CP-11-29` (drawer) · snap points · omit / `.snap_points(&[f32])` + `&mut Option<usize>` active · omit.
- `CP-15-26` (calendar) · "today" · caller `.today(Date)` / local date via the optional `jiff` feature (`tz-system`) / UTC from `SystemTime` · pick: local via `jiff` when enabled, else UTC, `.today()` overrides. _(tagged "Depends"; no topic file lists it)_

**G15 · Tagged 1 but about component shape, not selection**
Placed under the topic their question belongs to: CP-11-17, CP-11-25, CP-11-26, CP-19-11, CP-16-12.


**New choice points from the topic cross-check (no sketch pick)**

- `CP-T1-1` · crate-wide value type of a choice · generic `T: PartialEq (+ Clone)` (toggle-group, radio-group, select, native-select, tabs, accordion, menu radio, sidebar) / `usize` index (questionnaire, select shortcut, today's Tabs, Select) / domain type (calendar `Date`, slider `f32`, pagination, carousel) · idiom §Selection
- `CP-T1-2` · trait bound on generic values · `PartialEq + Clone` (toggle-group, radio-group, select, tabs, accordion) / `PartialEq` only, value moved in per item (native-select `option`, menu radio, sidebar `menu_value`) / per component · idiom §Selection
- `CP-T1-3` · how single mode says "nothing selected" · `&mut Option<T>` in the signature (toggle-group, calendar, accordion) / caller picks `V = Option<T>` (radio-group, select, tabs, menu radio) / collection of at most one (questionnaire) · idiom §Selection
- `CP-T1-4` · deselect by a second click: default and setter · allowed, no setter (toggle-group) / on by default, `.prevent_deselect(bool)` (calendar) / off by default, `.allow_deselect()` (select) / not offered (radio-group, tabs, menu radio) · no egui idiom
- `CP-T1-5` · multiple-mode container · `&mut Vec<T>` (toggle-group, accordion, calendar, questionnaire, select) / `&mut HashSet<T>` (chart legend, menu CheckboxGroup option) / `&mut [bool]` by index (chart option, CP-01-20 option) · no egui idiom
- `CP-T1-6` · spelling of single / multiple / range · `single`+`multiple` constructors (toggle-group, accordion) / `new`+`multiple`(+`range`) (calendar, slider, select) / one constructor + `.multiple(bool)` (questionnaire) / separate types (menu items) · idiom §Widget builders
- `CP-T1-7` · crate-wide rule for supplying items and placing the binding · group binding + item data (toggle-group, select, questionnaire, shortcuts) / group binding + chained `.item` (tabs) / group binding + handle in a closure (accordion) / per-item `_value` form (radio-group, native-select, menu radio, sidebar) / count + index closure (carousel) · idiom §Selection, §Containers
- `CP-T1-8` · value form of list shortcuts · index `&mut usize` + `&[&str]` (select shortcut) / generic `&mut T` + `&[(T, &str)]` (tabs, native-select) / same form as the builder · idiom §Selection (`ComboBox::show_index`)
- `CP-T1-9` · crate `radio_value(ui, &mut v, value)` has no text, egui's `ui.radio_value(&mut v, value, text)` does · keep name, other signature (radio-group) / take a text like egui (menu radio, sidebar) / other name (`radio_group_item`, CP-05-19) · idiom §Selection
- `CP-T1-10` · index base of index values · 1-based where the label shows the number (pagination) / always 0-based (carousel, questionnaire, select shortcut) · no egui idiom (egui is 0-based); batch-B Y16
- `CP-T1-11` · name of the "shown as chosen" flag when the caller owns the selection · `active` (navigation link, sidebar MenuButton) / `selected` (table row, egui `Button::selectable`) / `checked` (CommandItem) / per upstream attribute · idiom §Selection
- `CP-T1-12` · value that matches no item · show placeholder, nothing marked (select, radio-group) / fall back to the first item / debug assert; unstated for tabs, toggle-group, native-select, accordion · no egui idiom
- `CP-T1-13` · may a selection binding be optional with a memory fallback · required `&mut` (most widgets; CP-17-8 pick) / optional, memory otherwise (chart sketch field, carousel CP-16-13 (c), the CP-10-2 form) · idiom §Open state; batch-B Minor (chart)

### T2 · Open state

**G1 · Anchored overlay open state (memory, memory + binding, caller)**
See also (placed in another group): CP-10-2, CP-10-25, CP-10-3, CP-09-4, CP-13-2.

- `CP-08-7` (select) · open state · egui memory (today) / optional `.open(&mut bool)` · sketch: memory.

**G2 · Modal open state (Dialog family against CP-10-2)**
See also (placed in another group): CP-10-2, CP-11-35.

- `CP-11-1` (dialog) · who owns open state · `&mut bool` arg to `show` (today) / `.open(&mut bool)` setter like `Window::open` / caller shows only while open and reads `should_close()` like `egui::Modal` · `&mut bool` arg to `show`. The crate-wide rule is CP-11-35 against CP-10-2.
- `CP-08-24` (command) · CommandDialog open state · `&mut bool` (as `Dialog::show`) / egui memory with a toggle helper · sketch: `&mut bool`.

**G3 · Disclosure and panel state (non-overlay open/closed)**
See also (placed in another group): CP-12-28, CP-14-1.

- `CP-12-14` (accordion) · accordion open state owner · caller `&mut Option<V>` / `&mut Vec<V>` / memory + `.default_open` / both · sketch: caller-owned; CP-10-2 form would be memory per item + optional binding
- `CP-14-11` (sidebar) · collapsible sidebar groups and sub-menus · sidebar-specific collapsibles with memory / the Collapsible port (CP-12-27, CP-12-28: caller keeps a bool per sub-menu) / egui `CollapsingState` · sketch: the Collapsible port

**G4 · Other view state ownership (not "open", same question)**
See also (placed in another group): CP-14-33, CP-15-6.

- `CP-19-10` (message-scroller) · where scroll mode and commands live · egui memory keyed by id, commands as associated fns / a caller-owned `&mut MessageScrollerState` passed to `show` (commands are methods on it) · memory + associated fns (matches `ScrollArea` state and `Popup::open_id`)
- `CP-19-24` (questionnaire) · who owns active item and statuses · caller `&mut QuestionnaireState` / egui memory with `.active(&mut usize)` override (controlled vs uncontrolled as upstream) · caller-owned
- `CP-16-25` (sonner) · Where the queue lives · (a) `ctx` memory, global per context, free functions; (b) an app-owned `Toasts` struct passed by `&mut` to `toast()` and `show()` · (a)
- `CP-08-23` (command) · who owns the search text · egui memory / caller `&mut String` (upstream `bind:value` on the input) / both via an optional setter · sketch: memory.

**G5 · Binding setter name (crate-wide)**
See also (placed in another group): CP-14-43.


**G6 · Programmatic query/control of memory-owned state**
See also (placed in another group): CP-14-18.

- `CP-10-24` (popover) · how the trigger learns the open state for `Button::expanded` · `Popover::is_open(ctx, id)` static / caller `&mut bool` only / not offered · static `is_open`.

**G7 · Persistence of memory-owned state**

- `CP-14-2` (sidebar) · Persist the open state (upstream cookie, 7 days)? · (a) `insert_persisted` (b) temp memory (c) caller's job · sketch: (a) when memory-owned
- `CP-14-38` (resizable) · Persist the layout (upstream `autoSaveId`) · (a) persisted memory always (b) opt-in (c) never · sketch: (b)

**G8 · Opening mechanism (how the state changes, not who owns it)**

- `CP-10-15` (hover-card) · mechanism · egui `Tooltip` with interactive keep-alive (global `tooltip_delay`, one per layer) / `Popup` + own hover timer in memory · `Popup` + own timer.
- `CP-09-25` (context-menu) · Touch long-press (700 ms) · port / skip (egui apps rarely run on touch; egui has `long_touched()`) · port via `Response::long_touched()`, not a 700 ms timer of our own.
- `CP-09-29` (menubar) · Hover-switch and Left/Right switching between menus · port (bits) / egui `MenuBar` behaviour (none) · port.
- `CP-14-3` (sidebar) · Cmd/Ctrl+B shortcut · (a) always on (b) `.shortcut(Option<KeyboardShortcut>)`, default on (c) none · sketch: (b)
- `CP-15-7` (calendar) · view follows an external value change · no (bits-ui Calendar) / yes (bits-ui RangeCalendar watches `value.start`) · pick: follow upstream per mode (range follows, single not).

**G9 · Tagged 2 but about another topic**
Placed under the topic their question belongs to: CP-04-23, CP-09-19, CP-11-12, CP-13-14, CP-14-28.


**New choice points from the topic cross-check (no sketch pick)**

- `CP-T2-1` · do select, native-select, menubar and navigation-menu join the overlay open-state rule of CP-10-2 / CP-10-25 · yes, memory + optional binding / no, memory only (CP-08-7, CP-09-4, CP-13-2) / follow CP-10-25, listed explicitly · idiom §Open state form 2, §Popups (`Popup::open_bool`)
- `CP-T2-2` · does "memory + optional binding" (CP-10-2) reach disclosure, panel and view state · caller-owned only (accordion, collapsible, carousel, questionnaire, chart legend, tabs) / memory + binding (sidebar, resizable, calendar month) / memory only (message-scroller, sonner, command search) / one rule per family, written down · idiom §Open state
- `CP-T2-3` · how app code queries or drives memory-owned state · static fns keyed by `Id` (`Popover::is_open`, `Sidebar::toggle`, `MessageScroller::scroll_to_*`, `Dialog::open`) / free fns on `ctx` (sonner `toast()`) / typed state `load`/`store` (resizable, egui `CollapsingState`) / none, use `Popup::is_id_open` or the binding; sub-question: key = trigger id or own `id_salt` · idiom §Open state, §IDs
- `CP-T2-4` · initial value of memory-owned state · `.default_open(bool)` everywhere (sidebar, egui `CollapsingHeader`) / per-kind defaults where upstream has one (resizable `default_size`, sidebar) / no default setter (popover, menus, select, navigation-menu, hover-card, calendar) · idiom §Open state
- `CP-T2-5` · persistence of memory-owned state across restarts · persisted by default (sidebar CP-14-2) / opt-in setter (resizable CP-14-38) / temp only, caller persists via the binding (popover, menus, calendar, message-scroller) · idiom §Open state (`insert_persisted`)
- `CP-T2-6` · do hover overlays (hover-card, tooltip) take the optional open binding · no, memory/timer only (CP-10-15) / yes, as popover (upstream has `bind:open`) · idiom §Popups
- `CP-T2-7` · value type of "which item is open" · caller's `V` (accordion CP-12-14) / index `&mut Option<usize>` (navigation-menu CP-13-2) / opaque id in memory, no binding (menubar) · idiom §Selection
- `CP-T2-8` · chart legend hidden-series binding · required `&mut HashSet` (CP-17-8 pick) / `Option<&mut HashSet>` with memory fallback (sketch field; the CP-10-2 form) · idiom §Open state; batch-B Minor

### T3 · Overlay: anchor, side and align, dismissal

**G1 · Side and align expression for anchored overlays**

- `CP-10-1` (popover) · how to express side + align · `RectAlign` / separate `Side` and `Align` enums / both · `RectAlign` for popover, hover-card and tooltip.
- `CP-09-5` (dropdown-menu) · Placement setter · one `RectAlign` (egui) / shadcn `side` + `align` + `align_offset` · `.align(RectAlign)` + `.side_offset`; `alignOffset` (sub-content −3, menubar −4) applied internally.
- `CP-08-8` (select) · side and align · fixed bottom-start + `side_offset` (today) / `.align(RectAlign)` setter · sketch: fixed.
- `CP-14-23` (sidebar) · Collapsed tooltip · (a) `.tooltip(text)` shown only while collapsed (b) caller attaches `tooltip()` · sketch: (a); needs a side argument in `tooltip`, which CP-10-1 answers with `RectAlign` (`RectAlign::RIGHT`, upstream `side="right" align="center"`).
- `CP-09-24` (context-menu) · Default placement relative to the pointer · `RIGHT_START`, gap 2 (bits `side="right" sideOffset=2`) / egui `Popup::context_menu` default (`at_pointer_fixed` with `Popup`'s default align) · bits. _(tagged "3"; no topic file lists it)_

**G2 · Placement type for non-anchored placements**
See also (placed in another group): CP-11-18, CP-11-19, CP-14-13.

- `CP-18-18` (bubble) · Reactions placement type · two enums (upstream) vs. `egui::Align2` vs. `RectAlign` (outside-placement semantics do not fit an overlapping pill) · two enums.

**G3 · Collision fallback and containment**

- `CP-10-20` (popover) · collision fallback · egui default (`symmetries()` then `MENU_ALIGNS`, `popup.rs:287`) / opposite side only (bits `flip()`) / none · opposite side only.
- `CP-17-5` (chart) · Tooltip containment · egui default (screen) / clamp to chart rect like layerchart `contained='container'` · sketch: egui default.

**G4 · Modal placement**

- `CP-08-25` (command) · CommandDialog placement · `Dialog` gains a top-third placement option / `CommandDialog` builds its own `Modal` · sketch: option on `Dialog`.

**G5 · Escape**

- `CP-09-20` (dropdown-menu) · Escape · closes only the deepest sub-menu (bits) / closes every open menu (egui `Popup`, popup.rs:615) · bits.
- `CP-11-13` (alert-dialog) · Escape for an alert dialog · closes (bits-ui) / ignored · closes.

**G6 · Tab and focus while open**

- `CP-08-15` (select) · Tab while open · close (bits `:850`) / keep open and hand keys over (today) · sketch: close.
- `CP-09-21` (dropdown-menu) · Tab inside an open menu · close and move focus past the trigger (bits) / egui default (focus moves, menu stays until a click) · bits.
- `CP-10-7` (popover) · focus on open/close · none (today) / move focus to the first focusable widget and return it to the trigger on close / full trap · return focus to the trigger on close only.

**G7 · Opening on keyboard focus (hover overlays)**

- `CP-10-19` (hover-card) · keyboard focus opening (bits `onfocus` on focus-visible) · port / skip · port (open on `has_focus()` after keyboard navigation, close on `lost_focus()`).
- `CP-10-12` (tooltip) · open on keyboard focus (bits `#onfocus`) · port / skip (egui tooltips are hover-only) · port (show while `has_focus()` after keyboard navigation).

**G8 · Hover timing and click-outside on hover overlays**

- `CP-13-10` (navigation-menu) · Hover-intent timings: 200 ms open, 150 ms switch, 300 ms click suppression. Expose them? · constants with `// behaviour-ok` / `.delay(secs)` setter (bits `delayDuration`) · constants plus `.delay`.
- `CP-13-11` (navigation-menu) · Should a click outside close the menu for hover items, on touch? Upstream does not close it (`navigation-menu.svelte.ts:978`) · close on any outside click (egui `PopupCloseBehavior::CloseOnClickOutside`) / copy upstream · close on any outside click.

**G9 · Trigger activation**

- `CP-09-22` (dropdown-menu) · Trigger fires on pointer **down** (bits) or on click (egui `Popup::menu`, `clicked()`) · down / click · click (egui), drag-to-select (B-M7) dropped.

**G10 · Gesture and non-modal dismissal of edge panels and toasts**

- `CP-11-27` (drawer) · drag-to-dismiss · port (threshold 0.25, velocity 0.4 px/ms) / omit and close only by Esc/backdrop · port.
- `CP-11-28` (drawer) · non-modal mode (`modal=false`) · `.modal(bool)` on an `Area` / omit · `.modal(bool)`.
- `CP-11-30` (drawer) · nested drawers and background scaling · not applicable (egui cannot scale the app below) / draw parent drawers displaced by 16px · not applicable.
- `CP-16-31` (sonner) · Swipe to dismiss · (a) port it (45 px / 0.11 px/ms, directions from position); (b) no swipe, close only · (a)

**G11 · Overlay animation and stacking look**

- `CP-11-24` (sheet) · animation (slide-in) · none (crate rule: ANIM ignored) / `ctx.animate_bool` slide · none.
- `CP-13-4` (navigation-menu) · Viewport size and switch animation · animate size and slide (`data-motion`) / jump at once (crate ignores animation elsewhere) · jump at once.
- `CP-16-32` (sonner) · Collapsed stack look · (a) port scale 0.05/index and lift, expand on hover; (b) always expanded (`expand=true`) · (a)

**G12 · Overlay geometry details**

- `CP-10-13` (tooltip) · arrow gap · arrow reach `size/√2` so the tip touches the trigger (today) / bits `sideOffset + arrow box height` (10 px) · bits formula (unsure about the exact upstream geometry; compare with the docs page).
- `CP-08-9` (select) · scrolling · `egui::ScrollArea` (today) / port the hover-to-scroll buttons · sketch: ScrollArea.
- `CP-13-3` (navigation-menu) · Port `viewport = false` (per-item popup) as well as the shared viewport? · both, via `.viewport(bool)` / shared viewport only / per-item only (simpler, it is just `Popup`) · both.
- `CP-07-25` (native-select) · What does the open list look like (the OS list is unstyled upstream; `bg-[Canvas]`) · (a) egui's default popup visuals (closest to "native"), (b) the crate Select's `cn-select-content` / `cn-select-item` recipes, (c) a plain popup in theme `Popover` colours · sketch: (c)

**G13 · Painting outside the allocated rect**

- `CP-16-16` (carousel) · Button placement outside the rect · (a) paint outside the allocated rect, as CSS does (overlaps neighbours); (b) reserve 48 px gutters inside the allocation · (b)
- `CP-18-23` (bubble) · Space under/over reactions · reserve the overhang in the bubble's allocated rect vs. paint outside (upstream; overlaps neighbours) · reserve.

**G14 · Overlay machinery and scope**

- `CP-09-1` (dropdown-menu) · build menus on egui's menu machinery (`Popup::menu`, `SubMenu`, `MenuState`) themed by a `StyleModifier`, or on the crate's own popup code like Select · egui machinery / own code · sketch: egui machinery for state and sub-menus, own painting
- `CP-14-12` (sidebar) · sidebar mobile mode (< 768 px → Sheet) · port by viewport width / drop / caller `.mobile(bool)` · sketch: drop for the first port; with a port the Sheet's `&mut bool` (CP-11-1, CP-11-35) meets the sidebar's memory (batch-B N2)
- `CP-15-8` (calendar) · date picker · inline calendar only / also ship `DatePicker` (button + Popover + Calendar) · pick: inline only.

**Mis-tagged**
Placed under the topic their question belongs to: CP-05-18, CP-18-9, CP-13-34, CP-01-31.


**Moved here from another topic tag**

- `CP-11-12` (alert-dialog) · does Action close the dialog · no (bits-ui) / yes (common app expectation) · no. _(tagged 2)_

**New choice points from the topic cross-check (no sketch pick)**

- `CP-T3-1` · do list-style anchored overlays expose placement · fixed, no setter (select CP-08-8, native-select, menubar, navigation-menu, calendar caption) / `.align(RectAlign)` + `.side_offset` on every anchored overlay / `.side_offset` only on list pickers · idiom §Anchoring
- `CP-T3-2` · align offset (bits `alignOffset`) · internal constants (CP-09-5: sub-content −3, menubar −4) / `.align_offset(f32)` on every anchored overlay / not offered (popover, hover-card, tooltip, navigation-menu) · idiom §Anchoring (`Popup::gap` main axis only)
- `CP-T3-3` · collision fallback for all anchored overlays · egui default alternatives (select, menus, native-select) / opposite side only (popover family, CP-10-20) / per family as sketched · idiom §Anchoring (`align_alternatives`)
- `CP-T3-4` · Escape with several open overlays · egui default: one press closes every open `Popup` (popover, hover-card, tooltip, select, navigation-menu) / deepest only, Escape consumed by the top layer (CP-09-20, bits `EscapeLayer`) · idiom §Popups
- `CP-T3-5` · Tab or focus leaving an open non-modal overlay · closes it (select CP-08-15, menus CP-09-21, navigation-menu, hover-card) / stays open (popover sketch, egui) / trapped (bits popover `trapFocus`) · idiom §Focus and keyboard
- `CP-T3-6` · one focus rule on open and close · per component (CP-10-7, CP-11-7, CP-11-16, CP-11-33, CP-09-19) / initial focus + return for modals, return only for anchored overlays / none (egui default) · idiom §Focus and keyboard
- `CP-T3-7` · hover-delay setters · `.open_delay` + `.close_delay` (hover-card) / one `.delay(secs)` (navigation-menu) / none, global `tooltip_delay` (tooltip) / the same pair on all three · no egui idiom
- `CP-T3-8` · dismissal configurability · only drawer `.dismissible` / `.modal` (sketches) / anchored overlays forward egui `.close_behavior(PopupCloseBehavior)` / one `.dismissible(bool)` on every overlay and modal · idiom §Popups; setter form per CP-01-31
- `CP-T3-9` · sonner placement type · own `ToastPosition` (6 values, sketch) / `egui::Align2` as `Area::anchor` / edge `Side` (CP-11-19) + `Align`; decide with CP-18-18 whether one boundary covers every placement type · idiom §Popups (`Area` uses `Align2`)
- `CP-T3-10` · overlay animation rule (batch-B Y13) · none for any overlay (CP-11-24, CP-13-4, CP-14-16) / animate where upstream does (sonner CP-16-32, CP-16-35) / per component · no egui idiom (`ctx.animate_bool` exists)

### T4 · Sub-parts

**G1 · How the header parts (title, description, label) of a multi-part component are supplied**

- `CP-03-2` (card) · how are header, title, description, action supplied · free functions in the body (today: `card_header`, `card_header_with_action`) / builder setters on `Card` / a `CardHeader` builder · builder setters, free `card_title`/`card_description` kept for custom headers
- `CP-03-10` (card) · header without description · always both (today) / optional description · optional
- `CP-03-11` (item) · content supply · data builder (title, description, media on `Item`) / closure body with free `item_title`, `item_description`, `item_media` (the card's current style) · data builder, free functions for custom content
- `CP-03-23` (empty) · content supply · data builder (title, description, media) + content closure / closure body with free `empty_title` etc. · data builder (consistent with Item, Card)
- `CP-04-1` (alert) · how Title/Description/Action map · builder fields / `Alert::show` + free `alert_title`/`alert_description` like `card_title` / both · sketch: both
- `CP-07-5` (field) · how the ten Field parts map · slot builder (`Field::new().label().description().error()` + control closure) / free `field::*` fns in `Field::show` reading `Ui` context / struct per part · sketch: slot builder + free fns for description, error, separator
- `CP-07-15` (field) · FieldTitle vs FieldLabel · (a) separate `.title()` slot (cn-field-title, no control link), (b) `.label()` only · sketch: (a)
- `CP-07-19` (field) · FieldContent with Label + Description inside a horizontal Field (`examples/field-checkbox.svelte` last block) · (a) implied by label+description in horizontal mode, (b) explicit `field::content(ui, |ui| ..)` · sketch: (a)
- `CP-10-4` (popover) · header parts · one `popover_header(ui, title, desc)` (today) / separate `popover_title` + `popover_description` (card pattern, `card.rs:326,346`) / both · both, each returning `Response`.
- `CP-18-1` (attachment) · How the Svelte parts map · one builder with setters for media/title/description vs. a closure per part · builder.
- `CP-19-1` (message) · how avatar/header/footer are supplied · builder setters (closures) / free functions called inside the content closure (`message_header(ui, ..)`) / a `MessageUi` handle passed to the closure · builder setters (the root needs `has-footer` and order before it lays out)
- `CP-19-4` (message) · empty avatar slot (`<Message.Avatar />` in `message-group`) · `.avatar_space()` / `.avatar(|_| {})` / not offered · `.avatar_space()`
- `CP-14-5` (sidebar) · Header/content/footer mapping · (a) three closures in `show_inside` (Sides style) (b) one closure + `sidebar::header/content/footer` fns (c) `.header(..)`/`.footer(..)` builder closures + content closure · sketch: (c)
- `CP-16-1` (table) · How do the eight Svelte parts map? · (a) one builder with `header/row/footer` closures and `text/cell` on rows; (b) a struct per part (`TableRow::new().show`); (c) wrap `egui::Grid` and let the caller call `ui.end_row()` · (a)
- `CP-04-12` (progress) · Label/value parts · port the React base-ui `cn-progress-label`/`cn-progress-value` markers into a `.label(..)`/`.show_value()` option vs leave the label to the caller like svelte · sketch: caller (svelte has no such part).

**G2 · Separate type, mode of another type, or folded away**

- `CP-06-9` (textarea) · Separate type or constructor? · `Textarea::new` (shadcn name) / `Input::multiline` (egui `TextEdit::multiline` idiom) · pick: separate `Textarea`.
- `CP-07-22` (native-select) · Separate widget, or a mode of Select? · (a) own `NativeSelect` (own markers, plain list), (b) `Select::native()` switch on the existing Select, (c) not offered: point `<NativeSelect>` at `Select` in the build-ui map · sketch: (a)
- `CP-04-21` (avatar) · Image + Fallback mapping · one `Avatar::new(fallback).image(src)` vs separate `AvatarImage`/`AvatarFallback` structs · sketch: one widget.
- `CP-07-20` (form) · Port `form` at all? · (a) not applicable, fold the aria wiring (described-by, invalid-from-errors, required) into Field, (b) a thin `Form` container that collects field errors and gates a submit button, (c) a validation crate dependency · sketch: (a)
- `CP-12-27` (collapsible) · ship collapsible at all · nothing, document the `if` pattern / helper fn `collapsible_trigger` (`mark_changed`, `expanded`) / container `Collapsible::new(&mut open).show(..)` · sketch: helper fn
- `CP-19-9` (message-scroller) · how many parts · one `MessageScroller` builder with an item handle / separate `MessageScrollerViewport` + `MessageScrollerButton` widgets sharing an id / reuse `egui::ScrollArea` and add only helpers · one builder
- `CP-14-4` (sidebar) · Root shape · (a) `Sidebar` builder over `egui::Panel` `show_inside` (b) free fn layout (c) provider + root split as in Svelte · sketch: (a)
- `CP-14-17` (sidebar) · Rail · (a) `.rail(bool)` on `Sidebar` (b) separate `sidebar::rail(ui)` · sketch: (a)
- `CP-14-19` (sidebar) · Inset · (a) `sidebar::inset(ui, |ui|..)` that reads the variant from context (b) caller frames the central panel · sketch: (a)
- `CP-13-24` (pagination) · Composition · one widget with switches / expose parts (`PaginationLink`, `PaginationPrevious`, …) for an upstream-like custom layout / both, via public `page_items` · one widget plus public `page_items`.
- `CP-19-21` (questionnaire) · questionnaire overall shape · data model (`&[Question]` + caller state) / closure composition (only the active item's closure runs) · sketch: data model (validation of unseen items needs inspectable answers)
- `CP-19-26` (questionnaire) · custom layouts (progress in a card header, title in `Card.Header`, `questionnaire-validation`, `-card`) · not offered / free part functions (`questionnaire_progress(ui, &state)`) / a closure hook per item · free progress function only
- `CP-13-5` (navigation-menu) · Port the Indicator (arrow under the active trigger)? The shadcn Root never renders it · skip / opt-in `.indicator(true)` · skip.
- `CP-08-33` (command) · Loading part · omit / `cmd.loading(progress)` row · sketch: omit (no class, no demo).
- `CP-16-27` (sonner) · Custom component toasts (`toast.custom`, component title) · (a) not offered; (b) a toast with a `'static` content closure · (a)
- `CP-03-6` (card) · leading image and edge-to-edge content (`card-image`, `card-edge-to-edge`, header `[.border-b]`, footer `[.border-t]`) · not offered (today, variants ignored) / `.image(Image)` + a bleed content part · not offered
- `CP-11-17` (alert-dialog) · separate type or `Dialog` flag · separate `AlertDialog` / `Dialog::alert()` mode · separate type sharing the modal shell. _(tagged 1)_
- `CP-11-25` (sheet) · own type or Dialog variant · separate `Sheet` / `Dialog::side(..)` · separate type, shared shell. _(tagged 1)_
- `CP-11-26` (drawer) · Drawer as own type or Sheet option · separate `Drawer` / `Sheet::draggable()` · separate type (different look: rounded edge, handle, no close button, centred header). _(tagged 1)_

**G3 · How footer, action and close parts are supplied**

- `CP-03-3` (card) · footer layout: upstream demos use `flex-col` footers (`card-demo`, `card-small`) and `justify-end` (`card-edge-to-edge`) · fixed centred row (today) / `.footer_layout(Layout)` / caller nests `layout::vstack` · fixed row, caller nests a column
- `CP-03-15` (item) · header/footer parts · closures on the builder / not offered · closures
- `CP-11-5` (dialog) · footer shape · free fn `dialog::footer(ui, ..)` (today) / builder `.footer(|ui| ..)` like `Card::footer` (`card.rs:92`) / method on a handle passed to the body · free fn, returning `InnerResponse<R>`.
- `CP-11-10` (alert-dialog) · body content · closure is footer only (upstream structure) / closure is a free body and a `footer` fn as in Dialog · footer only.
- `CP-19-3` (message) · footer content type · closure (demos put text, buttons, a "Failed to send" + retry button) / `WidgetText` + a separate actions closure · closure
- `CP-11-23` (sheet) · footer pinning (`mt-auto`) · footer fn allocates the remaining height first so it sits at the bottom / footer follows the body · pinned.
- `CP-04-4` (alert) · Action slot · closure `FnOnce(&mut Ui)` (like `Card::footer`) vs `IntoAtoms`/Button only · sketch: closure.
- `CP-18-2` (attachment) · How actions are supplied · `show(ui, |ui| …)` closure vs. `.action(&Icon, label)` list returning a `Vec<Response>` · closure + `attachment_action()` helper.
- `CP-14-7` (sidebar) · Group API · (a) `group(ui, label: Option<&str>, |ui|..)` fn (b) `Group` builder with `.label/.action` and `show` returning an output with the action response · sketch: (b)
- `CP-14-9` (sidebar) · Menu action and badge · (a) options on `MenuButton`, `show(ui) -> MenuButtonOutput { response, action }` (b) separate widgets the caller places with `ui.place` (c) action content as a closure · sketch: (a)
- `CP-11-11` (alert-dialog) · how Action/Cancel are offered · helper fns `cancel`/`action` in the closure / builder `.cancel("..").action("..")` and `show` returns `Option<AlertDialogChoice>` / plain `Button`s and `ui.close()` · helper fns.
- `CP-03-16` (item) · a second content column (`item-image-demo` duration) · `.trailing(text)` setter / actions closure with `item_description` · actions closure
- `CP-03-24` (empty) · children after Empty.Content (`empty-demo` link button) · inside the content closure / a second `.footer` closure · inside the content closure
- `CP-18-17` (bubble) · Reactions supply · builder closure (like `Card::footer`) vs. a separate call after `show` using the returned rect vs. `Vec<WidgetText>` emoji list · builder closure.
- `CP-10-5` (popover) · `PopoverClose` · `ui.close()` only / a `popover_close(ui, …)` helper · `ui.close()` only.
- `CP-11-6` (dialog) · footer `showCloseButton` (outline "Close", `dialog-footer.svelte`) · omit (caller adds a Button + `ui.close()`) / `footer_with_close(ui, ..)` / flag on Dialog · omit.
- `CP-11-9` (dialog) · tab order of the corner close button · last, after the body (upstream order, today) / first · last.

**G4 · Built-in control parts vs caller-placed parts**
See also (placed in another group): CP-13-27.

- `CP-16-15` (carousel) · Prev/next buttons · (a) built in, toggled by `.buttons(bool)`; (b) caller places them with returned `can_prev`/`can_next` and calls a method; (c) both · (a)
- `CP-19-14` (message-scroller) · scroll button supply · builder option painted by the scroller / separate widget the caller places, reading `out.scrollable` / both · builder option
- `CP-19-25` (questionnaire) · action row · built in, labels via setters / caller places `questionnaire_next(ui, ..)` etc. / both · built in
- `CP-15-11` (calendar) · `buttonVariant` for prev/next · expose `ButtonVariant` / fixed ghost · pick: fixed ghost.
- `CP-16-20` (carousel) · Autoplay plugin · (a) not offered (caller advances the index with a timer); (b) `.autoplay(Duration)` with stop on hover · (a)

**G5 · How children reach their parent, and how items are supplied**

- `CP-01-7` (button-group) · how children learn they are in a group and where · `Ui` stack tag + previous-pass child count (a) / typed closure arg `ButtonGroupUi` that counts (b) / data-only item list (c) / repaint corners after the closure (d) · sketch: (a)
- `CP-05-11` (radio-group) · How does an item find its group? · `UiStackInfo` tag on the group's child `Ui` / an explicit handle passed to the closure (`|ui, g| g.item(ui, ..)`) · `UiStackInfo` tag.
- `CP-09-27` (menubar) · How a menu joins the bar · `menubar_menu(ui, ..)` inside `Menubar::show(ui, |ui| ..)` finding the bar via a `UiStack` tag (egui `MenuBar` + `menu_button`) / a bar handle passed to the closure (`|bar| bar.menu(..)`) · `UiStack` tag.
- `CP-14-6` (sidebar) · How parts learn collapsed/side/variant · (a) closure receives a `SidebarUi` wrapper (b) `UiStackInfo::tags` set by the sidebar, read by free fns/widgets (c) passed explicitly to each part · sketch: (b)
- `CP-03-18` (item) · `in-data-[slot=dropdown-menu-content]` padding · ignore / context flag set by DropdownMenu through `UiStackInfo` · context flag once DropdownMenu exists
- `CP-08-19` (command) · how items are supplied · immediate-mode closure, items return `Response` (egui menu idiom) / collector closure, then filter, sort and paint / data slice of entries returning `Option<index>` · sketch: immediate-mode closure.
- `CP-12-17` (accordion) · how items are supplied · closure with an `AccordionUi` item builder / free per-item fn `accordion_item(ui, &mut state, value, text, body)` without a root / slice of `(V, &str, body)` · closure with item builder.
- `CP-13-1` (navigation-menu) · How do Svelte Item/Trigger/Content map? · `nav.item(trigger, content)` handle method / `ui.add(NavigationMenuTrigger)` + separate `NavigationMenuContent::show` / free fn `navigation_menu_item(ui, …)` · handle method, modelled on `menu_button`.
- `CP-13-16` (breadcrumb) · breadcrumb parts mapping · handle with methods / plain closure + free `BreadcrumbLink`, `BreadcrumbPage`, `BreadcrumbSeparator` widgets / data-only `Breadcrumb::new(&[..])` · sketch: handle
- `CP-19-12` (message-scroller) · how items are supplied · per-item closure on a handle (`s.item(..)`) / a slice + row callback (`show_items(&msgs, |ui, m| ..)`) / plain children with an `anchor` marker call · per-item closure
- `CP-01-23` (toggle-group) · Items: data list of `ToggleItem<T>` (count known, no two-pass) / closure `show(ui, |g| g.item(..))` like `selectable_value` runs / reuse `Toggle` widgets inside a `show` closure with a group tag (as CP-01-7) · sketch: data list
- `CP-04-26` (avatar) · Group composition · data widget `AvatarGroup::new(iter)` vs container `AvatarGroup::new().show(ui, |ui| { ui.add(Avatar…) })` that sets negative spacing and an in-group context in the child `Ui` · sketch: data widget (the group must know the avatars' size for the count and paint each ring).
- `CP-12-2` (tabs) · how triggers are supplied · `&[&str]` in `new` (today) / chained `.tab(value, atoms)` / closure `show(ui, |tabs| tabs.tab(..))` · chained `.tab`.
- `CP-06-14` (input-otp) · How groups and separators are given · `.groups(&[usize])` / `.separator_after(&[usize])` / a closure per cell (Svelte snippet copy) · pick: `.groups`.
- `CP-14-32` (resizable) · Pane content supply · (a) index closure `show(ui, |ui, i|..)` with a pane spec slice (`show_rows` style) (b) collector `|g| { g.pane(..); g.handle(); }` (c) builder storing boxed closures per pane · sketch: (a)
- `CP-16-14` (carousel) · How slides are supplied · (a) count + `FnMut(&mut Ui, usize)`; (b) a `slide(|ui| ..)` call per slide inside one closure; (c) a slice of items · (a)
- `CP-05-8` (radio-group) · Shape of the group API? · container `RadioGroup::new(id).show(ui, |ui| radio_value(..))` / leaf `RadioGroup::new(&mut usize, &[&str])` like `Tabs`, `Select` / no group, bare `radio_value` runs (egui) · container (items need arbitrary neighbours: labels, descriptions, choice cards).
- `CP-12-4` (tabs) · does Tabs own the content panel · list-only widget, caller matches (today) / `.show(ui, content)` that lays out gap, side placement and content text size / both · both.
- `CP-06-24` (input-group) · Overall structure · builder with stored addon closures + `impl Widget` / `show(ui, |g| { g.addon(..); g.input(..) })` handle (needs deferred layout for `order-first/last`) / TextEdit `prefix`/`suffix` atoms on Input (no interactive buttons) · pick: builder with stored closures.
- `CP-14-35` (resizable) · Handles · (a) one per gap for the whole group, `with_handle` group-wide (b) per-gap control (fixed boundaries, per-handle pill/disabled) as upstream allows · sketch: (a)
- `CP-03-7` (card) · how nested free parts learn the card size · global temp key in `ctx.data` (today) / a tag in the body Ui's `UiStackInfo` read by walking `ui.stack()` / pass size explicitly · `UiStackInfo` tag _(tagged 6)_

**G6 · A crate widget placed inside a parent: shared widget reading context, or a dedicated type**

- `CP-01-8` (button-group) · Which widgets are groupable? · Button only / Button + Input + Select trigger + ButtonGroupText + nested ButtonGroup (+ InputGroup later) · sketch: all listed
- `CP-09-2` (dropdown-menu) · One set of item widgets that picks its recipe family from the enclosing menu's `UiStack` tag, or per-component types (`DropdownMenuItem`, `ContextMenuItem`, `MenubarItem`) · shared + lookup / per component · shared + lookup.
- `CP-09-34` (menubar) · Indicator side · from the recipe insets (left in menubar, right in the others; one shared widget reads `inset_left`/`inset_right`) / per-component code · from the recipes.
- `CP-06-26` (input-group) · InputGroupText · own widget / addon `Ui` styles plain `ui.label` with the text recipe · pick: no own widget.
- `CP-06-27` (input-group) · InputGroupButton · dedicated type over `Button::extra_layer` / plain `Button` that picks layers from an `IN_INPUT_GROUP` context set by the addon `Ui` · pick: dedicated type.
- `CP-06-28` (input-group) · Allowed controls · `Input` / `Textarea` only / any widget (custom-input demo) · pick: Input and Textarea only.
- `CP-14-21` (sidebar) · Sidebar input/separator · (a) reuse `Input`/`separator` with a hook to stack an extra recipe (b) separate fns that resolve the stacked recipes · sketch: (b)
- `CP-14-27` (scroll-area) · Bar painting · (a) hide egui's bars, paint and drag our own from the output state (b) restyle egui's bars through a scoped `Style` (their handle and track colours come from `widgets.inactive` and `extreme_bg_color`, which the content shares) · sketch: (a)
- `CP-14-30` (scroll-area) · Move `select`'s list onto the new ScrollArea (`select.rs:442`) · (a) yes, in the same PR (b) later · sketch: (a)
- `CP-15-9` (calendar) · caption dropdowns · crate `Select` (needs a borderless label-style trigger) / egui `ComboBox` / label only in the first port · pick: crate `Select` with a caption trigger look.
- `CP-01-26` (toggle-group) · Per-item `variant`/`size` override (upstream `ctx.variant || variant`, group wins anyway when set): drop / keep on `ToggleItem` · sketch: drop

**G7 · Separators and groups**

- `CP-01-10` (button-group) · Separator in a group: free function `button_group_separator(ui)` / reuse `separator_vertical` with group awareness / a method on a typed group handle · sketch: free function
- `CP-12-24` (accordion) · `not-last:border-b` in immediate mode · separator above every item but the first (equivalent, needs no lookahead) / record the last item and draw after the closure · separator above every item but the first.
- `CP-13-15` (breadcrumb) · Separators: automatic between items / explicit `bc.separator()` calls as upstream (allows a different separator per position) · automatic, plus `.separator(icon)` for the whole trail.
- `CP-08-3` (select) · select groups, labels, separators · nested `SelectGroup { label, items }` + `group_separators(bool)` / flat `SelectEntry::{Item, Label, Separator}` / per-group separator flag · sketch: nested + one switch (demos differ)
- `CP-02-10` (kbd) · KbdGroup form · struct per Svelte child / closure helper `kbd_group(ui, |ui| …)` / list constructor `KbdGroup::new(&["Ctrl","B"])` · closure helper (group may hold plain text between chips)
- `CP-18-21` (bubble) · Group · `bubble_group()` reading the recipe gap vs. leave to `layout::vstack` · `bubble_group()`.
- `CP-09-16` (dropdown-menu) · Group / GroupHeading · no group container, `menu_label` for both / a `MenuGroup` container carrying the label (AccessKit `labelled_by`) · no container.
- `CP-01-11` (button-group) · `ButtonGroupText`: free function with text only / builder with `.icon` · sketch: free function, builder deferred

**G8 · Nested sub-menus**

- `CP-09-17` (dropdown-menu) · Sub-menu API · `MenuSub::new(text).show(ui, ..)` / reuse `ui.menu_button` nested (egui) with theming only · `MenuSub`.
- `CP-14-10` (sidebar) · Sub menus · (a) `menu_sub(ui, |ui|..)` container + `MenuSubButton` widget (b) a nested builder on `MenuButton` · sketch: (a)

**G9 · Text / content type of parts (batch-A X9: flagged, open)**
See also (placed in another group): CP-01-2, CP-02-1, CP-03-9, CP-04-3, CP-11-4, CP-12-3, CP-12-18, CP-13-13, CP-13-20, CP-14-22, CP-08-28, CP-16-6, CP-18-24, CP-19-2, CP-19-33.

- `CP-02-14` (kbd) · chip content · text only / text or icon (`[&_svg]:size-3`) · text or icon (`Kbd::new` / `Kbd::icon`)

**G10 · Text shortcut, closure, or both for a content-holding part**
See also (placed in another group): CP-10-9.

- `CP-18-14` (bubble) · Content · closure `show()` + text shortcut vs. text-only `impl Widget` · both (`show` and `bubble()`).
- `CP-15-10` (calendar) · custom day content (`day` snippet, custom-days demo shows a price under the number) · `day_note(Fn(Date) -> Option<WidgetText>)` / full `day_ui(FnMut(&mut Ui, &DayInfo))` / none · pick: `day_note`.
- `CP-17-10` (chart) · Tooltip value formatting · `String` from caller / `f64` + crate locale formatter (`toLocaleString`) / closure `formatter` · sketch: caller `String`.

**G11 · Media and image slot**
See also (placed in another group): CP-11-14.

- `CP-03-14` (item) · media supply · enum `ItemMedia::{Icon, Image, Ui}` / three setters `.icon`, `.image`, `.media(closure)` · three setters (Badge uses `.icon(&Icon)`, `badge.rs:43`)
- `CP-03-28` (empty) · media supply · same as CP-03-14 (`.icon`, `.media(closure)`) / enum · setters
- `CP-04-22` (avatar) · Image parameter type · `impl Into<egui::ImageSource>` (any URI/bytes/texture, needs app loaders) vs `egui::Image` vs `TextureId` only · sketch: `ImageSource`.
- `CP-04-24` (avatar) · Badge · `.badge(AvatarBadge)` builder vs `.badge(Option<&Icon>)` + `.badge_color(Color32)` on `Avatar` · sketch: `AvatarBadge` struct.
- `CP-18-3` (attachment) · Media content · typed setters `.icon/.image/.spinner` vs. a media closure (any widget) · typed setters.
- `CP-18-13` (attachment) · Image media without loaders · draw the `Image` and let egui show its loading/error placeholder vs. fall back to an icon · egui placeholder.
- `CP-04-28` (avatar) · Clickable avatar (`avatar-dropdown.svelte`) · `Avatar` with `Sense::click` / `.sense(..)` vs extend `Button` to take arbitrary content · sketch: `Avatar` senses hover only; button content is the Button cluster's choice.
- `CP-18-4` (attachment) · Trigger · `.sense(Sense::click())` (egui `Label::sense` form) vs. `.clickable(bool)` vs. a separate trigger widget placed over the card · `.sense`.

**G12 · Icon parameter form (anchor CP-01-3)**
See also (placed in another group): CP-01-3, CP-02-8, CP-02-28, CP-03-34, CP-04-31, CP-04-32, CP-06-36, CP-01-2, CP-10-26, CP-13-33, CP-14-45, CP-18-33.


**G13 · Icon slot content: `Icon` only, `Glyph`, or a separate spinner setter**
See also (placed in another group): CP-01-32, CP-02-2, CP-18-26, CP-18-35.

- `CP-01-16` (toggle) · Icon-only toggle: `.icon_only()` flag / an `Icon` size variant like `ButtonSize::Icon` (upstream toggle has none) / empty text · sketch: `.icon_only()`

**G14 · Kbd, shortcut and indicator content inside host parts**

- `CP-02-12` (kbd) · how hosts embed a Kbd (button end, tooltip, input addon, menu shortcut) · widget only / widget + `paint_at`/`desired_size` (like `Spinner::paint_at`) / `Into<Atom>` · sketch: widget + `paint_at`; atom form waits for CP-02-1
- `CP-09-8` (dropdown-menu) · Shortcut text · `.shortcut(text)` builder on items (egui `Button::shortcut_text`) / a `menu_shortcut(ui, ..)` part inside an item closure · builder.
- `CP-08-27` (command) · checked indicator · `CommandItem::checked(bool)` / leave to the caller's own icon · sketch: `checked`.

**G15 · Per-library icon slots (anchor CP-05-6; slot per shape vs per role)**
See also (placed in another group): CP-05-6, CP-06-37, CP-12-21, CP-12-33, CP-13-31, CP-14-47, CP-15-36, CP-16-41, CP-16-44, CP-16-36, CP-19-39, CP-19-42, CP-02-18.


**G16 · Accessible name and label parts of controls**

- `CP-05-2` (checkbox) · Does the checkbox carry its own label? · `impl IntoAtoms` label like `ui.checkbox(&mut b, "text")` / none, named via `Label` + `labelled_by` (crate today, upstream composition) · none.
- `CP-05-20` (radio-group) · Does an item carry its own label? · `IntoAtoms` label like `ui.radio_value(&mut v, x, "text")` / none + `labelled_by` · none (same as CP-05-2).
- `CP-05-35` (slider) · Accessible name? · `.label(String)` (today) / `labelled_by` only, as Checkbox and Switch · `labelled_by`; `.label` deprecated.
- `CP-07-2` (label) · does clicking a label act on its control (`<label for>`) · no, `labelled_by` only (today) / Label senses click, caller forwards it / controls take their own label atoms (egui `Checkbox::new(&mut b, text)`) · sketch: caller forwards
- `CP-07-7` (field) · Horizontal order of label and control · upstream varies by demo (checkbox/radio: control first; switch, choice card, buttons: label first) · options: `.control_first()`, an enum `LabelSide`, closure order only · sketch: `.control_first()`, default label first
- `CP-07-12` (field) · choice card (FieldLabel wrapping Field) · `FieldCard` container with `selected: bool` / `Field::card(selected)` modifier / not offered · sketch: `FieldCard`; open: how the card learns the inner radio's focus-visible
- `CP-07-17` (field) · FieldError role=alert · (a) AccessKit live region (Assertive), (b) plain label · sketch: (a)
- `CP-03-22` (item) · accessibility role · clickable `Button` vs `Link`; passive `Other` vs `Label` · `Button` / `Other`

**G17 · Links inside parts**

- `CP-01-6` (button) · `href` link mode (`button-as-link.svelte`): port as `.link(url)` opening via `ctx.open_url` / leave to `egui::Hyperlink` / skip · sketch: skip (caller does `ctx.open_url` on `clicked()`)
- `CP-03-21` (item) · links inside a description (`[&>a]`) · not offered / accept a `LayoutJob` and let the caller style · not offered
- `CP-04-7` (alert) · Links inside title/description (`[&_a]` underline + hover) · support `egui::Hyperlink` styling in the `.show` body vs ignore `[&_a]` · sketch: ignore (add `[&_a]` to `IGNORED`).
- `CP-12-23` (accordion) · `cn-accordion-content-inner` link/paragraph rules (`[&_a]`, `[&_p]`) · ignore (caller content) / set link and paragraph spacing on the content `Ui` style · ignore.
- `CP-13-19` (breadcrumb) · Ellipsis: static marker (upstream) / built-in collapse of the middle items into a dropdown (`.max_items(n)`, as the responsive demo hand-builds) · static marker; collapsing waits for DropdownMenu.

**G18 · Body padding, scrolling, text size and frame owned by the container**

- `CP-11-21` (sheet) · body padding · caller pads (upstream demos pass `px-4`) / the sheet pads the body with the header's padding · caller pads.
- `CP-11-22` (sheet) · scrolling a long body · caller wraps in `ScrollArea` / the sheet wraps the body in a `ScrollArea` between header and footer · caller wraps.
- `CP-12-13` (tabs) · `cn-tabs-content` text size · not applied (today) / applied to the content `Ui` in `show` / exposed as a getter like `content_gap` · applied in `show`.
- `CP-03-32` (aspect-ratio) · clip content to the box · clip (`shrink_clip_rect`) / overflow as upstream · overflow as upstream
- `CP-03-33` (aspect-ratio) · background and rounding (`rounded-lg bg-muted` in every demo) · caller paints / `.frame(Frame)` setter · caller paints
- `CP-03-27` (empty) · background (`bg-muted/30` demo) · not offered / `.fill(Color32)` · not offered
- `CP-11-32` (drawer) · inset floating card in maia/mira/luma/rhea (`before:inset-2`) · paint the panel inset by the `before` recipe (needs a `before:` translation) / ignore and draw edge-flush · paint inset.
- `CP-06-32` (input-group) · Divider between a block addon and a textarea (`class="border-t"` in the textarea demo) · offer an option / not offered (generator ignores `[.border-*]`) · pick: not offered.
- `CP-06-25` (input-group) · Addon placement naming · `AddonAlign` enum mirroring upstream / `.prefix/.suffix/.header/.footer` / `.start/.end/.top/.bottom` · pick: `AddonAlign`.
- `CP-18-9` (attachment) · Group scrolling · horizontal `ScrollArea`, hidden bar, no snap (egui has none), fade only if a scroll-fade helper exists vs. a plain wrapping row · ScrollArea, no snap. _(tagged 3)_

**G19 · Animation of parts**

- `CP-12-20` (accordion) · open/close animation (`animate-accordion-down/up`) · none, show at once (crate rule `variant.rs:225` "egui shows and hides at once") / egui openness animation as `CollapsingState` does · none.
- `CP-12-30` (collapsible) · animation · none / egui openness animation (`CollapsingState::show_body_unindented`) · none (same answer as `CP-12-20`).

**G20 · Behaviour and interaction (tagged 4, not part mapping)**

- `CP-01-13` (button-group) · Focus ring above siblings: paint the focused child's ring in a reserved later shape slot / accept overlap · sketch: reserved slot
- `CP-05-21` (radio-group) · Roving focus code? · share one helper with `tabs.rs` in `shared.rs` / copy · shared helper.
- `CP-06-19` (input-otp) · Input model · own handling of `Event::Text`/`Paste`/Backspace/arrows / a hidden `TextEdit` under painted cells (bits' model, keeps IME) · pick: own handling.
- `CP-07-28` (native-select) · Keyboard on the closed trigger: Up and Down change the value without opening (Windows/Linux) or open the list (macOS, and the crate's Select) · sketch: open the list, matching the crate's Select
- `CP-08-18` (select) · pointer model · press-drag-release selection (bits pointerdown/pointerup) / click to open, click to choose (today, egui `clicked`) · sketch: keep click (egui idiom; ComboBox does the same).
- `CP-09-15` (dropdown-menu) · How an item reports selection · `Response::clicked()` / an `on_select` closure · `clicked()`.
- `CP-16-24` (sonner) · How action callbacks reach the app · (a) events returned from `Toaster::show` (`Vec<ToastEvent>`); (b) query `action_clicked(ctx, id) -> bool`; (c) stored `Box<dyn FnOnce + Send + Sync>` closures in memory · (a)
- `CP-16-26` (sonner) · Multiple toasters (`toasterId`) · (a) one per context; (b) keyed toasters · (a)
- `CP-16-28` (sonner) · Promise toasts · (a) loading toast updated by id; (b) a poll-able handle type · (a)
- `CP-17-14` (chart) · Keyboard access to data points · none (upstream) / arrow keys walk bands and show tooltip · sketch: none, matching upstream.
- `CP-19-13` (message-scroller) · scope of the first port · full primitive (anchoring, tail spacer, prepend, opening position, visibility) / follow-bottom + scroll-to-end button + `scroll_to_message` only (on `ScrollArea::stick_to_bottom`) · full primitive sketched, subset is a valid first step (maintainer to decide)

**G21 · Generator and recipe source for parts**

- `CP-01-14` (button-group) · Generator: add a child-target ("group child") recipe with new flags (`FIRST`, `LAST`, `NOT_FIRST`, `NOT_LAST`) and single-side radius/border fields / hand-code the corner and border rules in the component with a `behaviour-ok` note · sketch: generator (SKILL.md "no style values in code")
- `CP-08-26` (command) · group heading look · shadcn-svelte inline classes (same in every style; what the PIN renders) / the style's `**:[[cmdk-group-heading]]` rules (React; uppercase, tracking in some styles) · sketch: svelte inline classes.
- `CP-17-7` (chart) · Tooltip recipe source · `cn-chart-tooltip` from the style CSS (per style; shadcn-ui uses it) / svelte's inline vega classes (same for every style) · sketch: `cn-chart-tooltip` recipe.
- `CP-17-12` (chart) · Plot style values (`[&_.lc-*]` overrides) · new hand-written `chart` constants / extend stylegen to read them / a generated `chart` recipe · sketch: hand-written constants, documented.
- `CP-02-15` (kbd) · `font-sans` in the generator · ignore with reason "the crate's text family is sans" / new font-family field · ignore _(tagged "—"; no topic file lists it)_
- `CP-02-21` (spinner) · `animate-spin` / `animate-pulse` in the generator · IGNORE with reason + `// behaviour-ok` constants / new recipe field · sketch: IGNORE + constants; the existing ANIM reason calls animation out of scope, so scope is the maintainer's call _(tagged "—"; no topic file lists it)_

**G22 · Chart scope and data**

- `CP-17-1` (chart) · How to draw the plot · optional `egui_plot` dependency behind feature `chart` / paint bar-line-area-pie in the crate / ship only tooltip + legend + palette and leave plotting to the app · sketch: optional `egui_plot` (needs maintainer approval per Dependency discipline).
- `CP-17-2` (chart) · Which chart kinds in scope · bar only (the only svelte demos) / bar+line+area / all layerchart kinds (incl. pie, radar, radial) · sketch: bar first.
- `CP-17-4` (chart) · Series colour type · `Color32` only / `{light, dark}` pair (upstream `theme`) / palette index into `chart-1..5` · sketch: enum of all three.
- `CP-17-6` (chart) · `nestLabel` rule · `chart-tooltip.svelte` nests when one series and indicator != dot; demo helper `chart-tooltip-demo-item.svelte` nests when one series and indicator == dot · sketch: follow `chart-tooltip.svelte`.

**G23 · Names**
See also (placed in another group): CP-14-46, CP-16-38.


**G24 · Table specifics**

- `CP-16-7` (table) · `colspan` · (a) `cell_span(n, ..)`; (b) not offered (the demo footer would need an empty cell layout) · (a)
- `CP-16-8` (table) · Caption position · (a) below the table (`caption-bottom`) only; (b) `.caption_top()` option too · (a)

**G25 · Flag setter form (anchor CP-01-31)**
See also (placed in another group): CP-01-31, CP-13-35.


**Moved here from another topic tag**

- `CP-13-29` (pagination) · Implementation · compose the crate's `Button` widget (variants outline and ghost) / paint with button recipes directly · compose `Button`. _(tagged 5)_

**New choice points from the topic cross-check (no sketch pick)**

- `CP-T4-1` · header parts (title, description) of multi-part components · builder setters only (dialog family, empty, attachment, message) / setters + free `x_title`/`x_description` fns (card, item, field, alert) / free fns only (popover) · idiom §Widget builders, §Containers
- `CP-T4-2` · what a multi-part container's `show` closure holds · always the main content, actions/footer as builder closures (card, empty, message, bubble, tabs, dialog, field) / the slot upstream leaves free: actions (item, attachment), footer (alert-dialog), description (alert) · idiom §Containers
- `CP-T4-3` · one meaning for the `action` setter · closure (card, alert) / `Icon` (sidebar Group, MenuButton) / text (sonner Toast) / none: actions in `show` (item, attachment) or free helpers (`alert_dialog::action`, `attachment_action`) / per component
- `CP-T4-4` · media slot names and types · `.media(closure)` + `.icon`/`.image` (item, empty) / `.media(Icon)` (alert-dialog) / `.icon`/`.image`/`.spinner`, no closure (attachment); image as `egui::Image` (item) / `impl Into<ImageSource>` (avatar, attachment) · idiom §Atom
- `CP-T4-5` · a crate widget restyled by its parent · plain widget reads a `UiStack` tag (button-group children, menu items, item in dropdown, input-group text) / dedicated type per parent (`InputGroupButton`, `sidebar::input`, `MenuSubButton`) / per component · idiom §Containers
- `CP-T4-6` · form of a layout-only group or sub-list · free fn `x_group(ui, ..)` (kbd, bubble, message, attachment groups, `menu_sub`) / struct `XGroup::new().show` (ItemGroup, ButtonGroup, MenuSub) / fn without options, struct with (CP-19-7) · idiom §Containers
- `CP-T4-7` · labelled groups of items · container carrying the label (`SelectGroup`, `cmd.group`, `sidebar::Group`, FieldSet legend) / heading part then items (`menu_label`, native-select `opt_group`) / per component · idiom §Popups
- `CP-T4-8` · return type of passive part fns (separators, labels, headings) · `Response` (egui `Ui::separator`; `item_separator`, `field::separator`, `card_title`, CP-04-16) / `()` (`button_group_separator`, `menu_separator`, `sidebar::separator`, `menu_label`) · idiom §Response methods; batch-A X8
- `CP-T4-9` · naming of free part fns and part types · prefix `component_part`/`ComponentPart` (`card_title`, `menu_label`, `SelectItem`) / module path `component::part` (`dialog::footer`, `field::description`, `sidebar::Group`) / both · batch-A report-07 Minor
- `CP-T4-10` · how an overlay or disclosure gets its trigger · any widget's `Response` (popover, hover-card, tooltip, menus, collapsible) / drawn by the component from text/atoms (menubar, accordion, navigation-menu, tabs, select) / dedicated widget (`sidebar::trigger`); `dropdown_menu` shortcut trigger: `Button` / text / `impl Widget` · idiom §Popups
- `CP-T4-11` · content form of content-holding components · text shortcut + `show` closure (tooltip, bubble, alert) / text only (marker, badge, kbd) / closure only (hover-card, popover) · idiom §Widget builders, §Text
- `CP-T4-12` · scroll area inside composites (select, command, message-scroller, sheet body) · the crate's shadcn `ScrollArea` (CP-14-30) / raw `egui::ScrollArea` / caller wraps (CP-11-22) · idiom §ScrollArea; batch-B Y15
- `CP-T4-13` · how a crate `Icon` enters `IntoAtoms` content (tab triggers, nav items, alert-dialog buttons, native-select option) · `Atom::custom` placeholder painted by the component / `impl From<Icon>` into an atom / icons only via `.icon(..)` · idiom §Atom; batch-B Y14

### T5 · Container: `ui.add` or `show`, return type

**G1 · What does an in-flow container's `show` return?**

- `CP-03-1` (card) · what does `show` return · `R` (today) / `InnerResponse<R>` · `InnerResponse<R>`
- `CP-04-2` (alert) · Return type of the container form · `InnerResponse<R>` (egui) vs `R` (crate's `Card::show`, `card.rs:99`) · sketch: `InnerResponse<R>`.
- `CP-18-10` (attachment) · Return type · `InnerResponse<R>` vs. a custom `AttachmentResponse { response, actions }` · `InnerResponse<R>`.
- `CP-12-19` (accordion) · item return type · egui `CollapsingResponse<R>` / own `AccordionItemResponse<R>` / `Option<R>` · `CollapsingResponse<R>`.
- `CP-14-40` (resizable) · Return type · (a) `InnerResponse<Vec<R>>` (b) output struct with layout and `changed` · sketch: (b)
- `CP-06-29` (input-group) · Return value · control's `Response` / `InnerResponse` with addon results / group-frame `Response` · pick: control's `Response`.

**G2 · What does an overlay's `show` return? (anchor CP-10-3)**
See also (placed in another group): CP-10-25, CP-10-2.

- `CP-10-3` (popover) · return type of `show` · `Option<R>` (today) / `Option<InnerResponse<R>>` (egui `Popup::show`) · `Option<InnerResponse<R>>`.
- `CP-09-3` (dropdown-menu) · `DropdownMenu::show` return · `Option<R>` (crate Popover) / `InnerResponse<Option<R>>` (`menu_button`) / `Option<InnerResponse<R>>` (`Popup::show`) · sketch: `Option<R>`; follows CP-10-3
- `CP-11-2` (dialog) · return type of `show` · `Option<R>` (today) / `Option<InnerResponse<R>>` (Window-like, gives the panel Response) / a `DialogResponse { inner, response, closed }` · `Option<InnerResponse<R>>`, the same as CP-10-3 for anchored overlays; it follows CP-10-3's outcome.
- `CP-13-32` (navigation-menu) · return of `nav.item` · `InnerResponse<Option<R>>` (`menu_button`) / `Option<InnerResponse<R>>` (CP-10-3) · sketch: `InnerResponse<Option<R>>` (trigger response needed when closed); follows CP-10-3 if CP-10-25 says so

**G3 · Leaf (`ui.add`), container (`show`), or both, for one component**
See also (placed in another group): CP-12-4, CP-18-14.

- `CP-03-12` (item) · container form · `impl Widget` only / `show(ui, actions)` only / both · both
- `CP-07-6` (field) · Field entry points · `add(ui, widget) -> Response` (auto `labelled_by`), `show(ui, closure) -> InnerResponse<R>`, or both · sketch: both
- `CP-04-15` (separator) · Builder struct + `impl Widget` vs free functions only (current crate) · sketch: both (struct plus the two shortcuts).

**G4 · Shape of a group or collection: data leaf or closure container**
See also (placed in another group): CP-05-8.

- `CP-08-29` (command) · container shape · `show(ui, closure) -> InnerResponse<R>` / `Widget` with a data slice · sketch: `show`.
- `CP-19-7` (message) · Group form · free function `message_group` / `MessageGroup::new().show(..)` builder · free function (no options upstream)

**G5 · Own type, free layout function, or reuse egui's container**

- `CP-03-29` (aspect-ratio) · form and home · `AspectRatio` builder in `components` / free `layout::aspect_ratio(ui, ratio, add) -> R` / not ported (point to `egui::Image::maintain_aspect_ratio`, `fit_to_exact_size`) · `AspectRatio` builder
- `CP-14-26` (scroll-area) · Shape · (a) a `shadcn_egui::ScrollArea` mirroring egui's builder (b) no new type: the theme sets `style.spacing.scroll` and visuals so every egui `ScrollArea` looks shadcn (c) a `scroll_area(ui, h, |ui|..)` helper · sketch: (a)

**G6 · Richer output than `Response` from a leaf widget**

- `CP-06-3` (input) · Return richer output? · `impl Widget` only / plus `show(ui) -> TextEditOutput` · pick: both.
- `CP-06-16` (input-otp) · Completion signal · caller checks `changed() && len == n` / `show() -> { response, completed }` / callback · pick: `show` output field.
- `CP-19-28` (questionnaire) · submit signal · `output.submitted: bool` / a callback / `response.clicked()` on submit · `submitted: bool`
- `CP-19-17` (message-scroller) · output contents · `scrollable` + `visible` + `current_anchor` always computed / visibility only when asked (`.track_visibility(true)`, upstream arms it lazily) · always computed (cheap in egui: item rects are known)

**G7 · Entry point: first argument, builder vs extension, method name**
See also (placed in another group): CP-09-23.

- `CP-11-3` (dialog) · first argument · `&Context` (floating-container idiom, today) / `&mut Ui` (calls `ui.ctx()`) · `&Context`.
- `CP-10-14` (hover-card) · entry point · builder `HoverCard::new(&resp).show(..)` / `Response` extension `resp.on_hover_card(|ui| ..)` (egui `on_hover_ui` style) · builder, same shape as `Popover`.
- `CP-09-28` (menubar) · Bar method name · `show` (crate containers) / `ui` (egui `MenuBar::ui`) · `show`.

**G8 · Mis-tagged (not a container question)**
Placed under the topic their question belongs to: CP-04-29, CP-13-29.


**New choice points from the topic cross-check (no sketch pick)**

- `CP-T5-1` · carousel `show` return · `Response` with `changed()` (sketch) / `InnerResponse<()>` / output struct like `ResizableOutput` (CP-14-40 b) / `InnerResponse<Vec<R>>` · idiom §Containers
- `CP-T5-2` · crate-wide in-flow container return (CP-03-1, CP-04-2, `card_content`, `sidebar::inset`, `menu_sub`, `kbd_group`, `dialog::footer`) · `InnerResponse<R>` (egui `Frame`) / `R` (crate today) / own output struct only where extra data exists · idiom §Containers
- `CP-T5-3` · `CommandDialog::show` return · `Option<R>` (command sketch) / `Option<InnerResponse<R>>` as Dialog (CP-11-2, CP-10-3) · idiom §Popups
- `CP-T5-4` · container method name where egui's model uses another · always `show` (CP-09-28 pick) / egui's name: `show_inside` (sidebar), `show_ui` (native-select), `ui` (MenuBar) · idiom §Containers
- `CP-T5-5` · return of shortcuts attached to a trigger `Response` · the `Response` for chaining (tooltip, collapsible_trigger; egui `on_hover_ui`) / `Option<R>` (context_menu) / `Option<InnerResponse<R>>` (CP-10-3) / `InnerResponse<Option<R>>` (dropdown shortcut, `menu_button`) · idiom §Response methods
- `CP-T5-6` · how a secondary part's `Response` (action, badge) reaches the caller · field on an output struct (sidebar `MenuButtonOutput`, `GroupOutput`, CP-14-9, CP-18-10 option) / actions closure + `InnerResponse<R>` (item, attachment, alert, card) · no egui idiom
- `CP-T5-7` · content-less form of a component that also takes content · `ui.add(X)` plus `X.show(..)` (item, empty, alert, attachment, tabs, field) / free text shortcut + `show` (bubble, tooltip) / `show` only (card, message, button-group) · idiom §Widget builders, §Containers
- `CP-T5-8` · when a container is a free fn · when it has no options (`message_group`, `kbd_group`, `sidebar::inset`, `menu_sub`, `dialog::footer`) / always a builder with `show` (Card, ButtonGroup, RadioGroup, `sidebar::Group`) / both · idiom §Containers (`ui.horizontal` beside `Frame::show`)
- `CP-T5-9` · suffix of the crate's output structs · `*Output` (InputOtp, Questionnaire, Resizable, MessageScroller; egui `TextEditOutput`) / `*Response` (Attachment, AccordionItem, Dialog options; egui `CollapsingResponse`) / `*Output` for leaves, `*Response` for containers · idiom §Containers
- `CP-T5-10` · what message-scroller wraps · raw `egui::ScrollArea` + own `MessageScrollerOutput<R>` / the shadcn `ScrollArea` of CP-14-26 · idiom §ScrollArea; batch-B Y15
- `CP-T5-11` · return of leaf part fns · `()` (`sidebar::separator`, `card_header` today) / `Response` (egui `Ui::separator`, CP-04-16, `card_title`) · idiom §Widget builders; batch-B report-14 Minor

### T6 · Identity

**G1 — Root id of a stateful widget: required at construction, optional setter, or auto only**

- `CP-05-12` (radio-group) · Group id? · explicit `id_salt` in `new` / `ui.next_auto_id()` · explicit `id_salt`.
- `CP-08-10` (select) · identity · `id: &str` (today) / `id_salt: impl Hash` in `new` (like `Dialog::new`) / auto id + `.id_salt` setter · sketch: `id_salt` in `new`.
- `CP-07-32` (native-select) · identity of `NativeSelect` · required `id_salt` (`from_id_salt`, like `ComboBox`) / `new()` + optional `.id_salt` / `ui.next_auto_id()` · sketch: required; follows CP-05-12, CP-08-10
- `CP-09-30` (menubar) · Bar identity · `.id_salt(..)` with auto default / always auto (`next_auto_id`) / required id in `new` · optional `id_salt`.
- `CP-12-11` (tabs) · id · auto id only (today) / `.id_salt(impl Hash)` · add `.id_salt`.
- `CP-15-12` (calendar) · identity · optional `id_salt` (egui) / required id in `new` like crate `Select::new(id, …)` · pick: optional `id_salt`.
- `CP-18-11` (attachment) · Identity · auto id + `.id_salt()` vs. id derived from the title · auto id + `.id_salt()`.

**G2 — Overlay id: derived from the trigger or explicit**

- `CP-10-17` (popover) · overlay identity · derived from trigger id only / plus `.id_salt` / explicit `Id` in `new` · derived + optional `.id_salt`.

**G3 — Modal id source and scope**

- `CP-11-8` (dialog) · id source · `id_salt` hashed globally (today) / `Id` argument like `Modal::new(id)` / title as id like `Window::new` · `id_salt`.

**G4 — Legacy name**

- `CP-06-8` (input) · Remove the deprecated `id_source` now (pre-1.0 cleanup) or keep it · pick: remove.

**G5 — Tagged 6 but not about identity**
Placed under the topic their question belongs to: CP-03-7, CP-16-10, CP-16-23.


**Related, tagged with another topic**
Placed under the topic their question belongs to: CP-19-11.


**Moved here from another topic tag**

- `CP-19-11` (message-scroller) · item identity type · `impl Hash` → `Id` / `&str` like upstream `messageId` / index · `impl Hash` _(tagged 1)_

**New choice points from the topic cross-check (no sketch pick)**

- `CP-T6-1` · crate-wide anchor: root id of a stateful component · required `id_salt` in the constructor (radio-group, native-select, select, command, modals, navigation-menu, sidebar, resizable, table, carousel, chart, message-scroller) / auto id + optional `.id_salt` (button-group, toggle-group, inputs, menubar, popover, hover-card, tabs, accordion, pagination, scroll-area, calendars, attachment, questionnaire) / auto only (leaves) / required where egui requires one (`ComboBox`, `Area`, `Modal`, `Panel`) · subsumes CP-05-12, CP-08-10, CP-07-32, CP-09-30, CP-12-11, CP-15-12, CP-18-11, batch-A X9 · idiom §IDs
- `CP-T6-2` · id parameter type · `id_salt: impl Hash` (all but sidebar) / `impl Into<Id>` (sidebar, egui `Panel::left`) / `Id` (static helpers, `Modal::new`) / domain newtype (sonner `ToastId`) · idiom §IDs
- `CP-T6-3` · salt scope · global `Id::new(salt)` (modals, sidebar) / parent-scoped `ui.make_persistent_id` (select, resizable, message-scroller, calendars, inputs) / global for floating and app-level, scoped for in-flow · idiom §IDs
- `CP-T6-4` · how app code addresses a component's state from outside · same value as the constructor (sidebar) / derived `Id` in the output (message-scroller) / trigger widget's id (popover) / returned handle (sonner `ToastId`) / not offered (modals today); depends on CP-T6-3 · idiom §Open state
- `CP-T6-5` · popup id of a trigger-plus-list widget · from a required salt (select, native-select, command; `ComboBox::from_id_salt`) / derived from the trigger `Response` (menus, popover, hover-card, tooltip) / derived + optional `.id_salt` (CP-10-17) · idiom §Popups
- `CP-T6-6` · name of the id-taking constructor · `new(id_salt, ..)` (most) / `from_id_salt(salt)` (native-select) / variant-named with id (`Sidebar::left(id)`, `Resizable::horizontal(id_salt, ..)`) / free fn returning a foreign builder (`chart_plot(id_salt)`) · idiom §IDs
- `CP-T6-7` · item ids inside data-driven containers · index (toggle-group, tabs, accordion, navigation-menu, resizable, slider thumbs, questionnaire) / value hash or caller key (command, message-scroller CP-19-11, sonner, attachment) / auto (radio-group, breadcrumb, sidebar, input-group); tag form `id.with(("tab", i))` vs `id.with(i)` · idiom §IDs
- `CP-T6-8` · `layout::row` keys stored height by `ui.next_auto_id()` (call order) · keep / add `.id_salt` or an id argument (CP-T6-1) / drop the stored state · idiom §IDs

### T7 · Orientation and direction

**G1 · Orientation type (crate-wide anchor CP-05-37)**
See also (placed in another group): CP-05-37, CP-01-30.

- `CP-01-9` (button-group) · orientation type · per-widget `ButtonGroupOrientation` / one shared `Orientation` · sketch: per-widget; follows anchor CP-05-37
- `CP-04-20` (separator) · orientation type · methods `.horizontal()`/`.vertical()`, private `Option<bool>` / shared `Orientation` (CP-05-37) · sketch: methods; follows CP-05-37
- `CP-07-29` (field) · Field orientation type `FieldOrientation { Vertical, Horizontal, Responsive }` · per-widget enum / shared `Orientation` (CP-05-37) + separate responsive flag / shared enum with `Responsive` · sketch: per-widget; follows CP-05-37
- `CP-12-5` (tabs) · tabs orientation type · per-component `TabsOrientation` + `.vertical()` / one crate `Orientation` / methods `.horizontal()`/`.vertical()` · sketch: per-component; follows CP-05-37; bool form CP-12-31
- `CP-14-36` (resizable) · resizable axis · `horizontal()`/`vertical()` constructors (as `ScrollArea`) / `.direction(Direction)` / shared `Orientation` (CP-05-37) · sketch: constructors; follows CP-05-37
- `CP-16-40` (carousel) · Orientation type · per-widget `CarouselOrientation` enum (sketch) / the one shared crate `Orientation` of CP-05-37 / only a `.vertical()` setter, no enum · sketch: per-widget enum with `.vertical()`; follows the outcome of CP-05-37 (crate-wide anchor)
- `CP-18-5` (attachment) · attachment orientation type · per-widget `AttachmentOrientation` + `.vertical()` / shared `Orientation` (CP-05-37) / bool · sketch: per-widget; follows CP-05-37

**G2 · Boolean setter form of the orientation shortcut (anchor CP-01-31)**
See also (placed in another group): CP-01-31, CP-04-33, CP-12-31, CP-18-34, CP-07-30, CP-16-42.


**G3 · Default orientation**

- `CP-04-17` (separator) · Default orientation · infer from the `Ui` layout (egui `Separator`, `separator.rs`) vs always horizontal (bits-ui default) · sketch: infer when not set, `separator(ui)` stays horizontal.

**G4 · Offer an orientation or layout mode upstream has but shadcn does not style or demo**

- `CP-05-13` (radio-group) · Orientation? · `.orientation(..)`/`.horizontal()` that lays out and maps keys / none (bits-ui takes both axes; no class reads `data-orientation`) · none.
- `CP-13-6` (navigation-menu) · Support `orientation = vertical` (keyboard axis, entry key ArrowRight)? · horizontal only / `NavigationMenuOrientation` enum + `.vertical()` (Slider idiom) · horizontal only.
- `CP-13-25` (pagination) · Vertical orientation (arrow axis only) · omit / `.vertical()` · omit.
- `CP-08-30` (command) · grid mode (`columns`) · omit / support · sketch: omit (no shadcn demo uses it).
- `CP-18-28` (marker) · Stacked (`flex-col`, demo-only) · `.vertical()` vs. leave out · leave out.

**G5 · Keyboard axis and roving focus**
See also (placed in another group): CP-05-29.

- `CP-16-17` (carousel) · Keyboard target · (a) Svelte: Left/Right only while a prev/next button has focus; (b) React: Left/Right while anything inside has focus; (c) also Up/Down when vertical · (b)
- `CP-01-29` (toggle-group) · Shared roving-focus helper extracted from `tabs.rs` for ToggleGroup (and later RadioGroup, Toolbar) / per-component copy · sketch: shared helper

**G6 · Edge side of panels**
See also (placed in another group): CP-10-1.

- `CP-11-18` (sheet) · side selection API · `.side(Side::Left)` enum with default Right / constructors `Sheet::left(id, title)` like `egui::Panel::left` / `RectAlign` · `.side(Side)`.
- `CP-11-19` (sheet) · `Side` type scope · one `Side` for edge panels (Sheet, Drawer, Sidebar CP-14-13) / per-component enums / reuse `RectAlign` · sketch: one `Side` for edge panels; anchored overlays keep `RectAlign` (CP-10-1)
- `CP-14-13` (sidebar) · sidebar side · `Sidebar::left/right` constructors (Panel idiom) / `.side(Side)` with the edge `Side` of CP-11-19 · sketch: constructors; should match CP-11-18, CP-11-19 (batch-B N3)
- `CP-11-34` (drawer) · default side · Bottom (vaul) / Right (Sheet) · Bottom.

**G7 · Start/End alignment of chat parts**

- `CP-18-15` (bubble) · bubble align type · own `BubbleAlign` / one type shared with Message (CP-19-8) / `egui::Align` (has `Center`) / `.align_end()` · sketch: `BubbleAlign`; decide with CP-19-8
- `CP-19-8` (message) · message `align` type · `MessageAlign` / one enum shared with Bubble (CP-18-15) / `bool end` / `egui::Align` · sketch: `MessageAlign` + `.end()`; one decision with CP-18-15 (Y8)
- `CP-18-16` (bubble) · bubble inherits alignment from an enclosing Message · Message pushes align into the child `Ui`, `.align()` overrides / caller passes to both / always explicit · sketch: pushed (same as CP-19-6)
- `CP-19-6` (message) · how Bubble learns the message alignment · Message pushes `MessageAlign` into the child `Ui`, `Bubble::align()` overrides / caller passes to both / always explicit · sketch: pushed (one decision with CP-18-16)

**G8 · Breakpoint-driven direction changes**
See also (placed in another group): CP-07-29.

- `CP-07-14` (field) · Responsive threshold source · (a) available width of the Field at layout (448 pt = `@md` 28rem), (b) the enclosing FieldGroup's width passed down through `Ui` data · sketch: (a). The porting standard bans `available_width()` arithmetic, so this needs an explicit exception.
- `CP-15-13` (calendar) · several months · always side by side / wrap to a column when narrow (`md:` breakpoint) · pick: side by side.

**G9 · Child order and row/column composition of parts**
See also (placed in another group): CP-07-7, CP-03-3.


**G10 · Overlay arrow sides**

- `CP-10-23` (tooltip) · arrow for left/right sides · top/bottom only (today) / all four sides · all four, with `align`.

**G11 · Locale direction**

- `CP-15-14` (calendar) · week start default · Sunday (en-US, upstream) / Monday (ISO) / from a locale setting · pick: Sunday, `week_start(..)`.

**G12 · Mis-tagged (not about orientation)**
Placed under the topic their question belongs to: CP-06-23, CP-06-31, CP-19-36.


**Moved here from another topic tag**

- `CP-19-15` (message-scroller) · `scroll_to_message` align type · `egui::Align` (Min/Center/Max, no `nearest`) / own enum with `Nearest` as upstream / `Option<egui::Align>` like `Response::scroll_to_me` (None = nearest) · `egui::Align` in the sketch; `Option<Align>` would cover `nearest` _(tagged 10)_

**New choice points from the topic cross-check (no sketch pick)**

- `CP-T7-1` · orientation setter surface · `.orientation(E)` + shortcut for the non-default value (button-group, slider, tabs, carousel, attachment; field `.horizontal()`) / shortcut only (toggle-group) / one method per value (separator) / constructors per value (resizable, scroll-area); does the shortcut always name `vertical` · idiom §Direction and layout, §Slider
- `CP-T7-2` · default orientation or alignment from context · fixed per widget / inferred from the enclosing `Ui` layout (separator CP-04-17, egui `Separator`) / pushed by a parent via `Ui` data or `UiStack` tag, setter overrides (bubble from message CP-18-16/CP-19-6; CP-01-10 option) · idiom §Direction and layout
- `CP-T7-3` · arrow keys in vertical orientation (shared roving helper, CP-01-29) · keys follow the axis (tabs, toggle-group, accordion) / both axes always (radio-group CP-05-13) / Up/Down always, Left/Right only horizontal (slider CP-05-29) / Left/Right in both (carousel CP-16-17) · idiom §Focus and keyboard
- `CP-T7-4` · Start/End alignment type · per-widget enums (`MessageAlign`, `BubbleAlign`, `BubbleReactionsAlign`, `ScrollEdge`, `CarouselAlign`) / one shared `{Start, End}` (CP-19-8/CP-18-15 option) / `egui::Align` (has `Center`; carousel needs it) / no-argument shortcut (`Message::end()`) · idiom §Direction and layout
- `CP-T7-5` · placement names in an LTR-only crate · logical (`InlineStart`, `Start`/`End`: input-group, message, bubble, scroller, carousel) / physical (`Left`/`Right`/`Top`/`Bottom`: sheet, drawer, sidebar, bubble reactions) / egui types where they fit (`RectAlign`, `Align2`, `Align`) · idiom §Anchoring
- `CP-T7-6` · responsive breakpoints that switch direction · evaluate against available width (field `Responsive`, CP-07-14) / treat as always true (calendar `md:`, dialog `sm:`) / drop the narrow branch (sidebar mobile CP-14-12) / caller flag (`.mobile(bool)`, `.responsive()`) · no egui idiom; porting standard bans `available_width()` arithmetic
- `CP-T7-7` · footer direction and override · fixed per component as upstream (card, dialog, alert-dialog, sheet) / `footer_layout(Layout)` setter on every footer (CP-03-3 option) / caller nests a layout in the footer closure · idiom §Containers, §Direction and layout
- `CP-T7-8` · ScrollArea axis form under CP-05-37 · egui constructors `vertical()`/`horizontal()`/`both()` (sketch) / shared `Orientation` + a both-axes form / `Vec2b` · idiom §ScrollArea
- `CP-T7-9` · chart bar orientation and `seriesLayout` (absent from the sketch) · not offered / left to egui_plot / crate setter with the CP-05-37 type · no egui idiom
- `CP-T7-10` · `Weekday` type for `Calendar::week_start` (used, not defined; batch-B Minor) · own enum / `jiff::civil::Weekday` / `chrono::Weekday`; follows CP-15-1 · no egui idiom

### T8 · Sizing

**G1 · Width setter: which name and which meaning (fixed / cap / minimum / none)**

- `CP-01-5` (button) · Width control for stretching in a vertical group or form: `.min_width(f32)` (egui `Button::min_size`) / `.width(f32)` like `Tabs::width` / none · sketch: `.min_width`
- `CP-03-4` (card) · width · always available width (today) / `.max_width(f32)` / `.width(f32)` · `.max_width(f32)`, default fills
- `CP-03-30` (aspect-ratio) · width source · available width / `.max_width(f32)` / `.width(f32)` · available width plus `.max_width`
- `CP-04-5` (alert) · Width · fill available width (upstream `w-full`) with `.max_width` setter vs no setter (caller's layout) · sketch: fill, plus `.max_width(f32)` since every demo sets `max-w-md`.
- `CP-04-11` (progress) · Width setter name · `.width` (crate `Select::width`) vs `.desired_width` (egui `ProgressBar`) · sketch: `.width`.
- `CP-05-36` (slider) · Width? · fill only (today) / `.width(f32)` like `Select::width`, `Tabs::width` · add `.width`, default fill.
- `CP-06-4` (input) · Width API · `max_width(f32)` with NaN sentinel (today) / `max_width` stored as `Option<f32>` / `desired_width` (TextEdit name) / `width` (Select's name, `select.rs:97`) · pick: `max_width`, stored as `Option<f32>`.
- `CP-07-11` (field) · Width control · (a) none (fills width; caller limits), (b) `.max_width(f32)` on FieldSet/FieldGroup · sketch: (a)
- `CP-08-11` (select) · sizing · `width` only (today) / `width` + `max_width` (as `Input::max_width`) · sketch: both.
- `CP-10-18` (popover) · width knobs · `.width` only / `.width` + `.max_width` · `.width` only for popover and hover-card.
- `CP-18-6` (attachment) · Width · `.width(f32)` vs. a full-width flag vs. caller `ui.add_sized` · `.width(f32)`.
- `CP-09-31` (menubar) · Bar width · content width (upstream `flex`) / full available width (egui `MenuBar` `set_min_size(available_width)`) · content width + `.width()`.
- `CP-09-6` (dropdown-menu) · Content width · `max(trigger width, min-w)` as upstream `w-(--bits-…-anchor-width)` / content width with `min-w` only · upstream rule, `.width()` override.

**G2 · Size-variant type: per-component enum or shared enum**

- `CP-05-24` (switch) · Size type naming across the crate? · per-widget enums (`SwitchSize`, `SelectSize`) / one shared `Size` enum · per-widget enums (variants differ per component).
- `CP-11-15` (alert-dialog) · size type · per-component `AlertDialogSize` (like `CardSize`) / one shared `Size` enum across components · per-component enum.
- `CP-07-23` (native-select) · Size enum · reuse `SelectSize` vs a new `NativeSelectSize` · sketch: re-export `SelectSize`
- `CP-06-35` (input-group) · `InputGroupButtonSize::Sm` in styles without `cn-input-group-button-size-sm` (5 of 7 exported) · offer `Sm` (falls back to `cn-button-size-default` + group button base) / drop `Sm` · pick: offer it.

**G3 · Pixel-size API for leaf widgets with no variants**

- `CP-02-16` (spinner) · size API · `.size(f32)` like `egui::Spinner` / a size enum for the demo steps 3/4/6/8 · `.size(f32)`
- `CP-02-23` (skeleton) · size API · `new(Vec2)` / `new()` + `.width`/`.height` / `ui.add_sized` only · `new(impl Into<Vec2>)`
- `CP-02-24` (skeleton) · full width (`w-full`, `flex-1`) · `f32::INFINITY` width (TextEdit idiom) / `.full_width()` setter / caller computes · `f32::INFINITY`
- `CP-11-20` (sheet) · size override · one `size(px)` meaning width or height by side / separate `width` and `height` / `max_height` only for Top/Bottom · `size(px)`.

**G4 · Recipe-dimension overrides (cell, slot, spacing, panel widths)**

- `CP-03-5` (card) · `--card-spacing` override (`card-spacing` demo) · not offered / `.spacing(f32)` · not offered
- `CP-06-22` (input-otp) · Slot size override · none (recipe only) / `slot_size(Vec2)` / plus text size · pick: `slot_size` only.
- `CP-15-15` (calendar) · cell size · style's `--cell-size` only / `cell_size(px)` override (demos use 38, 40, 48 px) · pick: override.
- `CP-14-14` (sidebar) · 16rem/3rem/18rem widths · (a) behaviour constants (b) builder setters defaulting to them · sketch: (b)
- `CP-14-31` (scroll-area) · Hide delay 600ms and min thumb 18px · (a) behaviour constants (b) builder setters · sketch: (a) with `// behaviour-ok`
- `CP-04-18` (separator) · Explicit length (`data-vertical:h-4` in sidebar blocks, `w-full` override) · `.length(f32)` setter vs caller wraps in `ui.allocate_ui` · sketch: not offered until a block needs it.
- `CP-01-25` (toggle-group) · `spacing` unit: points (`f32`) / Tailwind units like upstream (`spacing={2}` = 8px) · sketch: points
- `CP-15-16` (calendar) · Calendar and RangeCalendar spacing differ (calendar: month `gap-4`, grid no row gap; range: grid `mt-4 gap-1`; range root hard-codes `p-3`, beating nova/lyra `p-2`) · port each as written / unify on Calendar · pick: unify on Calendar's classes (one widget, CP-15-2).
- `CP-13-7` (navigation-menu) · Viewport padding: bits sizes the viewport to the content plus 1rem (the shadcn `parity-ignore` note), while Radix uses the exact size · the shadcn-svelte +1rem / the exact content size · +1rem (follow the pin).

**G5 · Height model and who provides a height**

- `CP-03-26` (empty) · height (`flex-1`, `h-full` in `empty-background-demo`) · content height / fill available height / `.min_height(f32)` · content height
- `CP-06-10` (textarea) · Height model · grow with content from `min-h-16` (upstream `field-sizing-content`) / fixed `rows(n)` with inner scroll · pick: grow, `rows` raises the minimum.
- `CP-06-11` (textarea) · User resize handle (present in vega/nova/lyra via browser default) · none / egui `Resize` · pick: none.
- `CP-14-41` (resizable) · Group size · (a) take the available rect (b) `.height(f32)` / `.width(f32)` setters · sketch: (a) plus optional `.height`
- `CP-14-15` (sidebar) · Drag-resize like egui `Panel::resizable` (upstream has none) · (a) no (b) opt-in · sketch: (a)

**G6 · Size or stretch propagated through a group**

- `CP-01-12` (button-group) · Vertical group stretching: implement `items-stretch` (previous-pass width) / leave children at own width · sketch: implement
- `CP-03-17` (item) · group gap depends on child size (`has-data-[size=*]`) · explicit `ItemGroup::size` / read from first child via memory (one frame late) / free `item_group(ui, size, ..)` · `ItemGroup` builder with explicit size
- `CP-04-27` (avatar) · Group-count size · follows the group's (first / largest) avatar size vs its own `.size` · sketch: follows the largest avatar in the group (with mixed sizes both `group-has-data-[size=lg]` and `…sm` match upstream; the winner is CSS source order, unsure).
- `CP-06-33` (input-group) · Who owns the width · group's `max_width` (control's ignored) / control's `max_width` · pick: group's.

**G7 · Text overflow in a constrained width**

- `CP-07-3` (label) · Does a long label wrap? · (a) never (today, `layout_no_wrap`), (b) wrap at the available width like `egui::Label`, (c) `.wrap(bool)` · sketch: (b)
- `CP-03-20` (item) · line clamp · clamp title to 1 row, description to 2 (`LayoutJob` `wrap.max_rows`) / wrap freely · clamp as upstream
- `CP-13-17` (breadcrumb) · Per-item truncation (`max-w-20 truncate`, responsive demo only) · `.max_item_width(f32)` with elide / leave it to the caller via `bc.item` · leave it to the caller (demo-only value, per SKILL.md rule 5).

**G8 · Window-relative and layout extent**

- `CP-16-37` (sonner) · Narrow windows (< 600 px) · (a) port the full-width mobile layout; (b) always 356 px · (a)
- `CP-18-20` (bubble) · `max-w-[80%]` · a layout helper / recipe field for a fraction of the parent width vs. `.max_width(f32)` the caller passes vs. hard 80 % in code (needs `behaviour-ok`) · recipe field + layout helper.
- `CP-13-26` (pagination) · Centring: upstream root is `mx-auto w-full justify-center`, and the icons-only demo overrides it with `mx-0 w-auto` · centre by default with `.centered(false)` / never centre (caller lays out) · never centre: egui widgets take their own size, and the caller centres.
- `CP-16-2` (table) · table layout engine · own layout, column widths from the previous frame (like `Grid`) / `egui::Grid` (no colspan, no row hover) / `egui_extras::TableBuilder` (new dependency) · sketch: own layout
- `CP-16-3` (table) · Column sizing API · (a) declared up front (`Column::auto/exact/remainder`); (b) inferred from content only, as HTML does, extra width shared proportionally · (a)
- `CP-16-4` (table) · Who provides `overflow-x-auto` · (a) Table wraps itself in a horizontal `ScrollArea`; (b) caller wraps; (c) option `.scroll(bool)`, default on · (a)

**G9 · Tagged 8 but asking another topic's question**
Placed under the topic their question belongs to: CP-02-25, CP-13-35, CP-01-31, CP-01-16, CP-01-26, CP-03-7, CP-14-34.

- `CP-02-7` (badge) · custom colours (`badge-colors.svelte` overrides bg/text) · none / `.fill(Color32)` + `.text_color(Color32)` like `egui::Button::fill` · both setters
- `CP-14-25` (sidebar) · Sidebar colours: add the eight `sidebar-*` roles to `Palette` + stylegen, or map them onto existing roles · sketch: add the roles
- `CP-16-29` (sonner) · Look source · (a) hard-code svelte-sonner's CSS values as `// behaviour-ok` / `// token-ok` literals plus the `cn-toast` recipe for radius and the popover/border roles; (b) extend stylegen with a hand-written "sonner" recipe set; (c) put the values in `Metrics` · (a)
- `CP-16-30` (sonner) · Description colour · (a) svelte-sonner literal `#3f3f3f` / `hsl(0,0%,91%)`; (b) `muted-foreground` role · (a), flagged: the literal ignores the palette
- `CP-03-25` (empty) · outline (`border border-dashed`, `empty-outline-demo`) · not offered / `.outline(bool)` (dashed, 1px, Border role) · `.outline(bool)`; the 1px width has no recipe source (caller class), needs a decision on where it comes from
- `CP-07-26` (native-select) · Placeholder · (a) none: a `value=""` option is an ordinary option (upstream), (b) `.placeholder(text)` as Select has · sketch: (a)
- `CP-16-18` (carousel) · Drag behaviour · (a) drag with snap on release (by velocity and distance, own constants); (b) buttons and keys only, no drag; (c) drag-free (`dragFree`) as an option · (a)
- `CP-16-19` (carousel) · Which embla options to expose · (a) `align`, `loop`, `per_view`, `spacing`; (b) also `slides_to_scroll`, `drag_free`, `start_index`; (c) none beyond orientation · (a)
- `CP-16-21` (carousel) · Slide animation · (a) animate the offset (`ctx.animate_value_with_time`); (b) jump · (a)
- `CP-16-35` (sonner) · Animation (slide in, height, fade out) · (a) animate with `ctx.animate_value_with_time`, 400 ms; (b) appear and vanish at once, like the crate's Dialog · (a)
- `CP-19-19` (message-scroller) · prepend correction timing in a single pass · correct next frame from stored item rects (one-frame jump) / `ctx.request_discard` to redo the frame · next frame + `request_discard`

**Moved here from another topic tag**

- `CP-04-29` (avatar) · Inner border overlay (`after:border … mix-blend-darken/lighten`) · paint the `_after` border without blending vs skip it · sketch: paint it without blending (egui has no blend modes). _(tagged 5)_

**New choice points from the topic cross-check (no sketch pick)**

- `CP-T8-1` · crate-wide width setter: one name and meaning · `.width` = fixed (select, native-select, slider, progress, tabs, menus, popover, hover-card, dialog, attachment, sidebar) / `.max_width` = cap over fill (card, aspect-ratio, alert, input, textarea, input-group, command, scroll-area) / both (select) / `.min_width` (button) / `.desired_width` (egui TextEdit; CP-04-11, CP-06-4) / none, caller layout · idiom §Widget builders (`add_sized`), §TextEdit; batch-A X9
- `CP-T8-2` · default width of text-entry and choice controls · cap at `Metrics::field_max_width` (input, input-group) / fill (native-select, field, command) / widest option (select) / `max_width` only when set (textarea) · idiom §TextEdit, §Selection
- `CP-T8-3` · how a caller asks for full width · fill is default, `.width` narrows (progress, slider, card, alert) / `f32::INFINITY` (skeleton CP-02-24; TextEdit idiom) / `.full_width()` flag (CP-02-24, CP-18-6 options) / caller `ui.add_sized` · idiom §Widget builders, §TextEdit
- `CP-T8-4` · name and order of size variants · `Default` for the middle size (Button, Toggle, Card, Item, Avatar, Switch, Select, AlertDialog, Attachment, MenuButton) / `Md` (sidebar sub-button) / each upstream literal; and must every size enum derive `Default` (`AvatarSize`, `InputGroupButtonSize` do not) · no egui idiom
- `CP-T8-5` · meaning of the setter name `size` · always a variant enum, pixels get other names (`.diameter`, `.width`/`.height`) / pixels where there are no variants (spinner, sheet, drawer) / `Vec2` where both axes matter (skeleton, input-otp `slot_size`) · idiom §Widget builders (egui `Spinner::size(f32)`)
- `CP-T8-6` · how a group shares size with its children · explicit size on the group (toggle-group, item-group CP-03-17, input-group CP-06-33) / inferred from children (avatar group CP-04-27, button-group CP-01-12) / through the child `Ui` (card CP-03-7) / from the item list (toggle-group, tabs) · idiom §Containers
- `CP-T8-7` · how widgets that need a height get one · parent's available height (carousel vertical, message-scroller, resizable) / optional `.height` (resizable CP-14-41, chart) / `.aspect(ratio)` or the AspectRatio wrapper / content height + `.min_height` (empty) / `.max_height` (scroll-area) · idiom §ScrollArea, §Widget builders
- `CP-T8-8` · fraction-of-parent sizes · recipe field + layout helper (bubble CP-18-20) / absolute px from the caller (progress) / fraction argument (carousel `per_view`; CP-14-34 option) / percent 0–100 (resizable CP-14-34) / hard-coded only (sheet, drawer `w-3/4`) · no egui idiom
- `CP-T8-9` · when a recipe dimension gets a public setter · every demo-overridden value (input-otp `slot_size`, calendar `cell_size`, sidebar widths, carousel and toggle-group spacing, spinner size) / only values a shipped demo changes (CP-03-5, CP-04-18, CP-14-31, CP-10-18 stay constants) / never, restyle via theme · no egui idiom

### T9 · State flags

**G1 · Whole-widget disabled: `.enabled(bool)`, the `Ui` only, or both (anchor CP-01-1)**
See also (placed in another group): CP-16-39, CP-16-42, CP-16-43.

- `CP-01-1` (button) · crate-wide anchor: how is disabled expressed · keep `.enabled(bool)` on every widget (Button, Checkbox, Switch, Select, Tabs today) / drop it, `ui.add_enabled` as egui leaves / both; per-item extension (X2): item flag named `enabled(bool)` (`ToggleItem`) / `disabled(bool)` (`SelectItem`, `CommandItem`, CP-08-32) / none · sketch: drop `.enabled`; `ToggleItem::enabled`
- `CP-02-5` (badge) · per-widget `.enabled(bool)` (as `Button`/`Label` have) vs only `ui.add_enabled` (egui idiom) · `ui.add_enabled` only (X2: the per-widget disabled flag is a crate-wide question, see CP-01-1; this pick is flagged there)
- `CP-05-3` (checkbox) · Disabled on leaf widgets? · `.enabled(bool)` builder (crate today, also `Select`, `Label`) / only `ui.add_enabled` (egui: leaf widgets have no `.enabled`) · keep `.enabled(bool)` for crate consistency.
- `CP-06-1` (input) · How is a single input disabled? · `.enabled(bool)` builder (crate today, all components) / `ui.add_enabled` only (egui leaf-widget idiom) / both · pick: `ui.add_enabled` only (crate-wide decision).
- `CP-07-1` (label) · How does a label get the "disabled peer" look? · (a) `.enabled(bool)` on Label (today), (b) only `ui.add_enabled(false, Label::new(..))` / disabled `Ui` (egui idiom: no `.enabled` on leaf widgets), (c) both · sketch: (c), with `.enabled` kept for a label of a disabled control in an enabled row
- `CP-07-10` (field) · Does `Field::enabled(false)` disable the control? · (a) yes, `add_enabled_ui` (egui; `<fieldset disabled>` semantics), (b) fade the label only (`data-disabled` upstream) · sketch: (a)
- `CP-07-31` (native-select) · disabled on `NativeSelect` · keep `.enabled(bool)` (as Select) / `ui.add_enabled` only / both · sketch: keep; follows CP-01-1
- `CP-08-12` (select) · disabled · `.enabled(bool)` builder (today, as `Input`) / only `ui.add_enabled` (egui: leaf widgets have no `.enabled`) · sketch: keep `.enabled` for crate consistency; maintainer decides crate-wide.
- `CP-09-18` (dropdown-menu) · Disabled · `.enabled(bool)` builder (crate Button, Select, Checkbox) / only `ui.add_enabled` (egui idiom, idioms §enabled-state-add_enabled-uidisable) / both honoured · both honoured.
- `CP-12-7` (tabs) · disabling the whole list · `.enabled(bool)` (today, `tabs.rs:71`) / `ui.add_enabled` only (egui leaf-widget idiom) · `ui.add_enabled` only. This is option (b) of anchor CP-01-1.
- `CP-15-17` (calendar) · calendar disabled · `.enabled(bool)` / `ui.add_enabled` only / both · sketch: `.enabled` + honour `ui.is_enabled()`; follows CP-01-1 (per-date predicates are not per-item flags)
- `CP-18-37` (marker) · disabled on attachment, bubble, marker · `Ui` only / `.enabled` / both · sketch: `Ui` only (CP-01-1 b)

**G2 · Per-item disabled: name and shape (CP-01-1 per-item extension)**
See also (placed in another group): CP-01-1.

- `CP-05-15` (radio-group) · Disabling one item? · `ui.add_enabled(false, item)` / item `.enabled(false)` · `ui.add_enabled` plus group `.enabled` (CP-05-3 decides whether items also get `.enabled`).
- `CP-07-27` (native-select) · Per-option disabled (HTML `<option disabled>`) · (a) `ui.add_enabled_ui` around `option(..)` in the closure, (b) not offered · sketch: (a); the closure API gives it for free
- `CP-08-32` (command) · disabled command item · `CommandItem::disabled(bool)` / `add_enabled` around the item · sketch: `disabled(bool)` (as `SelectItem`); per-item name follows CP-01-1
- `CP-08-37` (command) · per-item disabled flag name across the crate · see CP-08-32 and CP-01-1 · sketch: follows CP-01-1.
- `CP-12-6` (tabs) · disabled trigger API · `.tab_enabled(bool, v, atoms)` / `.tab(..)` returning a `Tab` builder with `.enabled(bool)` / `&[(V, &str, bool)]` · `.tab_enabled`. Per-item extension of anchor CP-01-1; the form must agree with the accordion item (CP-12-22), see CP-12-32.
- `CP-12-22` (accordion) · disabled item API · `.enabled(bool)` on the item builder (CollapsingHeader precedent) / `ui.add_enabled_ui` around the item · `.enabled(bool)`. Per-item extension of anchor CP-01-1; differs in form from tabs' `.tab_enabled(bool, ..)` (CP-12-6), see CP-12-32.
- `CP-12-32` (accordion) · per-item disabled form, tabs and accordion · two forms as sketched (`tab_enabled(bool, v, atoms)`, `AccordionItem::enabled`) / item builder `.enabled` for both / `*_enabled` on the parent for both / `.disabled()` per CP-01-31 · sketch: two forms; extends CP-01-1
- `CP-13-8` (navigation-menu) · disable a navigation item · `add_enabled_ui` around `nav.item` / `.item_enabled(bool)` / builder `nav.item(..).enabled()` · sketch: `add_enabled_ui` (CP-01-1 b; per-item name per X2)
- `CP-19-38` (questionnaire) · questionnaire disabled (CP-01-1 per-item) · `Choice::disabled(bool)` / `Choice::enabled(bool)` / not offered; `Question::disabled` vs a distinct name for "removed from the flow" (`.skip_in_flow`, `.hidden`) · sketch: `Choice::disabled`, `Question::disabled`
- `CP-19-31` (questionnaire) · Next/Submit disabled by the caller (`navigation-state` demo) · `.next_enabled(bool)` / whole-row `add_enabled_ui` / not offered · not decided in the sketch

**G3 · Disabled trigger under an attached overlay**

- `CP-10-10` (tooltip) · disabled triggers · enabled only (`for_enabled`, today) / separate disabled variant (egui `on_disabled_hover_text`) / both alike (bits via the wrapper span) · both alike.

**G4 · Boolean setter form (anchor CP-01-31)**
See also (placed in another group): CP-01-31, CP-02-29, CP-03-35, CP-06-38, CP-07-30, CP-09-36, CP-11-36, CP-12-31, CP-13-34, CP-14-44, CP-15-37, CP-16-39, CP-16-42, CP-16-43, CP-17-15, CP-18-34, CP-19-37.


**G5 · `invalid`: presence and scope**

- `CP-02-4` (badge) · `invalid` flag (base classes style `aria-invalid`) · omit (today) / `.invalid(bool)` like `Button::invalid` · `.invalid(bool)`
- `CP-05-14` (radio-group) · Where does `invalid` live? · item only (upstream puts `aria-invalid` on each Item) / group, inherited by items / both · both (group default, item override).
- `CP-06-18` (input-otp) · Invalid scope · whole widget `.invalid(bool)` / per slot (demo sets `aria-invalid` per Slot) · pick: whole widget.
- `CP-07-8` (field) · Does `Field::invalid` reach the control? · (a) only label colour; caller sets `.invalid` on the control (upstream demos), (b) also put INVALID into the closure's `Ui` context for crate controls to read · sketch: (a)
- `CP-15-19` (calendar) · `invalid` · drop (no shadcn class styles `data-invalid`) / `.invalid(bool)` with destructive ring · pick: drop.

**G6 · Validation messages and `required`**

- `CP-07-9` (field) · Is `invalid` derived from errors? · (a) independent flag (Field), (b) `!errors.is_empty()` implies invalid (formsnap `getAriaInvalid`), (c) both: errors imply invalid, `.invalid(true)` also without a message · sketch: (c)
- `CP-07-21` (form) · Field `required` marker (formsnap `aria-required` from constraints) · (a) `Field::required(bool)` setting AccessKit `required` (no visual upstream), (b) not offered · sketch: (b); noted because formsnap carries it
- `CP-19-27` (questionnaire) · external validation (`invalid` prop + caller text) · `Question::error(Option<text>)` / `.invalid(bool)` + separate error text · `error(Option<text>)`
- `CP-19-34` (questionnaire) · text strings ("Question N of M", the two default error messages, "Next") · English defaults with setters / required arguments · English defaults with setters (upstream ships these defaults)

**G7 · Click sense and link on display widgets**
See also (placed in another group): CP-18-4.

- `CP-02-3` (badge) · link badge (`href` → `<a>`) · not ported (today) / `.link(bool)` → click sense + focus + `WidgetType::Link`, caller reacts to `clicked()` / `.href(url)` opening via `ctx.open_url` like `egui::Hyperlink` · `.link(bool)`
- `CP-03-13` (item) · clickable item (upstream `child` → `<a>`) · `.sense(Sense)` / `.clickable(bool)` / separate `ItemButton` / not offered · `.sense(Sense)`
- `CP-18-19` (bubble) · Clickable content · `.sense(Sense::click())` vs. a separate `BubbleButton` widget · `.sense`.
- `CP-18-25` (marker) · Link vs. button · `.link(true)` (anchor look) and `.sense(click)` (plain button, no hover since the hover is demo-only) vs. one `.sense` that always underlines · two setters.

**G8 · Context from an enclosing component**

- `CP-02-13` (kbd) · how the tooltip/input-group context reaches Kbd · explicit setter `.in_tooltip(bool)` / detect from `ui.stack()` (`UiKind::Tooltip`; an InputGroup tag in `UiStackInfo`) · detect from `ui.stack()`
- `CP-06-30` (input-group) · Disabled addon fade (dead upstream: nothing sets `data-disabled`) · fade addons when the `Ui` or control is disabled / port literally (never) · pick: fade.
- `CP-06-34` (input-group) · `in-data-[slot=combobox-content]` context · handle now with a flag / leave to the Combobox port · pick: Combobox port.
- `CP-13-12` (navigation-menu) · Should content links lose the focus ring (`**:data-[slot=navigation-menu-link]:focus:ring-0`)? · keep the ring (stylegen drops the `**` class) / add an `IN_CONTENT` override to suppress it · follow upstream: no ring in content, via the in-content flag.
- `CP-15-20` (calendar) · background (`bg-background`, transparent inside card/popover content) · `.background(bool)` default on / detect crate Card/Popover via the `UiStack` / always transparent · pick: `.background(bool)`.
- `CP-18-36` (marker) · ghost signal from Bubble to Message · caller sets `Message::ghost(bool)` / Bubble writes a `Ui` flag read one frame late / ignore · sketch: caller (one decision with CP-19-5)
- `CP-19-5` (message) · how Message learns a ghost Bubble is inside · `.ghost(bool)` by the caller / Bubble writes a `Ui` flag read one frame late / ignore · sketch: `.ghost(bool)` (one decision with CP-18-36)

**G9 · Single-component flags and enums**

- `CP-05-1` (checkbox) · How is indeterminate expressed? · `.indeterminate(bool)` display flag (egui `Checkbox`) / tri-state `&mut CheckState` enum / `&mut Option<bool>` · `.indeterminate(bool)`; click from indeterminate writes `true`.
- `CP-06-7` (input) · HTML `type` values (`email`, `url`, `tel`, `search`, `file`) · ignore all but `password` / add a `kind` enum (search could add a clear button) · pick: `password(bool)` only; `file` not ported.
- `CP-09-9` (dropdown-menu) · Destructive · `.variant(MenuItemVariant::Destructive)` enum (crate `ButtonVariant`) / `.destructive(bool)` · enum.
- `CP-09-14` (dropdown-menu) · `closeOnSelect` defaults · shadcn's (item closes, checkbox/radio stay open) / bits' (all close) / egui's (`CloseOnClick`: every click closes) · shadcn's, with `.close_on_select(bool)`.
- `CP-11-31` (drawer) · `dismissible(false)` · port (blocks Esc, backdrop, drag) / omit · port.
- `CP-15-18` (calendar) · `read_only` (focus and navigate, no select) · keep / drop · pick: keep.
- `CP-15-28` (calendar) · outside-month days · selectable (shadcn passes `disableDaysOutsideMonth=false`) / disabled (bits-ui default) · pick: selectable, `disable_outside_days(true)`.
- `CP-16-34` (sonner) · `rich_colors` · (a) not offered (shadcn leaves it off); (b) offered with svelte-sonner's colours · (a)
- `CP-18-7` (attachment) · State · one `AttachmentState` enum vs. separate flags (`.error(bool)`, `.busy(bool)`) · enum, default `Done`.
- `CP-18-27` (marker) · Shimmer · `.shimmer(bool)` caller parameter (demo-only class) vs. leave out · include, gated on a shimmer painter existing.
- `CP-18-29` (marker) · `role="status"` · `.status(bool)` setting an AccessKit live region vs. leave to the caller · `.status(bool)`.
- `CP-07-16` (field) · Label weight override (`class="font-normal"` in field-checkbox and field-radio demos) · (a) `Field::label_weight(Weight)`, (b) accept `RichText` and let its weight win, (c) not offered · sketch: (b)
- `CP-07-18` (field) · FieldGroup `data-slot="checkbox-group"` gap and nested-group gap · (a) `.tight()` setter + auto-detect nesting via `Ui` data, (b) `.tight()` and `.nested()` setters · sketch: (a)
- `CP-04-6` (alert) · Custom colours (`alert-colors.svelte` amber override) · offer `.fill/.text_color/.stroke` setters vs leave to recipe replacement · sketch: no setters (recipe replacement, as for other components).
- `CP-04-25` (avatar) · Caller colours (demo `bg-green-600` badge, `grayscale` class) · `.fill(Color32)` on the badge and an image tint/greyscale setter vs none · sketch: badge `.fill` only; no greyscale.
- `CP-03-31` (aspect-ratio) · degenerate ratio (0, negative, NaN, infinite) · height 0 as upstream / fall back to 1.0 · height 0
- `CP-13-28` (pagination) · Degenerate input: `per_page = 0`, `total_pages = 0`, `page` outside the range · clamp `page` into `1..=max(total_pages,1)` every frame and treat `per_page = 0` as one page / leave it like upstream · clamp, and do not mark it as changed (unsure: clamping silently writes the caller's value).
- `CP-16-12` (table) · Striping / clickable rows · (a) neither (upstream has neither); (b) `.striped(bool)` like `Grid` · (a) _(tagged 1)_

**G10 · `State` bits and generator rules (owned by T11; listed for completeness)**
Placed under the topic their question belongs to: CP-18-30, CP-11-37, CP-11-38, CP-12-34, CP-13-36, CP-14-48, CP-15-38, CP-19-40, CP-19-41, CP-01-17, CP-19-18, CP-02-9, CP-06-17, CP-01-18, CP-18-31, CP-16-9, CP-16-22, CP-18-22, CP-09-32, CP-09-33, CP-14-24, CP-15-27, CP-15-34, CP-15-35.


**G11 · Animation and repaint**

- `CP-02-17` (spinner) · continuous animation · always spin with `ui.request_repaint()` (egui::Spinner) / `request_repaint_after` frame cap (cheaper on CPU renderers) / `.animate(bool)` or honour `style.animation_time == 0` as reduced motion · always spin, `request_repaint()`
- `CP-02-27` (skeleton) · pulse · always (with `request_repaint`) / static / `.animate(bool)` · follow CP-02-17's pick
- `CP-04-13` (progress) · Animate value changes (`transition-all`) · `ctx.animate_value_with_time` vs jump (generator ignores animations) · sketch: jump.
- `CP-06-20` (input-otp) · Caret blink · egui's `visuals.text_cursor` blink / own 1 s `animate-caret-blink` · pick: egui's.
- `CP-14-16` (sidebar) · 200ms width animation · (a) animate (`animate_bool_with_time` / `show_animated_inside`) (b) snap, as the crate does for other transitions · sketch: (b)
- `CP-15-32` (range-calendar) · hover preview of the pending range (`data-highlighted`, unstyled upstream) · none, as upstream / accent preview · pick: none.
- `CP-17-13` (chart) · Hover highlight dimming of other series (opacity 0.1 in BarChart, overridden to 1 in the container for area/spline) · implement / skip · sketch: skip.
- `CP-18-8` (attachment) · Title shimmer while uploading/processing · animate (continuous repaint) vs. static (spec §4.4 puts animation out of scope) · animate; maintainer to confirm against §4.4.
- `CP-19-16` (message-scroller) · smooth vs instant scroll · always egui's scroll animation / `.animated(bool)` like `ScrollArea::animated` / per-command behaviour argument · not decided in the sketch

**G12 · Accessibility role and name**

- `CP-01-19` (toggle) · `widget_info` type: `Button` + selected / `Checkbox` / `SelectableLabel` (egui `toggle_value`) · sketch: `Button` + selected
- `CP-01-28` (toggle-group) · `widget_info`: single → `RadioGroup` + `RadioButton` items, multiple → `Button` + selected (upstream roles) / one type for both · sketch: upstream roles
- `CP-02-19` (spinner) · accessible name · fixed "Loading" / `.label(..)` override with default "Loading" · `.label(..)`
- `CP-02-26` (skeleton) · accessibility · nothing (as upstream) / `WidgetType::ProgressIndicator` / `WidgetType::Other` · `WidgetType::Other`, no label
- `CP-04-19` (separator) · Accessibility default · decorative (radix base `decorative = true`, nothing reported) vs semantic (bits-ui `decorative = false`) · sketch: decorative, no `widget_info` (as egui's `Separator`).
- `CP-05-23` (switch) · Accessibility role? · `WidgetType::Checkbox` (today) / `Role::Switch` set through `accesskit_node_builder` · `Role::Switch`.
- `CP-08-34` (command) · accessibility of the highlight · item `WidgetInfo.selected` = highlighted (cmdk `aria-selected`) / = `checked` · sketch: highlighted, as upstream.
- `CP-12-12` (tabs) · accessibility role · `WidgetType::SelectableLabel` per trigger (today) / AccessKit `Role::Tab`/`TabList`/`TabPanel` via `accesskit_node_builder` · `SelectableLabel` plus AccessKit roles where available.
- `CP-12-25` (accordion) · accessibility · `WidgetType::CollapsingHeader` labelled only (egui's own) / plus `selected = open` / plus AccessKit `expanded` and heading level · `CollapsingHeader` + AccessKit `expanded`.
- `CP-19-20` (message-scroller) · accessibility: `role=log` live region · AccessKit node builder with `Role::Log` / `WidgetType::Other` only · AccessKit node builder
- `CP-16-10` (table) · Accessibility · (a) AccessKit `Role::Table/Row/Cell` via `accesskit_node_builder`; (b) no table roles, cells report their own widgets only · (a) _(tagged 6)_
- `CP-16-23` (carousel) · Accessibility · (a) root `WidgetType::Other` with label "carousel", slide index as value; (b) AccessKit roledescription via `accesskit_node_builder` · (b) _(tagged 6)_

**G13 · Focus and keyboard**

- `CP-01-27` (toggle-group) · Roving tab stop: upstream (first item until one is toggled on or reached by arrows) / SKILL.md rule (selected item, else first) as Tabs does · sketch: upstream
- `CP-11-7` (dialog) · initial focus and focus return (bits-ui FocusScope) · port both / port initial focus only / none (today) · port both.
- `CP-11-16` (alert-dialog) · initial focus · content (bits-ui; keyboard needs one Tab to reach Cancel) / Cancel button (safer default in egui, which has no focusable panel) · Cancel button; deviates from upstream.
- `CP-11-33` (drawer) · initial focus · none (vaul `autoFocus=false`) / first tabbable like Dialog · none.
- `CP-14-29` (scroll-area) · Keyboard scrolling and focus ring of the viewport (browser native; egui has none) · (a) add (focusable viewport, arrows/PgUp/PgDn/Home/End) (b) skip · sketch: (b)
- `CP-14-39` (resizable) · F6 handle cycling · (a) port (b) drop, Tab order is enough · sketch: (b)
- `CP-16-33` (sonner) · Hotkey Alt+T and Escape · (a) port both; (b) leave out · (a)
- `CP-19-35` (questionnaire) · how ArrowLeft/Right and shortcut keys coexist with other widgets on the screen · only while focus is inside the questionnaire / always while it is visible · only while focus is inside (upstream listens on the form)
- `CP-06-23` (input-otp) · Focus on a full value · select the last cell (bits) / caret at end · pick: bits. _(tagged 7)_
- `CP-06-31` (input-group) · Addon click focus · focus the control whatever its kind / only an Input (upstream `querySelector("input")`) · pick: any control. _(tagged 7)_
- `CP-09-19` (dropdown-menu) · Keyboard model · implement bits' (loop, Home/End, PageUp/Down, typeahead, ArrowRight/Left for sub-menus, first item on keyboard open) as a shared primitive with Select / rely on egui's Tab and arrow focus movement (memory/mod.rs:561-563) · implement, shared with Select. _(tagged 2)_
- `CP-13-14` (navigation-menu) · Tab from an open trigger into the content first (bits focus proxy) · emulate (panel right after its trigger, content out of tab order when closed) / accept egui's order · sketch: emulate _(tagged 2)_
- `CP-15-23` (calendar) · focus model · one focusable widget with an internal focused date and arrow `EventFilter` (one Tab stop like the roving tabindex) / every day a focusable widget · pick: one widget. _(tagged "Behaviour"; no topic file lists it)_
- `CP-15-24` (calendar) · keys · arrows, Enter, Space only (upstream) / add PageUp/PageDown/Home/End (WAI-ARIA grid) · pick: upstream set. _(tagged "Behaviour"; no topic file lists it)_
- `CP-15-25` (calendar) · ±7 step with disabled days · count selectable cells (upstream quirk) / step by date and skip to the next enabled day · pick: step by date. _(tagged "Behaviour"; no topic file lists it)_

**G14 · Overlay timing**

- `CP-10-6` (popover) · bits trigger props `openOnHover`/`openDelay`/`closeDelay` (unused by shadcn) · port / skip · skip (HoverCard covers hover).
- `CP-10-11` (tooltip) · open delay · egui global `tooltip_delay` 0.5 s (today) / theme sets `tooltip_delay = 0` to match shadcn's Provider / per-tooltip `.delay(secs)` with own timer · egui global, the theme leaves it (deviation from shadcn's 0 documented).
- `CP-10-16` (hover-card) · delay defaults · bits 0.7 s / 0.3 s / shadcn demo values (0.01/0.1, 0.1/0.1) / egui `tooltip_delay` · bits defaults with `.open_delay`/`.close_delay`.
- `CP-10-22` (hover-card) · keep open during pointer-down / text selection in the content · port both / pointer-down only (egui labels are not selectable by default) / none · pointer-down only.

**G15 · Other behaviour tagged 9**

- `CP-02-20` (spinner) · default colour · `palette.foreground` (as IconView) / inherited text colour of the enclosing `Ui` (CSS `currentColor`) · inherited text colour
- `CP-14-20` (sidebar) · Skeleton width (upstream random 50–90%) · (a) caller passes it (b) deterministic from the widget id (c) random · sketch: (b)
- `CP-14-37` (resizable) · Collapsible panes (Enter toggles, snap) · (a) port with min/max (b) min/max only · sketch: (a)
- `CP-14-42` (resizable) · Drag cursor at a limit (`w-resize`/`e-resize` vs `ew-resize`) · (a) port (b) always the two-way cursor · sketch: (a)
- `CP-19-30` (questionnaire) · per-item status callback (`onStatusChange`, `questionnaire-navigation-state`) · caller reads `state.answers[i].status` / `output.status_changed: Option<(usize, ItemStatus)>` · read from state
- `CP-04-23` (avatar) · Loading-status control (bits `loadingStatus`, `delayMs`, `onLoadingStatusChange`) · expose (`.delay(Duration)`, return status) vs rely on egui's loader poll only · sketch: not exposed; Pending/Err show the fallback. _(tagged 2)_
- `CP-14-28` (scroll-area) · Visibility modes · (a) only `hover` (shadcn's default) (b) own enum with bits-ui's four types (c) egui's `ScrollBarVisibility` (has no hover/scroll mode) · sketch: (b) _(tagged 2)_
- `CP-19-32` (questionnaire) · reset (`type="reset"` button restores defaults) · `QuestionnaireState::reset(&mut self, defaults)` / caller rebuilds the state · caller rebuilds (state is plain data) _(tagged 10)_

**New choice points from the topic cross-check (no sketch pick)**

- `CP-T9-1` · do containers and overlays carry `.enabled(bool)` when leaves do not · yes, as egui `Area`/`Window`/`CollapsingHeader::enabled` (Field, FieldSet, AccordionItem, MenubarMenu, HoverCard) / no, `add_enabled_ui` around (table, carousel, accordion, breadcrumb, navigation-menu, sidebar, resizable, questionnaire) / same as leaves (CP-01-1) · idiom §Enabled state
- `CP-T9-2` · one shape for per-item disabled · setter on the item builder (ToggleItem, AccordionItem, SelectItem, CommandItem, MenuItem, Choice) / `*_enabled(bool, ..)` on the parent (tabs) / `add_enabled_ui` around a closure item (navigation-menu, native-select, radio, attachment action) / setter for data items, `add_enabled_ui` for closure items · idiom §Enabled state
- `CP-T9-3` · overlay bound to a disabled trigger · follow the trigger, no open (popover, collapsible, context-menu) / open anyway (tooltip CP-10-10) / own `.enabled` (hover-card) / hint overlays open, action overlays do not · idiom §Popups (`on_disabled_hover_text`)
- `CP-T9-4` · how `invalid` flows between a group and its parts · group sets, parts inherit and may override (radio-group) / whole widget only (input-otp) / caller sets each part (toggle-group, Field CP-07-8 a) / wrapper reads the inner control (input-group) / group pushes `INVALID` into the `Ui` (CP-07-8 b) · no egui idiom
- `CP-T9-5` · one validation API for Field and Questionnaire · `.invalid(bool)` + `.error(text)`, errors imply invalid (Field CP-07-9) / `error(Option<text>)` only (Questionnaire CP-19-27) / both on both; text type `impl Into<String>` vs `Option<impl Into<WidgetText>>` · no egui idiom
- `CP-T9-6` · `required` flag · behavioural (questionnaire) / AccessKit-only on Field and controls (CP-07-21 a) / not offered outside questionnaire (CP-07-21 b) · no egui idiom
- `CP-T9-7` · read-only flag · `read_only(bool)` where upstream has `readonly` (calendar CP-15-18; radio-group, checkbox upstream) / calendar only / none; egui `TextEdit::interactive` naming · idiom §Enabled state
- `CP-T9-8` · positional `bool` parameters · allowed as sketched (`tab_enabled(bool, ..)`, `row_with(selected, expanded, ..)`, `MenuButton::action(.., show_on_hover)`, `menu_skeleton(ui, show_icon)`, `FieldCard::new(selected)`) / chained setters / small struct or enum · idiom §Widget builders (same question as CP-T11-9)
- `CP-T9-9` · name of the caller-set "current item" flag · `.active(bool)` (navigation link, sidebar; upstream `data-active`) / `.selected(bool)` (table row, FieldCard, legend item; egui `Button::selected`) / upstream name per component · idiom §Selection (same question as CP-T1-11)
- `CP-T9-10` · how a display widget becomes clickable, and what `link` means · `.sense(Sense)` like egui `Label::sense` (item, attachment, bubble, marker) / `.link(bool)` = click + focus + `WidgetType::Link` (badge) / `.link(bool)` look only + `.sense` (marker) / a separate clickable type · idiom §Widget builders, §widget_info
- `CP-T9-11` · how a part learns context from its parent · detect via `ui.stack()` / `UiStackInfo` tags (kbd, grouped button, nav content, sidebar) / explicit caller setter (`Message::ghost`, `Calendar::background`) / parent writes `Ui` data (CP-07-8 b, CP-19-5 option) / not propagated (Field `invalid`) · idiom §Containers
- `CP-T9-12` · who sets the trigger's `expanded` state · caller via `Button::expanded(open)` + a read accessor like `Popover::is_open` (popover, table rows) / the helper owning the state (collapsible, menus, select, navigation-menu) / both · no egui idiom
- `CP-T9-13` · mutually exclusive states: enum or `bool` flags · `bool` display flag (checkbox `indeterminate`) / enum (`AttachmentState`, `MenuItemVariant::Destructive`, alert variant) / enum for three or more states, `bool` for two · no egui idiom
- `CP-T9-14` · caller switch for animation (batch-B Y13) · always animate (spinner, skeleton, shimmer) / never (progress, sidebar width) / per-component `.animate(bool)` like `ScrollArea::animated` (CP-19-16, `.shimmer`) / one global rule from `style.animation_time` (CP-02-17) · idiom §ScrollArea
- `CP-T9-15` · caller override of built-in Previous/Next buttons · automatic only (pagination, carousel) / `.next_enabled(bool)`-style setter (CP-19-31 option) / `add_enabled_ui` around the whole component · no egui idiom

### T10 · Shortcut function

**G1 · Return type of a shortcut that edits a value**
See also (placed in another group): CP-16-39, CP-13-18.

- `CP-08-13` (select) · shortcut return · `bool` (today) / `Response` (egui shortcuts `ui.checkbox`, `ComboBox::show_index` return `Response`) · sketch: `Response`.
- `CP-05-5` (checkbox) · Shortcut return type across the crate? · `Response` (`checkbox`, `toggle`) / `bool` (`select`, `tab_bar`) · `Response`.
- `CP-12-10` (tabs) · shortcut · `tab_bar(..) -> bool` (today) / `tabs(..) -> Response` (egui `ui.*` shortcuts return `Response`) / none · `tabs(..) -> Response`. Follows anchor CP-08-13 (crate-wide shortcut return type).
- `CP-15-21` (calendar) · shortcut return · `bool` changed (today's `select`, `tab_bar`) / `Response` (egui `ui.checkbox`, `ComboBox::show_index`) · pick: `bool`, but CP-08-13 (crate-wide anchor) and CP-12-10 pick `Response` and change exactly `select` and `tab_bar`; this CP follows CP-08-13's outcome.
- `CP-15-33` (range-calendar) · shortcut · `range_calendar(ui, &mut DateRange) -> bool` / `-> Response` / no shortcut · pick: shortcut returning `bool`; the return type follows CP-08-13 (crate-wide, picks `Response`) and CP-12-10, like CP-15-21.
- `CP-13-23` (pagination) · pagination shortcut return · `bool` / `Response` · sketch: `bool`; follows CP-08-13 once decided

**G2 · Return type of a part helper that paints a fixed element**

- `CP-04-16` (separator) · Shortcut return type · `Response` (egui `Ui::separator`) vs `()` (current crate) · sketch: `Response`.

**G3 · Offer a shortcut for this component at all**
See also (placed in another group): CP-03-29.

- `CP-01-4` (button) · Add a `button(ui, text) -> Response` shortcut (egui `ui.button`)? · yes / no · sketch: yes
- `CP-04-8` (alert) · Shortcut · `alert(ui, title, description) -> Response` vs none · sketch: provide it.
- `CP-04-14` (progress) · Shortcut · `progress(ui, fraction) -> Response` vs none · sketch: provide it.
- `CP-05-38` (slider) · Shortcut? · none (today) / `slider(ui, &mut f32, range) -> Response` · add `slider`.
- `CP-06-5` (input) · Shortcut function · none (today) / `input(ui, &mut String) -> Response` like `checkbox`/`toggle` · pick: add it.
- `CP-16-11` (table) · Shortcut · (a) `table(ui, headers, rows)` text-only; (b) no shortcut · (a)
- `CP-18-32` (marker) · Shortcut · `marker(ui, text) -> Response` vs. only `impl Widget` · provide `marker()`.
- `CP-03-8` (card) · keep `layout::card` · keep / remove / move to `components::card::card(ui, ..)` · remove
- `CP-03-19` (item) · shortcut function · none / `item(ui, title, description) -> Response` · none
- `CP-04-30` (avatar) · Shortcut · `avatar(ui, fallback) -> Response` vs none · sketch: none (an avatar without image is rare enough; builder suffices).
- `CP-07-13` (field) · Shortcut function · (a) none beyond the builder, (b) `field(ui, label, widget) -> Response` · sketch: (a); `Field::new().label(..).add(..)` is already one line
- `CP-08-35` (command) · shortcut function · none / `command_palette(ctx, &mut open, &[&str]) -> Option<usize>` · sketch: none.
- `CP-10-21` (popover) · a shortcut for popover · none / `popover(&trigger, |ui| …)` · none.
- `CP-18-12` (attachment) · Shortcut · only `impl Widget` vs. a free `attachment(ui, title, description) -> Response` · only `impl Widget`.
- `CP-17-11` (chart) · Shortcut · `chart_tooltip(resp, ..)` free fn like `tooltip()` / `Response` extension / none · sketch: none (open).

**G4 · Shortcut name**
See also (placed in another group): CP-01-15, CP-05-22, CP-05-19, CP-02-22, CP-12-10.


**G5 · Arguments of one shortcut**

- `CP-02-6` (badge) · shortcut signature · `badge(ui, &str, variant)` (today) / `badge(ui, text)` like `ui.button(text)` · `badge(ui, impl Into<WidgetText>)`

**G6 · Form of a helper attached to a trigger `Response`**
See also (placed in another group): CP-10-8, CP-17-11, CP-10-14.

- `CP-12-29` (collapsible) · helper shape · free fn `collapsible_trigger(Response, &mut bool) -> Response` / extension trait `resp.toggles(&mut open)` / `Collapsible` builder · free fn. Return type follows anchor CP-08-13 (`Response`).

**G7 · Trigger helper owning open state**

- `CP-14-18` (sidebar) · Trigger · (a) `sidebar::trigger(ui, id) -> Response` toggling memory (b) `SidebarTrigger::new(&mut open)` widget (c) caller `Button` + `Sidebar::toggle(ctx, id)` · sketch: (a) plus `toggle`

**G8 · Keyboard-shortcut display helper**

- `CP-02-11` (kbd) · an egui-native shortcut helper · none / `kbd_shortcut(ui, &KeyboardShortcut)` via `ModifierNames` (`SYMBOLS` on mac, `NAMES` elsewhere) / one chip with `ctx.format_shortcut` text · one chip per modifier via `ModifierNames`

**G9 · Tagged 10 but not about a shortcut function**
Placed under the topic their question belongs to: CP-19-15, CP-19-32.


**New choice points from the topic cross-check (no sketch pick)**

- `CP-T10-1` · return of a shortcut that takes a content closure · `InnerResponse<Option<R>>` (dropdown_menu, menubar_menu; `menu_button`) / `Option<R>` (context_menu) / `InnerResponse<R>` (bubble_group, message_group) / the wrapped container's output (attachment_group) / bare `R` (`layout::aspect_ratio`) / whatever the builder returns (CP-10-3, CP-03-1) · idiom §Containers, §Popups
- `CP-T10-2` · return of a part helper painting a fixed element · `()` (button_group_separator, menu_label, menu_separator, sidebar::separator, opt_group; today separator, card_header, popover_header) / `Response` (item_separator, card_title, field::separator, CP-04-16) / `Option<Response>` when it may paint nothing (field::error) · idiom §Widget builders (same as CP-T4-8, CP-T5-11)
- `CP-T10-3` · text type of shortcuts and part helpers · `impl Into<WidgetText>` (badge, kbd, label, card_title, tooltip, bubble, marker, …) / `impl Into<String>` (button, toggle_button, menu_value, toast) / `&str` (menu_label, menubar_menu, attachment_action, breadcrumb; today badge, label, tooltip) / `impl IntoAtoms` (alert_dialog cancel/action) · idiom §Text, §Atom
- `CP-T10-4` · variant in a shortcut's arguments · default variant only (badge CP-02-6, alert) / positional variant (bubble; badge today) · idiom §Widget builders (same as CP-T11-12)
- `CP-T10-5` · label text in value shortcuts · none, label is a separate `Label` (checkbox, toggle/switch, radio_value) / a text argument (toggle_button, menu_value; egui `ui.checkbox(&mut b, text)`) · idiom §Selection, §Widget builders
- `CP-T10-6` · value form of a selection shortcut · `&mut usize` + `&[&str]` (select shortcut, pagination, `tab_bar` today) / `&mut V` + `&[(V, &str)]` (tabs, native_select) / `&mut Option<V>` + `&[(V, &str)]` (toggle_group) / same as the builder; item slice type `&str` / `(V, &str)` / `Vec<String>` / `impl AsRef<str>` · idiom §Selection (same as CP-T1-8)
- `CP-T10-7` · id argument in shortcuts of stateful widgets · required `id_salt` (select, native_select, sidebar::trigger) / none, auto id (tabs, toggle_group, pagination, calendars, breadcrumb) / the text as id source · idiom §IDs
- `CP-T10-8` · how a helper takes the trigger `Response` · by value, returned for chaining (tooltip, collapsible_trigger, chart_tooltip; `on_hover_text(self)`) / `&Response` (context_menu, overlay builders); context-level commands take `&Context` and return a handle (toast, command_palette, `Sidebar::toggle`) · idiom §Response methods
- `CP-T10-9` · naming scheme of part helpers · component-prefixed free fns (button_group_text, menu_label, card_title, popover_header, collapsible_trigger, attachment_action, bubble_group) / short names via the module (field::description, alert_dialog::cancel, sidebar::trigger, dialog::footer), bare names then repeat (`field::separator`, `separator::separator`) · no egui idiom (same as CP-T4-9, CP-T11-8)
- `CP-T10-10` · trigger argument of a menu shortcut · crate `Button` (dropdown_menu) / text `&str` (menubar_menu) / `impl IntoAtoms` (egui `menu_button`) / `impl Widget` · idiom §Popups; batch-A report-09 Minor
- `CP-T10-11` · crate rule for which components get a shortcut · every leaf whose common use is one call (button, badge, kbd, spinner, skeleton, progress, separator, label, checkbox, slider, input, textarea, marker) / only widgets with an egui `Ui` counterpart / per component (today, G3); unflagged: kbd, skeleton, textarea · idiom §Widget builders

### T11 · Cross-cutting conventions

**G1 · Boolean setter form (anchor CP-01-31)**
See also (placed in another group): CP-01-16.

- `CP-01-31` (toggle-group) · crate-wide anchor: boolean flag setter form · no-argument `.vertical()` (egui `Slider::vertical`) / `.flag(bool)` (egui `Button::selected`) / both · sketch: no-argument for named-state flags (`vertical`, `icon_only`), `bool` for state flags (`expanded`, `invalid`, `has_popup`) _(tagged crate-wide)_
- `CP-01-30` (toggle-group) · Orientation type: per-widget enum `ToggleGroupOrientation` + `.vertical()` / shared `Orientation` (CP-01-9) · sketch: `.vertical()` over a per-widget enum. References CP-05-37 (crate-wide orientation type, X3) and CP-01-31 (setter form). _(tagged 7)_
- `CP-02-25` (skeleton) · shape override · `.circle()` / `.radius(Radius)` / egui `.corner_radius(CornerRadius)` · `.circle()` + `.radius(Radius)` _(tagged 8)_
- `CP-02-29` (badge) · boolean setter form (X6): `.link(bool)`, `.invalid(bool)` beside `Skeleton::circle()` · settles under CP-01-31 · sketch mixes both forms _(tagged 9)_
- `CP-03-35` (empty) · flag setter form of `Empty::outline` (`.outline(bool)` vs `.outline()`) · with `bool` / without argument · follows the crate-wide CP-01-31; the sketch's `.outline(bool)` is a placeholder until CP-01-31 is decided _(tagged 9)_
- `CP-04-33` (separator) · Boolean setter form for `Separator::vertical()` / `.horizontal()` · no-argument flag (as sketched) vs `.vertical(bool)` · sketch: no-argument flag; the crate-wide question is CP-01-31. _(tagged 7)_
- `CP-05-37` (slider) · crate-wide anchor: orientation type · per-widget enums (`SliderOrientation`, `TabsOrientation`) / one shared crate `Orientation` / reuse `egui::SliderOrientation` · sketch: shared `Orientation` + `.vertical()`; 01, 04, 07 reference it; bool form is CP-01-31 _(tagged 7)_
- `CP-06-38` (input-group) · Boolean flag setter form on `Input`, `Textarea`, `InputOtp` (`password`, `invalid`) · `.invalid(bool)` as sketched / `.invalid()` without argument · pick: `(bool)`, crate-wide question CP-01-31 decides. _(tagged 9)_
- `CP-07-30` (field) · flag setter form of `Field::horizontal()`, `control_first()`, `FieldGroup::tight()` vs `invalid(bool)`, `enabled(bool)` · no-argument for modes, `bool` for states / `bool` everywhere / no-argument everywhere · follows CP-01-31 _(tagged 9)_
- `CP-09-36` (dropdown-menu) · flag setter form of `MenuItem::inset`, `close_on_select`, `MenuCheckboxItem::indeterminate` · no-argument / `bool` · sketch: `bool`; follows CP-01-31 (`close_on_select` defaults differ per item kind) _(tagged 9)_
- `CP-11-36` (dialog) · setter form of `show_close_button(bool)`, `modal(bool)`, `dismissible(bool)` (default true) · `bool` / no-argument negative (`.hide_close_button()`, `.non_modal()`) / both · sketch: `bool`; follows CP-01-31 _(tagged 9)_
- `CP-12-31` (tabs) · boolean setter form in this report · `Tabs::vertical()` with no argument (Slider form) and `AccordionItem::enabled(bool)` with an argument, as sketched / all `bool` (`.vertical(bool)`) / both · sketch keeps `vertical()` no-argument and `enabled(bool)`; follows anchor CP-01-31. _(tagged 7/9)_
- `CP-13-27` (pagination) · Previous/Next text: upstream hides it below `sm`, and the "icons-only" demo depends on that · always show text (egui counts as ≥ sm) / `.prev_next_text(bool)` · always show text plus `.prev_next_text(bool)` (setter form: CP-13-35). _(tagged 4)_
- `CP-13-34` (navigation-menu) · setter form of `NavigationMenu::viewport(bool)`, `NavigationMenuLink::active(bool)` · `bool` / no-argument (`.no_viewport()`, `.active()`) / both · sketch: `bool`; anchor CP-01-31 _(tagged 3, 9)_
- `CP-13-35` (pagination) · setter form of `Pagination::numbers`, `prev_next`, `prev_next_text`, `centered` (default true) · `bool` / negative no-argument (`.hide_numbers()`) / both · sketch: `bool`; anchor CP-01-31 _(tagged 4, 8)_
- `CP-14-44` (sidebar) · boolean setters in report 14 (`Sidebar::rail`, `MenuButton::active`, `action(.., show_on_hover)`, `with_handle`, `stick_to_bottom`, `menu_skeleton(ui, show_icon)`) · `bool` / no-argument / both · sketch: `bool`; anchor CP-01-31 _(tagged 9)_
- `CP-15-37` (calendar) · setter form of `paged_navigation`, `fixed_weeks`, `prevent_deselect`, `read_only`, `background`, `disable_outside_days`, `exclude_disabled` · `bool` / no-argument / both · sketch: `bool`; follows CP-01-31 _(tagged 9)_
- `CP-16-39` (table) · table vs crate-wide anchors · disabled via `add_enabled_ui` / `.enabled`; row flags as positional bools / chained setters / no-argument `.selected()`; shortcut `Response` / `bool` · sketch: CP-01-1 (b), positional bools (CP-16-5), `Response` (CP-08-13) _(tagged 9, 10)_
- `CP-16-42` (carousel) · carousel setters and disabled · `.vertical()` beside `.looping(bool)`, `.buttons(bool)` (mixed) / all `bool` / all no-argument; disabled via the `Ui` / `.enabled` · sketch: mixed (CP-01-31), disabled per CP-01-1 (b) _(tagged 9)_
- `CP-16-43` (sonner) · sonner setters and disabled · all `bool` (`expand`, `close_button`, `dismissible`) / no-argument / both; no `.enabled` / `.enabled` for uniformity · sketch: `bool`, no `.enabled`; pending CP-01-31, CP-01-1 _(tagged 9)_
- `CP-17-15` (chart) · Flag setter form of `ChartTooltip::hide_label()` and `hide_indicator()` (no argument; upstream props `hideLabel`, `hideIndicator` default `false`) · (a) no argument, (b) `bool` argument, (c) both. Crate-wide question: CP-01-31 · sketch: (a); it follows the answer to CP-01-31 _(tagged 9)_
- `CP-18-34` (marker) · boolean setter form in report 18 (`.vertical()`, `.spinner()` beside `.link(bool)`, `.shimmer(bool)`, `.status(bool)`) · no-argument / `bool` / both · sketch: no-argument for named states, `bool` for toggles; follows CP-01-31 _(tagged crate-wide)_
- `CP-19-37` (message) · boolean setter form in report 19 (`Message::end()` beside `ghost(bool)`, `auto_scroll(bool)`, `Question::multiple(bool)`, …) · no-argument / `bool` / both · sketch: mixed; follows CP-01-31 _(tagged crate-wide (9))_

**G2 · Icon parameter form (anchor CP-01-3)**
See also (placed in another group): CP-03-14, CP-03-28.

- `CP-01-3` (button) · Icon setter argument: `&Icon` (today) or `Icon` by value (it is `Copy`) · sketch: by value. Crate-wide anchor (X1): every report that takes an icon references this CP (`&Icon` vs `Icon`). _(tagged 4)_
- `CP-02-8` (badge) · how an `Icon` (which is `Copy`) is passed · `&Icon` copied (Badge/Button today) / `&'a Icon` borrowed (IconView today) / by value · by value _(tagged 4)_
- `CP-02-28` (badge) · icon parameter form across the sketches (X1) · by value (`Icon` is `Copy`) / `&Icon` copied (today) / `&'a Icon` (as `IconView`) · sketch: by value; defers to CP-01-3 _(tagged 4)_
- `CP-03-34` (empty) · icon parameter form of `Item::icon`, `Empty::icon` · by value (CP-01-3) / `&'a Icon` (Badge today) · follows CP-01-3; the sketch's `&'a Icon` is a placeholder _(tagged 4)_
- `CP-04-31` (alert) · Icon parameter form of `Alert::icon` · `&'a Icon` (as sketched) vs `Icon` by value (`Icon` is `Copy`) · sketch: `&'a Icon`; the crate-wide question is CP-01-3, and the pick changes to by-value if CP-01-3 settles on by-value. _(tagged 4)_
- `CP-04-32` (avatar) · Icon parameter form of `AvatarBadge::icon` and `AvatarGroup::count_icon` · `&'a Icon` (as sketched) vs `Icon` by value (`Icon` is `Copy`) · sketch: `&'a Icon`; the crate-wide question is CP-01-3, and the pick changes to by-value if CP-01-3 settles on by-value. _(tagged 4)_
- `CP-06-36` (input-group) · `InputGroupButton` text and icon parameters · text `impl Into<String>` / `WidgetText` / `IntoAtoms`; icon by value / `&Icon` · follows CP-01-2 and CP-01-3 _(tagged 4)_
- `CP-08-36` (select) · icon parameter form of `SelectItem::icon` and `CommandItem::icon` · `Icon` by value (`Icon` is `Copy`; follows CP-01-3) / `&'a Icon` · sketch: by value, following CP-01-3 (earlier draft took `&'a Icon`). _(tagged 1)_
- `CP-09-35` (dropdown-menu) · icon argument on `MenuItem::icon`, `MenuCheckboxItem`, `MenuSub::icon` · `&'a Icon` (sketch) / by value (CP-01-3) · follows CP-01-3 _(tagged 1)_
- `CP-10-26` (tooltip) · icon parameter form in the tooltip usage (`Button::icon`) · `&Icon` (today) / `Icon` by value (`Copy`) · by value, as CP-01-3 (the tooltip itself takes no icon). _(tagged 4)_
- `CP-11-14` (alert-dialog) · alert-dialog media · icon only / closure `media_ui` / icon + colour override for the destructive demo · sketch: icon only (destructive look open); icon form per CP-01-3; no `IconSlots` slot needed _(tagged 4)_
- `CP-13-33` (breadcrumb) · Icon parameter form of `Breadcrumb::separator` · `&'a Icon` (sketch, like today's `Button::icon(&Icon)`) / `Icon` by value (`Icon` is `Copy`) · `&'a Icon`. Crate-wide anchor CP-01-3, which proposes by value. If CP-01-3 is taken, the sketch becomes `separator(icon: Icon)` and loses its lifetime. _(tagged 4)_
- `CP-14-45` (sidebar) · Icon parameter form (`MenuButton::icon`, `MenuButton::action`, `Group::action`) · `Icon` by value / `&Icon` · sketch: by value. Crate-wide anchor: CP-01-3. _(tagged 4)_
- `CP-18-33` (marker) · icon form of `Attachment::icon`, `attachment_action`, `Marker::icon` · `&'a Icon` / by value · sketch: `&Icon`; follows CP-01-3 _(tagged 4)_

**G3 · Icon slot content**

- `CP-01-32` (button) · icon slot content in `Button` (loading spinner demo) · `Icon` only / `Glyph { Icon, Spinner }` via `impl Into<Glyph>` (CP-02-2) / atoms (CP-01-2 b) · sketch: `Icon` only; form per CP-01-3 _(tagged 4)_
- `CP-02-2` (badge) · what an icon slot accepts · `&Icon` only (today) / an enum `Glyph { Icon, Spinner }` shared with Button / atoms · `impl Into<Glyph>` _(tagged 4)_
- `CP-02-18` (spinner) · drawing · rotate the active icon set's spinner slot (upstream look, needs a rotation-capable `paint_icon`) / reuse egui's arc · rotate the slot icon _(tagged 4)_
- `CP-18-26` (marker) · Spinner in the icon slot · `.spinner()` setter vs. an icon slot taking any widget closure · `.spinner()`. The general question (icon slot content: `Icon` only per CP-01-32, `impl Into<Glyph>` per CP-02-2, or a separate `.spinner()`) is CP-18-35. _(tagged 4)_
- `CP-18-35` (marker) · icon slot content (attachment media, marker icon) · `Icon` only (CP-01-32) / `impl Into<Glyph>` (CP-02-2) / separate `.icon` + `.spinner()` · sketch: separate setters; decide with CP-01-32, CP-02-2 _(tagged 4)_

**G4 · `IconSlots` additions and the slot rule (anchor CP-05-6)**
See also (placed in another group): CP-11-14.

- `CP-05-6` (checkbox) · crate-wide anchor: indeterminate icon · new `IconSlots::minus` per library, `IconSlots` made `#[non_exhaustive]` / a painted line · sketch: new slot; input-otp separator (06) and menus (09) need the same slot _(tagged 4)_
- `CP-05-18` (radio-group) · Checked indicator? · painted filled circle from `cn-radio-group-indicator-icon` (the icon's stroke is the same colour inside the dot) / new `IconSlots::circle` slot · painted circle. _(tagged 3)_
- `CP-06-37` (input-group) · how the `InputOtpSeparator` minus is drawn (X5) · `IconSlots::minus` (CP-05-6) / painted line / caller content · sketch: `IconSlots::minus` _(tagged 4)_
- `CP-12-21` (accordion) · accordion chevron-up source · new `IconSlots::chevron_up` via icongen / rotate `chevron_down` 180° · sketch: new slot; the closed chevron cannot reuse Select's `chevron_down` for tabler, hugeicons (see CP-12-33) _(tagged 4)_
- `CP-12-33` (accordion) · slot rule for accordion chevrons · one slot per role (`accordion_open`/`_closed` or reuse Select's) / one slot per shape (`chevron_up` + a true `chevron_down`) · sketch: per shape (anchor CP-05-6, review Y1) _(tagged 4)_
- `CP-13-31` (breadcrumb) · icon slots for this cluster (anchor CP-05-6): `chevron_left`, `chevron_right`, `more_horizontal`, nav trigger chevron · reuse `chevron_down` (wrong for tabler, hugeicons) / new `chevron_down_plain` / rename Select's slot `select_trigger`; slot per shape vs per role differ (calendar hugeicons `ArrowLeftIcon`) · sketch: three new slots + a nav chevron slot _(tagged 4)_
- `CP-14-47` (sidebar) · sidebar trigger icon · new `IconSlots::sidebar` per library / fixed lucide icon · sketch: new slot (anchor CP-05-6) _(tagged 4)_
- `CP-15-36` (calendar) · calendar prev/next and caption icons · new `chevron_left`/`chevron_right` + calendar down chevron / one slot per shape with pagination, carousel (hugeicons names must unify) / one slot per role / lucide only · sketch: `chevron_left`, `chevron_right`, `chevron_down_plain` (CP-05-6, Y1) _(tagged 4)_
- `CP-16-41` (carousel) · carousel prev/next icons · new `chevron_left`/`chevron_right` slots shared with pagination, calendar (hugeicons names differ) / rotate `chevron_down` / lucide only · sketch: new slots (CP-05-6) _(tagged 4)_
- `CP-16-44` (sonner) · toast kind icons · five new `IconSlots` (shape names `loader`, `circle_check`, … or role names `toast_*`) / lucide only / painted shapes · sketch: new slots (CP-05-6); close reuses `IconSlots::close` _(tagged 4)_
- `CP-16-36` (sonner) · warning/error icon mapping (remixicon swapped between Svelte and React) · follow React / follow Svelte at the pin · sketch: React; moot unless CP-16-44 picks slots _(tagged 4)_
- `CP-19-39` (message-scroller) · scroll button arrow icon · new `IconSlots::arrow_down` / lucide always / painted · sketch: new slot (CP-05-6, Y1) _(tagged 4)_
- `CP-19-42` (questionnaire) · choice check icon · reuse `IconSlots::check` (same five per-library names as Checkbox) / Lucide `CHECK` always · reuse `IconSlots::check`; per anchor CP-05-6 no new slot is needed. The radio dot is painted (no icon upstream) _(tagged 4)_

**G5 · `State` flag budget (anchor CP-18-30)**
See also (placed in another group): CP-16-39.

- `CP-18-30` (marker) · crate-wide anchor: `State` flag budget (9 free bits; ~30 asked) · widen to `u64` / reuse flags / resolve variants in code by stacking recipes · sketch: widen to `u64`; referenced by 11, 12, 13, 14, 15, 16, 19 (batch-B N6) _(tagged 9)_
- `CP-01-17` (toggle) · State flag for pressed: new `PRESSED` (from `aria-pressed`, `data-[state=on]`) / map to existing `SELECTED` / `CHECKED` · sketch: new `PRESSED` _(tagged 9)_
- `CP-02-9` (badge) · `[a]:hover` rules once a link badge exists · keep ignored / translate `[a]` to a `LINK` state flag · translate (only if CP-02-3 ports links) _(tagged 9)_
- `CP-11-37` (sheet) · new state flags `SIDE_TOP/RIGHT/BOTTOM/LEFT` (sheet `data-[side=*]`, drawer `data-[vaul-drawer-direction=*]`) · four new flags shared by both / resolve the side in code by stacking per-side recipes / reuse nothing · four shared flags. Counts against the flag budget of CP-18-30. _(tagged 9)_
- `CP-11-38` (alert-dialog) · `HAS_MEDIA` flag (`has-data-[slot=alert-dialog-media]`, `group-has-data-[slot=alert-dialog-media]/…`) · new flag / resolve the media layout in code with a separate recipe · new flag. Counts against the flag budget of CP-18-30. _(tagged 9)_
- `CP-12-34` (accordion) · accordion `State` flags · `OPEN` (shared with navigation-menu, sidebar; Y12) + `NOT_LAST` / none, resolve in code · sketch: `OPEN`, no `NOT_LAST`; counts against CP-18-30 _(tagged 9)_
- `CP-13-9` (navigation-menu) · Active link style: vega and others use `data-[active=true]`, which never matches bits' `data-active=""`, while nova and lyra match · show the active background in all styles (stylegen maps both to SELECTED) / reproduce the web (no background in vega and others) · show it in all styles. _(tagged 1)_
- `CP-13-36` (navigation-menu) · navigation `State` flags (open trigger, in-content link, no-viewport) · three new / reuse `EXPANDED` + two new / resolve in code / port only `viewport = true` · sketch: reuse `EXPANDED` + `IN_NAV_CONTENT`, `NO_VIEWPORT`; counts against CP-18-30 _(tagged 9)_
- `CP-14-48` (sidebar) · sidebar `State` flags (~10: collapse modes, variants, `SIDE_RIGHT`, `HAS_ACTION`, `PEER_HOVER`, `OPEN`, sizes) against 9 free bits · add flags / widen `State` / resolve in code · sketch: no pick (anchor CP-18-30; batch-B N4) _(tagged 9)_
- `CP-15-38` (calendar) · calendar flag budget: `TODAY`, `OUTSIDE_MONTH`, `UNAVAILABLE`, `RANGE_START/END/MIDDLE`, `FIRST_IN_ROW`, `LAST_IN_ROW` (up to 8 bits) · new flags (needs CP-18-30) / resolve in code / reuse flags · sketch: new flags _(tagged 9)_
- `CP-19-18` (message-scroller) · `data-[active=true]` maps to `SELECTED` today with another meaning · reuse `SELECTED` / new flag `ACTIVE_EDGE` · new flag _(tagged 9)_
- `CP-19-36` (questionnaire) · indicator type flag for the generator · two flags `RADIO`/`CHECKBOX` / one `MULTIPLE` flag (radio = not multiple) · one `MULTIPLE` flag (sera's `group-data-[type=checkbox]` becomes `MULTIPLE`). Counted against CP-18-30 in CP-19-41 _(tagged 7)_
- `CP-19-40` (message-scroller) · scroll button state flags (`data-active`, `data-direction`) · `ACTIVE_EDGE` + `EDGE_START` / reuse `SELECTED`, direction in code / count against CP-18-30 · sketch: two new flags _(tagged 9)_
- `CP-19-41` (questionnaire) · questionnaire flag budget (~7: `ALIGN_END`, `GHOST`, `ACTIVE_EDGE`, `EDGE_START`, `MULTIPLE`, `HAS_DESCRIPTION`, `HAS_SHORTCUT`) · widen `State` (CP-18-30) / reuse flags / resolve in code · sketch: follows CP-18-30 _(tagged 9)_

**G5b · State bits: flag mapping, generator and recipe fidelity (T9 G10, owned here)**

- `CP-01-18` (toggle) · does `Toggle::icon()` set `ICON_START` (Button does, toggle demos keep `px-2.5`) · always / never / separate `.icon_padding()` · sketch: never _(tagged 9)_
- `CP-06-17` (input-otp) · Active-cell ring · port React's intent (`SELECTED` on the active cell, ring drawn) / port shadcn-svelte literally (never drawn, bits sets `data-active=""`) · pick: React's intent. _(tagged 9)_
- `CP-09-32` (menubar) · Disabled menubar checkbox item fade · reproduce upstream (no `opacity-50` in any style) / fade like dropdown/context · reproduce upstream (recipes as generated); note egui's own disabled-`Ui` fade still applies when disabled through the `Ui` (SKILL.md "Interaction states"). _(tagged 9)_
- `CP-09-33` (menubar) · Keyboard focus on a closed trigger has no visible style upstream · reproduce (nothing drawn) / add a focus ring · reproduce. _(tagged 9)_
- `CP-14-24` (sidebar) · rhea outline variant: reproduce the browser's dropped shadow (invalid `hsl(oklch)`) or draw the intended 1px ring · sketch: draw the intended ring (stylegen translates the arbitrary shadow) _(tagged 9)_
- `CP-15-27` (calendar) · `data-[selected]:hover:text-foreground` paints near-black text on near-black `bg-primary` in light neutral themes · port as written / drop the hover text change · pick: port as written (flag to maintainer). _(tagged 9)_
- `CP-15-34` (range-calendar) · unavailable day look differs (calendar `text-muted-foreground line-through`, range `line-through` only) · per mode as written / unify · pick: unify on calendar (follows CP-15-16). _(tagged 9)_
- `CP-15-35` (range-calendar) · month/year select background differs (calendar's hidden `<select>` has `bg-background dark:bg-popover`, range's none) · irrelevant once the native select is replaced (CP-15-9) / mirror · pick: irrelevant, use the Select popup look. _(tagged 9)_
- `CP-16-9` (table) · `[&:has([role=checkbox])]:pr-0` · (a) a `Column::checkbox()` kind that drops the right padding; (b) the cell detects a checkbox (impossible in egui); (c) drop the rule · (a) _(tagged 9)_
- `CP-16-22` (carousel) · Generator handling of the ternary classes · (a) the component hard-codes slide geometry as behaviour constants (`// behaviour-ok`), stylegen reads only the two markers; (b) teach the extractor ternaries and add logical-side/rotate/basis rules · (a) _(tagged 9)_
- `CP-18-22` (bubble) · Tinted colour · new relative-OKLCH `Paint` in stylegen vs. per-palette precomputed role · new `Paint` kind. _(tagged 9)_
- `CP-18-31` (marker) · `*:data-[slot=X]:` and `before:` (cluster-wide generator) · retarget child-slot rules to the child part's recipe and add a `_before` recipe vs. hand-written recipes · generator retarget + `_before`. _(tagged 9)_

**G6 · Binding setter name (anchor CP-14-43)**

- `CP-14-43` (sidebar) · crate-wide anchor: name of the "memory with optional caller binding" setter · `.open_bool` (CP-10-2, egui `Popup::open_bool`) / `.open` (egui `Window::open`) / one name per state kind (`.open`, `.layout`, `.view_month`) / common `.bind(&mut T)` · sketch: no pick (batch-B N4) _(tagged 2)_
- `CP-10-2` (popover) · who owns the open flag · egui memory only (today) / memory + optional `.open_bool(&mut bool)` / caller `&mut bool` always (like `Dialog::show`, `dialog.rs:69-74`) · memory + optional `.open_bool`. _(tagged 2)_
- `CP-14-1` (sidebar) · sidebar open state owner · caller `&mut bool` / memory + `default_open` / memory, `.open(&mut bool)` overrides · sketch: memory + `.open`; the CP-10-2 rule on a panel, setter name per CP-14-43 _(tagged 2)_
- `CP-14-33` (resizable) · Layout state owner · (a) egui memory, typed state (b) caller `&mut [f32]` (c) memory by default, `.layout(&mut Vec<f32>)` overrides · sketch: (c) _(tagged 2)_
- `CP-15-6` (calendar) · visible month owner · memory only / caller `&mut Date` / memory + optional `view_month(&mut Date)` · sketch: memory + binding (CP-10-2 rule on a view value; name per CP-14-43) _(tagged 2)_
- `CP-13-2` (navigation-menu) · who owns the navigation-menu open item · memory / caller `&mut Option<usize>` / memory + optional binding (CP-10-2) · sketch: memory; follows CP-10-2 if CP-10-25 picks one overlay shape _(tagged 2)_
- `CP-09-4` (dropdown-menu) · menu open state owner · egui memory / `&mut bool` / both (`.open_bool`) · sketch: memory only; follows CP-10-2 _(tagged 2)_
- `CP-10-25` (popover) · do menu contents (09) follow the overlay shape of CP-10-2 and CP-10-3 · yes / no, menus keep `Option<R>` and memory only / only one of the two · sketch: yes; decide with CP-09-3, CP-09-4 _(tagged 2, 5)_
- `CP-11-35` (dialog) · modals vs the anchored-overlay rule CP-10-2 · keep the caller `&mut bool` argument to `show` / follow CP-10-2 (memory + optional `.open_bool`, `Dialog::open(ctx, id)`) / required `.open_bool` setter · sketch: keep; applies to all four modals and the sidebar mobile Sheet _(tagged 2)_

**G7 · Text parameter type**
See also (placed in another group): CP-06-36.

- `CP-01-2` (button) · Button content type? · (a) `impl Into<String>` + `.icon/.icon_end` (today) (b) `impl IntoAtoms` like egui `Button` (crate icons are not egui `Image`s, would need `Atom::custom`) · sketch: (a) _(tagged 4)_
- `CP-02-1` (badge) · type of the text parameter · `impl Into<String>` (today) / `impl Into<WidgetText>` / `impl IntoAtoms<'a>` (icons as atoms) · `impl Into<WidgetText>` (non-clickable text; icons stay typed slots) _(tagged 4)_
- `CP-03-9` (card) · text parameter type · `&str` (today) / `impl Into<WidgetText>` · `impl Into<WidgetText>` _(tagged 4)_
- `CP-04-3` (alert) · Text parameter type · `impl Into<WidgetText>` (egui labels/titles) vs `impl Into<String>` / `&str` (crate's `Badge::new`, `Label::new`, `card_title`) · sketch: `impl Into<WidgetText>`. _(tagged 4)_
- `CP-06-2` (input) · Hint parameter type and name · `hint(impl Into<String>)` (today) / `hint(impl Into<WidgetText>)` / `hint_text(impl IntoAtoms<'static>)` (TextEdit) · pick: `hint_text(impl IntoAtoms<'static>)`. _(tagged 1)_
- `CP-07-4` (label) · Label text type · `impl Into<String>` (today) vs `impl Into<WidgetText>` (egui labels) · sketch: `Into<WidgetText>`. A `RichText` colour or weight then has to beat the recipe; which one wins needs its own rule. _(tagged 1)_
- `CP-08-14` (select) · item label type · `&str` / `impl Into<WidgetText>` / `IntoAtoms` (icon + text in one) · sketch: `&str` + optional `.icon` (typeahead needs plain text). _(tagged 1)_
- `CP-08-28` (command) · item content type · `&str` label + `.icon` + `.shortcut` / `IntoAtoms` content with a separate `.value` for filtering · sketch: `&str` (the filter needs text). _(tagged 4)_
- `CP-09-7` (dropdown-menu) · Item content type · `impl Into<String>` + `.icon(&Icon)` (crate Button) / `impl IntoAtoms` (egui `menu_button`, `Button`) · `impl Into<String>` + `.icon`. _(tagged 1)_
- `CP-10-9` (tooltip) · content type · text only (today) / closure `|ui|` (needed for the Kbd demo) / both · both (shortcut = text, builder = closure). _(tagged 4)_
- `CP-11-4` (dialog) · text parameter type for title/description · `&'a str` (today, matches `card_header`) / `impl Into<WidgetText>` (egui idiom for titles) · `impl Into<WidgetText>`. _(tagged 4)_
- `CP-12-3` (tabs) · trigger content type · `&str` (today) / `impl IntoAtoms` (icons, `tabs-icons.svelte`) / `impl Into<WidgetText>` · `IntoAtoms` (clickable body with icon, per idioms §Text). _(tagged 4)_
- `CP-12-18` (accordion) · trigger content type · `impl Into<WidgetText>` (wrapping label text) / `impl IntoAtoms` (icons) · `Into<WidgetText>`. _(tagged 4)_
- `CP-13-13` (navigation-menu) · Link content: title plus optional description (demo ListItem), with an icon via atoms (demo "With Icon") · `NavigationMenuLink::new(atoms).description(text)` / arbitrary closure content (`NavigationMenuLink::show(ui, |ui| …)`) · atoms plus description. _(tagged 4)_
- `CP-13-20` (breadcrumb) · Text parameter type · `Into<WidgetText>` (label-like, as `Hyperlink`) / `IntoAtoms` (allows an icon in a link) · `Into<WidgetText>`. _(tagged 4)_
- `CP-14-22` (sidebar) · Menu button content type · (a) `impl Into<String>` + `.icon` (crate today) (b) `impl IntoAtoms` · sketch: (a) (same question as CP-01-2) _(tagged 4)_
- `CP-16-6` (table) · Cell content type · (a) both `text(impl Into<WidgetText>)` and `cell(|ui| ..)`; (b) closure only; (c) `IntoAtoms` · (a) _(tagged 4)_
- `CP-18-24` (marker) · Content type · `Into<WidgetText>` + `.icon(&Icon)` vs. `impl IntoAtoms` vs. a `show` closure · `WidgetText` + `.icon`. _(tagged 4)_
- `CP-19-2` (message) · header content type · `impl Into<WidgetText>` (all demos pass text) / closure · `WidgetText` _(tagged 4)_
- `CP-19-33` (questionnaire) · choice label with two lines (bold title + muted description; demos use spans, the `ChoiceDescription` part exists) · `Choice::description(..)` / `Choice::new(impl IntoAtoms)` · `description(..)` _(tagged 4)_

**G8 · Constructor shape**
See also (placed in another group): CP-01-24, CP-02-14, CP-02-23, CP-04-9, CP-05-25, CP-11-18, CP-12-16, CP-13-22, CP-14-13, CP-14-36, CP-15-2, CP-15-31, CP-05-12, CP-07-32, CP-08-10, CP-09-30, CP-10-17, CP-11-8, CP-12-11, CP-15-12, CP-18-11.


**G9 · Name clashes**

- `CP-01-15` (toggle) · Shortcut name: `toggle` is taken by the switch shortcut (`switch.rs:182`) · rename switch's to `switch` and use `toggle` / `toggle_button` / `toggle_value` (egui's name) / none · sketch: `toggle_button` _(tagged 10)_
- `CP-05-22` (switch) · Shortcut name? · `toggle` (today; egui's `toggle_value` is a different widget, a selectable button) / `switch` (upstream name; not a Rust keyword) · keep `toggle`. _(tagged 10)_
- `CP-12-28` (collapsible) · collapsible open state owner · caller `&mut bool` / memory by id for trees (`CollapsingState`) · sketch: caller; sidebar sub-menus (CP-14-11) then keep their own bools; name clash with `SidebarCollapse` (CP-14-46; batch-B N1) _(tagged 2)_
- `CP-14-46` (sidebar) · name of the sidebar collapse-mode enum · `Collapsible` (clashes with the collapsible component; `None` variant shadows `Option::None`) / `SidebarCollapse` / `SidebarCollapsible` with `Fixed` · sketch: `SidebarCollapse` (renaming `None` is part of this CP) _(tagged 4)_
- `CP-02-22` (spinner) · shortcut name `spinner(ui)` next to egui's `ui.spinner()` · keep / drop the shortcut · keep (`shadcn_egui::components::spinner::spinner`) _(tagged 10)_
- `CP-05-19` (radio-group) · Shortcut name? · `radio_value` (egui's name) / `radio_group_item` / `radio` · `radio_value`. _(tagged 10)_
- `CP-09-23` (context-menu) · Entry point · `ContextMenu::new(&resp).show(..)` builder (crate Popover form) / an extension trait so `resp.context_menu(..)` keeps egui's spelling but draws shadcn / both · builder + free `context_menu(&resp, ..)`. _(tagged 5)_
- `CP-10-8` (tooltip) · entry point · free fn `tooltip(resp, text)` only (today) / extension trait method `resp.tooltip(text)` / builder `Tooltip::new(&resp)` + keep the free fn · builder + free fn. _(tagged 10)_
- `CP-16-38` (sonner) · Module and type names · (a) `components::sonner` with `Toaster`/`Toast`/`toast()`; (b) `components::toast` · (a) _(tagged 4)_

**New choice points from the topic cross-check (no sketch pick)**

- `CP-T11-1` · does every builder setter carry `#[inline]` · yes (sketches 01, 02, 05, 14; egui) / none (03, 04, 06–13, 15–19) / leaf widgets only · idiom §Widget builders; batch-A X10
- `CP-T11-2` · `#[must_use]` scope and message · every builder incl. data items and sub-part builders / only types that do nothing unless consumed / as sketched (missing on Accordion, Table, Carousel, Toast, Toaster, ChartLegend, CommandDialog); message for containers whose consuming method is not `show` (`show_inside`, `show_ui`, `cmd.add`): name it / one generic message · idiom §Widget builders, §Containers
- `CP-T11-3` · do public enums derive `Default` with `#[default]` · every public enum / only where a builder needs it / as sketched (15 without, e.g. `AlertVariant`, `AvatarSize`, `ToastKind`); one crate derive list (`Clone, Copy, Debug, Default, PartialEq, Eq`, sometimes `Hash`) · no egui idiom; report-04 Minor
- `CP-T11-4` · secondary text parameters (part fns, shortcut data, `alt`, `error`, item labels) · same type as the main text (G7) / `&str` or `impl Into<String>` where the text is also data, `WidgetText` where only drawn / as sketched (`Avatar::alt`, `Field::error`, `menu_label(ui, &str)`, `SelectItem::new(.., &str)`, …) · idiom §Text
- `CP-T11-5` · name of the empty-field hint · `hint_text` (egui TextEdit; Input, Textarea sketch) / `hint` (crate Input today) / `placeholder` (upstream; Select, Command) / `placeholder` for pick-lists, `hint_text` for text fields · idiom §TextEdit; batch-A X7
- `CP-T11-6` · constructor naming for modes and alternative content · `new` = common mode + named others (`Calendar::range`, `Slider::multiple`, `Progress::from_value`, `Pagination::from_count`) / named for every mode, no `new` (`ToggleGroup::single`, `Accordion::multiple`) / one `new` + mode setter; alternative content: constructor (`Kbd::icon`, `InputGroupButton::icon`) / `new` + `.icon()` · idiom §Widget builders; batch-B report-15 Minor
- `CP-T11-7` · type names equal to egui's (`Button`, `Checkbox`, `Label`, `Separator`, `Slider`, `Spinner`, `Tooltip`, `ScrollArea`; today `SliderOrientation`) · keep, callers use paths or `as` / keep, but never for helper types with another meaning (`SliderOrientation`) / prefix or distinct name for clashes · no egui idiom
- `CP-T11-8` · names of free part fns and associated fns · component prefix, glob-safe (`button_group_separator`, `card_title`, `bubble_group`) / generic names, module-qualified (`dialog::footer`, `field::error`, `sidebar::input`), glob imports discouraged / methods on the component or a handle; also `Sidebar::toggle` beside the `toggle` shortcut · no egui idiom; batch-A report-07 Minor
- `CP-T11-9` · positional `bool` parameters · never; setters or an enum (`MessageScrollerUi::item(.., anchor, ..)`, `FieldCard::new(selected)`, `tab_enabled`, `row_with`, `MenuButton::action`, `menu_skeleton`) / allowed where per call and data-driven / as sketched · idiom §Widget builders (drift `shared.rs` `clip_padding`; same as CP-T9-8)
- `CP-T11-10` · flags whose default is true · positive name + `bool`, call with `false` (`numbers(false)`, `viewport(false)`, `close_on_select(false)`) / negative no-argument name (`hide_close_button()`, `non_modal()`) / per flag; and one name for the corner close button: `show_close_button` (Dialog, Sheet) vs `close_button` (Toaster) · no egui idiom
- `CP-T11-11` · colour parameter type · `impl Into<Color32>` (`Spinner::color`, egui `Image::tint`) / `Color32` (`Badge::fill`, `text_color`, `AvatarBadge::fill`, `IconView::color` today) · no listed idiom
- `CP-T11-12` · variant in a shortcut fn · content only, variant via the builder (`badge(ui, text)`, CP-02-6) / positional variant (`bubble(ui, text, variant)`, `badge` today) · idiom §Widget builders (same as CP-T10-4)
- `CP-T11-13` · name of the plain down-chevron slot (`IconSlots::chevron_down` holds Select's selector icon for tabler, hugeicons) · new `chevron_down_plain` (CP-13-31, CP-15-36) / rename the Select slot `select_trigger`, `chevron_down` becomes the true chevron (CP-12-33) / one `chevron_down`, accept Select's icon · users: accordion, navigation-menu, calendar; renaming breaks `IconSlots` (batch-B N5)
- `CP-T11-14` · one flag for `data-open` (accordion item, nav trigger, sidebar menu button) · new `OPEN` (CP-12-34, CP-14-48) / reuse `EXPANDED` (CP-13-36) / resolve in code · batch-B Y12
- `CP-T11-15` · meaning of `SELECTED` · one meaning, new `PRESSED` (CP-01-17) and `ACTIVE_EDGE` (CP-19-18) / also for `aria-pressed`, `data-active` (CP-13-9, sidebar) and the scroll button, saving bits
- `CP-T11-16` · side flags for sheet, drawer, sidebar · four `SIDE_*` (CP-11-37) / only the sides a recipe reads (`SIDE_RIGHT`, CP-14-48) / resolve the side in code · counts against CP-18-30
