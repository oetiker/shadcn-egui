# Changelog

All notable changes to the `shadcn-egui` crates and the Claude Code plugin
are recorded here. The format is based on
[Keep a Changelog](https://keepachangelog.com), and the project aims to follow [Semantic Versioning](https://semver.org).

## [Unreleased]

### New
- Every component takes app variants, and Button, Card, Switch and Select also app sizes: `const DANGER: AppVariant<ButtonVariant> = ButtonVariant::app("danger");` names one, `Theme::vega().variant(DANGER, ButtonVariant::Ghost, |r, _style| r.button.recolor(Role::Foreground, Role::Destructive))` defines it on the theme from a built-in variant (`Theme::size` for a size), and `Button::new("Delete").variant(DANGER)` draws it. The edit gets one recipe per part of the component (`ButtonRecipes`, `CardRecipes`, `TabsRecipes`, …), changes values with `prop.set(..)`, `prop.on(state, ..)` and `recipe.recolor(from, to)`, and keeps everything else of the base in every style, state and mode; card, dialog and popover parts take their parent's variant. Components that had no variant now have one with only `Default` and a `.variant(..)` setter: `CheckboxVariant`, `InputVariant`, `LabelVariant`, `SeparatorVariant`, `SwitchVariant`, `SelectVariant`, `SliderVariant`, `CardVariant`, `DialogVariant`, `PopoverVariant` and `TooltipVariant`.
- App colours give app variants a colour no palette role has: `Theme::color(BRAND, light, dark)` defines one for both modes, with `const BRAND: Role = Role::custom("brand");`. A name the theme does not define, as variant or as colour, stops a debug build with an assertion and draws as the `Default` variant or the foreground colour in a release build.
- `Tooltip::new(&resp).show(ui, |ui| ..)` shows any content in a tooltip, with `.variant(..)` for an app variant; it opens as `tooltip(resp, text)` does, which stays as the shortcut for a text.
- `ui.add(Separator::new())` draws a separator, with `.orientation(SeparatorOrientation::Vertical)` and `.variant(..)`; `separator(ui)` and `separator_vertical(ui)` stay as shortcuts.
- Text a style marks `text-balance` or `text-pretty` breaks its lines where Chrome does: balanced text of up to six lines gets lines of about equal length, and pretty text no longer leaves a single short word alone on its last line. Text that Chrome leaves alone (one line, more than six lines for balance, a word wider than the box) wraps as before.
- Components follow the window width as shadcn's `sm:` (640 px) and `md:` (768 px) breakpoints do: below 768 px an input's text is 16 px instead of 14 px, and below 640 px a dialog spans the window less a 16 px margin on each side. `layout::window_at_least` gives an app the same check on the window width, and `layout::container_at_least` on the width left in its `Ui`.
- `IconAtom::new(icon)` puts a crate icon wherever egui takes atoms, such as an `egui::Button`'s content. Without `.tint(..)` it draws in the theme's text colour, and a crate widget draws it in its own state colour; a text colour that a host `Ui` overrides does not reach it, so tint it there.
- `layout::capped_width`, `layout::overlay_width` and `layout::allocate_overflowing` give an app's own widget the crate's width rules: a width capped to the room or the window and raised to the content's minimum, and a widget wider than its room that overflows instead of widening its column.
- `layout::allocate_item` and `layout::allocate_item_space` place an app's own widget in a `layout::wrap_row` as a crate widget is placed: a widget that does not fit the rest of its line starts the next line, and that line is as high as its own tallest widget. They take the place of egui's `allocate_exact_size` and `allocate_space`; a line that a widget allocated with those starts keeps the height of a taller line before it.
- `components::shared::widget_info_with_role` gives an app's own widget a screen-reader role that egui's widget types lack, such as alert, alertdialog, navigation or group.
- `layout::stretch_height(ui)` gives a widget in a `layout::row`, `layout::space_between` or a card footer the height of the row's tallest other widget, as `self-stretch` does in shadcn, for example for a divider between keys. In a `layout::wrap_row` it gives the height of the tallest other widget on the widget's own line. It gives no height in a column or outside these rows.

### Changed
- A vertical separator in a `layout::row`, `layout::space_between` or a card footer (`separator_vertical(ui)`, or `Separator` with `SeparatorOrientation::Vertical`) is as tall as the row's tallest other item, as in shadcn; in a `layout::wrap_row` it is as tall as the tallest other item on its line. Before, it took the row's whole height, which never drops below egui's widget height, so beside a label it stood taller than the text, 32 px instead of 14 px in the gallery.
- A theme holds a light and a dark palette and follows egui's mode: apply it once and switch with `ctx.set_theme(egui::Theme::Light)`, or follow the system with `egui::ThemePreference::System`; crate widgets and plain egui widgets change without a second `apply`. `Theme::vega_dark()` and `Theme::vega_light()` are now one `Theme::vega()`, `Preset::theme(mode)` and `Preset::apply(ctx, mode)` lose the mode argument, and `preset::Mode` is gone.
- `Theme::new(style, light, dark, radius)` takes both palettes. The `theme.palette` field is now the two fields `theme.light` and `theme.dark`, with `theme.palette()` for the palette of the current mode and `theme.palette_of(mode)` for either; the boolean `theme.dark` is now `theme.is_dark()`. `MenuColors` and `Theme::menu(..)` are replaced by `Theme::menu_inverted(true)`, which draws menus in dark mode in both modes.
- Text has the ink of Chrome's text, within 5 % in light and in dark mode. Before, text in dark mode drew 3 to 25 % heavier, and a light theme in a context left in egui's dark mode drew its text 10 to 24 % heavier.
- `ButtonVariant`, `ButtonSize`, `BadgeVariant`, `TabsVariant`, `CardSize`, `SwitchSize` and `SelectSize` now hold app cases and are `#[non_exhaustive]`: a `match` on one outside the crate needs a `_` arm. Their setters, `badge(ui, text, variant)` included, take `impl Into<..>`, so they take a built-in case or an app constant, and existing calls compile unchanged. `ButtonVariant`, `ButtonSize` and `BadgeVariant` now implement `Default`.
- `reference::GalleryState::new` and its `mode` field take an `egui::Theme` in place of the removed `preset::Mode`.
- Every icon library now supplies a plain down chevron, up, left and right chevrons and a three-dot icon to `Theme::icons`, as upstream's components use them. The Select trigger icon moved from `chevron_down` to `select_trigger`, and an app builds its own icon set from `IconSlots::all(icon)` instead of listing every slot, so later slots do not break it.
- The browser gallery's icon card shows every icon slot and wraps them onto several lines.
- `Theme::state` takes the egui context as its first argument, to read the window width.
- `State` no longer exposes its bits: combine flags with `|` or `State::or` and drop one with `State::without`. `Shift::SelfMinus(n)` is now `Shift::Calc(1.0, -n)`, which also expresses a share of the part's own size such as `translate-x-1/2`.
- `components::shared::RecipeBox` replaces `box_shapes`, `paint_box`, `ring`, `ring_shape` and `corner` for apps that paint their own parts: `RecipeBox::resolve(ctx, &theme, &recipes, state).paint(&painter, rect)` paints fill, border, shadow, ring and outline from a recipe stack, with each corner's radius and each side's border. `components::shared::line_width` gives a border width on the screen's device pixels, and `components::shared::inset_frame` an egui `Frame` that insets content by a fraction of a point.
- `components::shared::part_painter`, `part_opacity`, `part_ui` and `content_ui` replace `style_painter`, `ui_opacity` and `faded`: a part now paints at its own opacity with egui's disabled fade taken out once, and `content_ui` fades a container's content while plain egui widgets in it keep the theme's disabled fade.
- `theme.style` recipes carry more of each style: corner radii per side, the left border, column gaps, text wrapping and alignment, blend modes, visibility, and the enter, exit and transition motion values.
- `Theme::shadow` returns every layer of a Tailwind shadow as `ShadowLayer`s (offset, blur, spread, colour, alpha) instead of one egui `Shadow`.
- `components::shared::interaction_state` no longer takes an `enabled` argument: it reads the disabled state from the widget's `Ui`. Apps that paint their own clickable parts call `pressable(&mut response, ActivationKeys::..)` for focus on press and the element's own keys, and use `text_entry_state` for a field that takes typed text.
- A mouse or touch press now focuses buttons, checkboxes, switches, slider thumbs, tab triggers and the select trigger without showing a focus ring, as the browser does; the ring shows only after the focus moved by keyboard, while an input shows it on any focus.
- Keys act as on shadcn's elements: Space clicks a button when released, Enter no longer toggles a checkbox, Enter and Space no longer move a focused slider to the mouse pointer, and pressing a tab selects it on the press.
- `Input::id_source` is removed; use `Input::id_salt`, which takes the same string.
- `components::shared::line_box` is removed. `components::text::TextLook` resolves a part's text from its recipes and lays it out and paints it as the crate's own widgets do: `.label(..)` adds it as an egui label, `.layout(..)` returns a `TextBlock` to paint, `.color(..)` sets its colour, `.resolved_color(ui)` reads it and `.leading(Leading::Field)` places it as in a text field; `TextLook::line_height` is the height of one line.
- `theme.style` recipes carry `underline_offset`, the `underline-offset-*` distance of a part's underline below the baseline.
- `components::text::content_scope` hands a container's text size, line height, colour, font weight, letter spacing and upper case to the crate text inside it; text that sets its own keeps it.

### Fixed
- In a `layout::wrap_row`, items placed before a taller item are now centred on their line, as in a `layout::row`; before, they sat at the top of the line, as the small icon buttons before the large one did in the gallery. A line that starts with a shadcn-egui widget or a `layout::vstack` is now as high as its own tallest item, even after a taller line; before, it kept the taller line's height. A line that starts with a plain egui widget still keeps the height of a taller line before it and centres its items in that height.
- A plain egui label in a `layout::wrap_row` now takes its own height and is centred on its line, and a label that does not fit the rest of a line moves to the next line whole; before, a label after a taller item stood as high as that item, and a long label continued on the next line as running text. A plain egui label longer than a whole line now runs past the line's end instead of wrapping, where shadcn would wrap its text.
- A shadcn-egui `Label` longer than a whole line of a `layout::wrap_row` now takes the line's width and wraps its text inside it, as in shadcn, and the next item starts the line below it. Before, it ran past the line's end on one line, 355 px wide in a 200 px row.
- A shadcn-egui `Label` wider than its column now wraps its text at the column's width, as in shadcn; before, it ran past the column on one line, 355 px wide in a 120 px column. In a `layout::row`, `layout::space_between` or a card footer it shrinks to the room the other items leave, not below its longest word, and wraps there, as a `layout::vstack` of text does. In a plain egui `ui.horizontal` it follows the text wrap mode of that `Ui`, as an egui label does, and stays on one line by default.
- A horizontal separator in a `layout::vstack` that fits its `layout::row`, `layout::space_between` or card footer now spans the stack, as in shadcn; before, it had no length there. Where the text of such a stack changes, the change shows on the next frame without the text first appearing wrapped at the old width.
- A `layout::vstack` of text that fits its `layout::row`, `layout::space_between` or card footer no longer lays out every frame twice and no longer keeps the app redrawing while nothing changes. Before, this happened where the stack's widest text was less than half a pixel wider than a whole number of pixels, such as 243.06 px.
- A horizontal separator in a `layout::vstack` inside a `layout::row` no longer stops the app with "Negative size: [NaN 0.0]", for the crate's `separator(ui)` as for egui's `ui.separator()`. The crate's separator there has no length, so it does not widen the stack; egui's makes the stack take the room the row's other items leave, as it fills a plain egui row.
- After a theme with another body font is applied, `layout::vstack`s that wrap their text in a `layout::row`, `layout::space_between` or card footer share the row's width by the new font's text widths. Before, where a stack kept its size across the switch, the row kept the old font's split, so a stack could wrap text it had room for.
- A card in a `layout::row`, `layout::wrap_row` or `layout::space_between` is now as wide as its content and shrinks, wrapping its text, when the row has no room for it, as shadcn's card, which sets no width, does in a flex row; before, it took the whole rest of the row, pushing a button after it past the row's end or onto the next line of a `wrap_row`. In a horizontal `ScrollArea` a card now takes its content's width instead of only its padding. In a column a card still spans the available width.
- A `layout::row`, a `layout::wrap_row` or a `layout::vstack` in either, shown in an `egui::Area` that measures its size again (`Area::sizing_pass(true)`), no longer lays out the measuring frame twice and the frame after it twice. Before, each such measurement cost two extra layout passes.
- A card title, description or content in the footer of a small card (`CardSize::Sm`) now takes the small card's look, as in its body, for example its smaller title text; before, the footer drew them at the default size.
- Text inside popover, tooltip, dialog and card content now takes the container's font weight, letter spacing and upper case, as in shadcn, as it already took its size and colour; text that sets its own keeps it.
- The underline of a hovered link Button or link Badge sits where Chrome draws it: 4 px below the baseline, as the style's `underline-offset-4` sets, and a tenth of the font size thick (1 device pixel at 1x, 2 at 2x). Before, it was one border width thick at the bottom of the line.
- At a display scale of 1.5, borders are as wide as Chrome draws them: a 1 px border is one device pixel wide instead of one and a half, and a 2 px border three. The text in an input and the content of popovers, dialogs, tooltips and select lists no longer start one device pixel right of where Chrome puts them.
- Crate text is spaced as Chrome spaces it: letters keep the widths of the weight on screen and kerned pairs such as "Te" or "fo" sit closer, also where the colour changes between the two letters. Before, text in Figtree, Manrope, Merriweather, Montserrat, Nunito Sans, Outfit, Oxanium, Public Sans, Raleway, Source Sans 3, Space Grotesk and DM Sans, and every bold or medium text, took the letter widths of another weight, up to 20 px off over a 110-character line. Buttons, badges, tabs and select values sized by their label are therefore up to 4 px wider or narrower than before.
- Text in Inter, DM Sans, Merriweather and Nunito Sans takes the letter shapes Chrome draws for its size, as these fonts carry a design per size: Inter headings and 16 px inputs are slimmer and tighter, and DM Sans text at every size is no longer drawn in its 9 px design.
- In an Input, the caret, the selection and a mouse click land between the characters where Chrome puts them. Before, they stopped up to one device pixel left of the character edge, and the width of crate text could end up to one device pixel short.
- Wrapped crate text breaks its lines where Chrome breaks them: spaces at the end of a line no longer push the last word to the next line, and a word wider than the box overflows instead of being split.
- Wrapped crate text also breaks where Chrome does after an em or en dash, at a zero-width space, at a soft hyphen and between Chinese or Japanese characters. At a soft hyphen that ends a line it draws a hyphen, as the browser does, and elsewhere a soft hyphen stays invisible. Selected or copied text is the text as written, without the drawn hyphen or an added line break.
- A line of wrapped text ends at the same word as in Chrome when the box is not a whole number of pixels wide or a kerned pair sits at the line end; before, a line could take one word more or less than in the browser, as in Playfair Display, Lora and IBM Plex Sans.
- In fonts with a weight axis, such as Nunito Sans and Source Sans 3, kerned pairs sit exactly as close as in Chrome; before, some pairs stood a fraction of a pixel further apart, enough to end a line one word early. Text in these fonts can shift by one device pixel.
- Balanced text (`text-balance`) breaks as in Chrome at every display scale, including 1.25 and 1.75; before, a balanced line at such a scale could end one word off.
- Text sits on the pixel row Chrome puts it on: in Inter, badges, tooltips, extra-small buttons and the text in inputs (typed and placeholder) moved 1 px down, card titles 3 px down, descriptions and help text 2 px down and dialog titles 1 px up, while buttons, tabs, select triggers, select lists and labels kept their place. With another font, text moves up to 2 px up or down, as the font crates no longer carry a per-font vertical offset (`FontFiles::y_offset_factor` is 0 for every font).
- Text on a line height that is not a whole pixel, such as descriptions in the lyra and mira styles and small buttons in nova, sits on Chrome's pixel row on every wrapped line, and what follows it starts where the browser starts it; before, the third line of a lyra card description sat 1 px low. Selecting text in a label highlights the line the text sits in, not a band up to 2 px lower.
- Wrapped text on a line height that is not a whole pixel, placed below other such text, puts every line on Chrome's pixel row; before, a line could sit 1 px low, as the second line of the notification card's description in the lyra and mira styles at phone width.
- A card footer's text now takes the card's text size, not only its colour.
- The text of a small nova button sits 1 px higher at 1.5x display scale, on Chrome's row: its line height is now its 12.8 px text size times the ratio of `text-sm`, as in the browser. Mira badges and keyboard keys inside a container with its own text size take that container's line height.
- An input's caret is 1 px wide in the text colour and as tall as the font, and its selection highlight is as tall as the text line, as in Chrome; before, the caret was 2 px wide in the ring colour and both ran 3 to 6 px taller. On a phone-width window the text in nova and rhea inputs moved 1 px up to Chrome's row. Where a font's line is taller than the input's content box, as Source Sans 3 in a phone-width nova or rhea input at 1.5x and 2x, the text, caret and selection now centre on Chrome's rows instead of sitting 1 px low.
- Plain egui text fields under the theme draw a 1 px caret in the text colour.
- The label of a `layout::form_row` now has the look of a `Label`: the style's label font, weight and colour, on Chrome's pixel row, instead of egui's plain body text.
- Shadows now match the browser: popovers, select lists, dialogs, cards, tabs and controls draw every layer of their Tailwind shadow with its negative spread and the browser's soft blur. Before, a maia popover's shadow stuck out 12 px too far on each side, and the other styles' overlay shadows had a hard, short edge and lacked their second layer.
- The shadow and focus ring of a popover, select list or dialog now lie under the panel and its content, as in the browser, instead of over the panel's outer edge and over content that runs past it.
- A tooltip now shows the border, ring, shadow and per-corner radii that a style or an app sets on `theme.style.tooltip_content`, and a slider range and the tabs line indicator follow their own recipe's radius and border; a slider range's end is square under the thumb, as in the browser, instead of taking the track's rounding.
- A shadow no longer shows through a transparent or translucent box: the light vega input, select trigger and checkbox no longer look grey inside, and dark-mode switches, inputs and outline buttons no longer darken their translucent fill.
- Plain egui widgets in a disabled area now fade by the style's disabled opacity (`disabled:opacity-50`) instead of egui's own value, and a style's `disabled:opacity-*` on a part shows at exactly that value inside a disabled area.
- A focus ring shown after keyboard focus no longer disappears when the mouse drags a scroll bar or presses the focused widget again, and a key pressed on a widget focused by a click now shows its ring, while a Ctrl, Alt or Command shortcut does not, as in the browser.
- Rounded corners now paint the focus ring under the box's edge, as the browser does, so the ring's corner pixels match shadcn's.
- A checked checkbox with keyboard focus now shows the focus border instead of the checked border, as shadcn does; so does a checked switch in the styles that give it a checked border.
- The lines of a `layout::wrap_row` are now exactly the gap apart; before, a line of badges or other items lower than a button took a button's height, so its lines stood further apart than the gap.
- In the settings example, rows of buttons and badges wrap onto further lines in a narrow window instead of running past the card.
- A `layout::vstack` inside a `layout::wrap_row` now moves to the next line as a whole when it does not fit, as a flex item does, instead of running past the right edge.
- In the luma and rhea styles, an invalid switch that is checked now shows the red invalid border instead of the checked border, as shadcn does.
- A Button or Badge with an icon and an empty text is no longer one gap wider than the icon and its padding, as in the browser, where empty text takes no room.
- A `layout::vstack` inside `layout::row`, `wrap_row`, `space_between` or a card footer now grows to its widest child, as a flex column does: an input or select wider than the room left pushes the row's next children aside instead of being covered by them.
- At phone width in the luma, vega and maia styles, a tab list wider than the page column no longer widens the card under it; the list runs past the column's edge as in the browser, and the card keeps the column's width.
- An input's placeholder now has the size of the typed text: 16 px below 768 px, and 12 px in the lyra and mira styles, instead of always 14 px.
- `Input::max_width(f32::NAN)` now gives the narrowest field instead of the default width.
- An input in a column narrower than its smallest width now runs past the column's edge instead of widening the column and the widgets in it.
- A select list whose options are wider than the window now stays inside the window and wraps the long option texts, instead of running past the window's edge; its minimum width is now the trigger's width, not 2 px more.
- A select trigger whose longest option, or whose `.width(..)`, is wider than the room left in its column now stays inside the column and cuts its text off, instead of running past the window's edge; in a column narrower than its padding and chevron it runs past the column's edge without widening the column.
- In a `layout::row`, `layout::wrap_row`, `layout::space_between` or a card footer, an input, select, slider or tab list wider than the room left no longer covers the widgets after it: they now follow it, as in the browser. Wide content in these rows, such as a long label, no longer widens the column and the widgets in it; it runs past the column's edge.
- A `layout::vstack` with wrapping text inside `layout::row`, `layout::space_between` or a card footer now narrows as a flex column does: its text wraps in the room the row's other widgets leave, and several stacks share that room as in Chrome. Before, the text wrapped at the whole row's width, and the widgets after it ran past the row or lay over the text. The split follows when a stack's text, an image in it, its font or the display scale changes.
- A `layout::vstack` in a `StripBuilder` or table cell now fills the cell and wraps its text at the cell's width, as a grid item does; before, it took only its content's width. A widget wider than its strip cell inside a `layout::row` now runs past the cell instead of pushing the row's next widgets aside.
- Plain egui widgets under the theme, such as egui's own buttons, text fields and windows, draw their 1 px outlines on whole device pixels at every display scale, as crate widgets do; at 1.5x the outline was one and a half device pixels wide and blurred. The width also follows a later change of the display scale.
- The `build-ui` skill now says how a design's `gap-N` maps to a `StripBuilder`: its item spacing is N×4 px, and 0 without a gap. Before, strips built from it kept egui's 8 px between cells where the design had none.
- The `build-ui` and `port-component` skills said to regenerate snapshot PNGs with `UPDATE_SNAPSHOTS=1` alone, which keeps the old pixels of 1 px borders and rounded corners while the test passes. They now say to delete the PNGs a change touches first.

## 0.6.0 - 2026-10-17
### New
- `Tabs::width` makes a tab list wider than its labels: `.width(f32::INFINITY)` spans the available width, as `<TabsList class="w-full">` does, and a width in points sets the list to that width; the triggers share the width in equal parts. A trigger whose label needs more keeps its own width, and without the option the list stays as wide as its labels. (#18)

## 0.5.0 - 2026-10-16
### New
- The component gallery runs in the browser at
  https://oetiker.github.io/shadcn-egui/ and is updated with each release.
  A link such as `?preset=bIkeymG&mode=dark` opens it with that preset and mode.
- Every colour, size and spacing of every component can be changed through
  `theme.style`, for example a filled `primary` active tab with a solid 2 px
  focus ring. `Rule::important(When::all(State::SELECTED), value)` adds a
  rule ranked `Rule::IMPORTANT`, as a Tailwind `!` class is: it beats every
  rule of the style that is not important. A rule the style itself marks
  important still wins over it where that rule also names a state, such as
  hover. (#6)
- `Popover::side_offset` and `Select::side_offset` set the gap between the
  trigger and the popup; the default stays 4
  (`components::shared::DEFAULT_SIDE_OFFSET`). (#6)
- An `opacity` set in `theme.style` on any part now fades that part and
  everything inside it, multiplied as in the browser and also inside a
  disabled `Ui`: a badge, card, dialog
  (its backdrop too), popover, tooltip, separator, tab list, select list,
  checkbox tick, select chevron or select tick. Before, only the outer part
  of a button, input, checkbox, switch, slider, label, select trigger or tab
  took its `opacity`. A custom component fades a container part's content
  with `components::shared::faded`. (#6)
- Inside a disabled area (`ui.add_enabled(false, ..)`, `Ui::disable`), a
  part whose style sets a `disabled:` opacity is drawn at the disabled fade
  alone, as in the browser, even where `theme.style` also gives it a base
  opacity. A part without a `disabled:` opacity keeps its own opacity on top
  of the disabled fade. (#6)

### Changed
- The Tab key now stops once per tab list, on the selected tab, and the next
  Tab leaves the list; before, it stopped on every tab. The arrow keys, Home
  and End still move between the tabs.
- In a line tab list (`TabsVariant::Line`), a focused tab that is not active
  shows its label in the foreground colour and a 2 px underline in the ring
  colour, in every style, and no longer draws a rounded ring around itself.
  The active tab looks the same focused or not. (#6)
- In the luma and rhea styles, the dialog's close button sits on a round
  disc in the secondary colour, as on the shadcn site. (#6)
- The separator's thickness comes from the style's `separator` recipe. An app
  that changed `metrics.border` to make separators thicker now changes
  `separator.height` (horizontal) or `separator.width` (vertical) instead. (#6)
- Replacing a whole recipe on a `Style` now needs a `*`:
  `*style.button = Recipe { .. }`. A `let` array of recipes for `pick`
  needs it too: `[&*style.button, &*style.button_size_sm]`. Setting one
  value, as in `style.button.height.base = Some(36.0)`, is unchanged.
- `Recipe` has the new fields `height_inset`, `outline_width` and
  `outline_color`, and `Style` the new recipes `separator` and
  `tabs_trigger_after` (the line tab underline). Code that builds either
  with a struct literal needs `..Default::default()`. (#6)
- `components::shared::disabled_painter` is gone. A custom widget fades a
  disabled part with `style_painter(ui, &[stack], state)`, which takes the
  fade from the part's recipe (`disabled:opacity-50`). (#6)
- `Input::id_salt(..)` gives a text field a fixed id from any salt, such as
  `format!("row{i}")` for a field per row. `Input::id_source(..)`, which
  took only a string literal, still works but warns that it is deprecated.
- `Badge::icon` and `Badge::icon_end` take any icon reference, such as
  `&Theme::current(ctx).icons.check`, not only an icon that lives for the
  whole program.
- In `shadcn-egui-core` (re-exported by `shadcn-egui`), `Icon::segs()` and
  `Icon::fill_mesh()` are replaced by `Icon::layers()`, an `IconLayers`
  iterator that yields each layer of an icon (`IconLayer`) with its strokes,
  its fill and its `alpha`. Code that draws icons itself paints each layer
  at its `alpha`; `paint_icon` and `IconView` already do.
- A rule pushed into `theme.style` with a rank made of variant bits only now
  loses to the style's own rule for the same state. Ranks now also count
  selector specificity, as the browser does, so a `data-[size=sm]:` rule
  beats an `md:` rule and a `placeholder:` text colour beats a `dark:` one;
  the `Rule` documentation lists what to add, or use `Rule::important`.
- Setting `icon_size.base` in `theme.style` no longer changes the icon size
  of buttons, select triggers, select items, select scroll buttons and tab
  triggers. Assign `icon_size = Prop::fixed(..)` instead.
- The installed `shadcn-egui` Claude Code plugin now takes about 100 KB on
  disk instead of a full copy of the repository with its font and icon
  data, about 40 MB per installed version.

### Fixed
- The Hugeicons icons `ARROW_BIG_RIGHT_DASH`, `HAMBURGER_01` and
  `RIGHT_TO_LEFT_LIST_BULLET`, and the Tabler icon `BRAND_PARSINTA`, drew
  their faded parts (the arrow's dash, the burger's bun top and seeds, the
  list's middle line and bullets, a quarter of the Tabler ring) at full
  strength. They now draw them faded, as on the web.
- Applying a theme over an app's own fonts (`apply_with_fonts`, or `apply`
  after the app's own `ctx.set_fonts`) removed `shadcn-body-400` and the
  other theme faces from the app's own font families, and dropped every app
  font whose name starts with `shadcn-`. Both now stay.
- With headings of different sizes in dark and light mode, replacing the
  fonts with `ctx.set_fonts` left one mode's headings in the body font even
  after the theme's fonts were back, and gave a dark-mode screen the light
  heading size for a frame. Each mode's headings now return to the heading
  font at their own size.
- A `FontSource::Variable` whose `wght` range runs backwards or has a NaN
  bound crashed the app with `min > max, or either was NaN` when the theme
  was applied. A backwards range now gives every weight its end, and a NaN
  bound is ignored.
- The crates.io pages of the font, icon and core crates now show a README,
  keywords, a category and a documentation link; before they were empty.
  Each README names the `shadcn-egui` feature that enables the crate and
  the licence of its font or icon set.
- A popover whose style recipe an app replaced whole
  (`*style.popover_content = Recipe { .. }`) showed each word on its own
  line; it is now as wide as its content. Buttons, badges, inputs, select
  triggers, tab lists, checkboxes and switches in the same case are no longer
  0 px high, and checkbox ticks, select chevrons and the check mark of the
  selected option no longer shrink to a dot. (#6)
- In the mira style, the open select list showed a tiny dot next to the
  selected option; it now shows a check mark. (#6)
- An app that held or copied several styles in one function could crash
  with `thread '<unknown>' has overflowed its stack`, mostly on worker
  threads and in WebAssembly, where the stack is 1 to 2 MiB. Each `Style`
  now takes a few hundred bytes of stack instead of about 110 KB.
- Building an app with Rust 1.99 no longer prints
  `warning: the following packages contain code that will be rejected by a future version of Rust: shadcn-egui v0.4.0`.

## 0.4.0 - 2026-10-13
### New
- A gallery example shows every component in each variant and size, under
  a preset you paste as a code or pick part by part (style, colours, fonts,
  icon library, radius, menu), in light or dark mode. Run it with
  `cargo run --example gallery --features fonts-all,icons-all`; an app shows
  the same screen with `reference::gallery_ui`.
- `Preset::from_code("b…")` reads a preset code copied from ui.shadcn.com,
  and `preset.apply(ctx, Mode::Dark)` sets up the app with its style, base
  colour, theme, chart colour, fonts, icon library, radius and menu colours.
  `Preset::default()` is the site's default (Nova, neutral, Inter, Lucide);
  `to_code()` gives the code back. Codes from shadcn-svelte.com are not
  supported: that site numbers colours and icon libraries differently.
- When a preset asks for something this build cannot show, the app uses a
  stand-in and logs a warning such as
  `shadcn-egui: font "geist" needs the "font-geist" feature; using "inter"`.
  The Sera style becomes Nova, the gray base colour, theme or chart colour
  (ui.shadcn.com has no colours for gray) becomes neutral or the base colour,
  and a font or icon library whose Cargo feature is off becomes Inter or
  Lucide. The translucent menu colours draw as the default or inverted menu
  colour, because egui cannot blur what lies behind a menu.
- With the Lyra or Sera style every corner is square, whatever radius the
  preset names, as on the shadcn site.
- A translucent menu colour always highlights menu items with the subtle
  accent, also when the preset asks for the bold one, as on ui.shadcn.com.
- An open Select list draws in the preset's menu colours: with the
  `inverted` menu colour it is dark also on a light page.
- Every font of the shadcn create page is available, each behind its own
  feature named after the font (`font-geist`, `font-jetbrains-mono`,
  `font-playfair-display`, …), and a preset that names the font uses it.
  The `fonts-all` feature turns them all on. A preset that names a font
  whose feature is off renders in Inter and logs a warning.
- The font crates carry the font's own licence next to MIT: `MIT AND OFL-1.1`
  for most fonts, `MIT AND Apache-2.0` for the Apache-licensed ones.
- The checkbox tick, select arrow and dialog close button follow the preset's
  icon library: Lucide by default, and Tabler, Phosphor, Remix Icon and
  Hugeicons with the features `icons-tabler`, `icons-phosphor`,
  `icons-remixicon` and `icons-hugeicons` (`icons-all` for all). Every icon of
  each set is available as a constant, for example
  `shadcn_egui::icons::tabler::HOME`.
- Filled icons (Phosphor, Remix Icon) are drawn with smooth edges, also when
  faded for a disabled widget or a translucent menu.
- `Palette::from_upstream(base, theme, chart, dark)` gives the palette the
  shadcn site builds for a base colour, a colour theme and a chart colour,
  for example `Palette::from_upstream("stone", "blue", "rose", true)`.
- `Palette` has the five chart colours (`chart-1` … `chart-5`) as `chart`.
- The `settings` example has a toolbar with a style select (vega, nova, maia,
  lyra, mira, luma, rhea), a Dark/Light tab pair and a radius slider from 0 to
  20 px, so the look can be changed while the window is open.
- `Slider` with one thumb, horizontal or vertical. A click or drag sets the
  value, the arrow, Page Up/Down, Home and End keys step it, and
  `.enabled(false)` draws it at half opacity and ignores input.
- Button sizes `Xs`, `IconXs`, `IconSm` and `IconLg`, and leading or trailing
  button icons. Buttons also show the expanded, invalid and has-popup states.
- Badge variants `Ghost` and `Link`, and leading or trailing badge icons.
- `SwitchSize::Sm` (24 x 14 px) and `Tabs::new(..).variant(TabsVariant::Line)`,
  which draws an underline under the active tab.
- `Input::invalid`, `Checkbox::invalid`, `Switch::invalid` and
  `Select::new(..)` options `.width(..)`, `.size(SelectSize::Sm)`,
  `.placeholder(..)`, `.invalid(..)` and `.enabled(..)`; `Dialog` gets
  `show_close_button(false)`.
- `Card::new().size(CardSize::Sm).footer(..).show(ui, ..)` with
  `card_header`, `card_header_with_action` and `card_content`, and
  `popover_header(ui, title, description)`; the card footer is a tinted strip
  reaching the card edges in nova.
- `separator_vertical` draws a vertical line.
- `Palette::with_bold_menu_accent()` for shadcn's `menuAccent: "bold"`.
- `Metrics` has new fields `line_xs`, `line_sm`, `line_base` and `line_lg`
  (Tailwind's line heights for the text sizes), and `Theme::line_px(TextSize)`
  returns the line height for a text size.
- `Theme::vega_dark()` and `Theme::vega_light()` for the default vega style.

### Changed
- The Claude Code plugin is now called `shadcn-egui` and builds on the
  crate instead of copying code into the project; its skill `build-ui`
  builds a UI and `port-component` ports a missing component. It installs
  from the release tag, so it updates only with a crate release. Remove an
  `egui-shadcn` install with `/plugin uninstall egui-shadcn@egui-shadcn-dev`
  and `/plugin marketplace remove egui-shadcn-dev`, and install the new
  plugin with `/plugin marketplace add oetiker/shadcn-egui` and
  `/plugin install shadcn-egui@shadcn-egui`.
- The crate is published on crates.io as `shadcn-egui`: add it with
  `cargo add shadcn-egui` and change `egui_shadcn::` to `shadcn_egui::` in
  your imports. Copying the component module into a project is no longer
  supported.
- The default font is Inter, as on the shadcn site, so all text looks
  different from 0.3. Oxanium remains available through the `font-oxanium`
  feature.
- Medium and semibold text (button labels, badges, tabs, card and dialog
  titles) now renders at its real weight; before, every weight looked like
  regular text.
- Text in Inter and Oxanium sits up to one pixel higher in some sizes, for
  example in badges and tooltips.
- Card and dialog titles use the theme's heading font, which is the body
  font unless `Theme::fonts(body, Some(heading))` sets another one.
- An app that installs fonts of its own calls
  `Theme::apply_with_fonts(ctx, its_font_definitions)` instead of
  `Theme::apply`; this keeps the app's fonts next to the theme's.
- `Theme::apply` sets egui's light and its dark style alike: text sizes,
  spacing and colours that an app set with `ctx.set_style_of` or
  `ctx.style_mut_of` before the call are overwritten in both modes.
- `Theme` has the new public fields `style`, `fonts`, `icons` and `menu`, so
  a `Theme { .. }` struct literal written for 0.3 no longer compiles; build
  themes with `Theme::new(..)` and set the rest with `Theme::fonts(..)`,
  `Theme::icons(..)` and `Theme::menu(..)`. Checkbox, Select and Dialog draw
  the icons of the theme, so a preset's icon library changes them.
- A `Palette { .. }` struct literal written for 0.3 no longer compiles,
  because `Palette` has the new field `chart`; start from `Palette::light()`,
  `Palette::dark()` or `Palette::from_upstream(..)` and change single fields.
- `Button::icon` and `Button::icon_end` accept any icon reference, not only
  constants.
- Icon data you generated with 0.3 no longer compiles with 0.4. Use the
  icons in `shadcn_egui::icons` (Lucide, and the other sets behind their
  features) or regenerate your own.
- Remix Icon stays at version 4.8.0, its last release under the Apache-2.0
  licence; later releases forbid redistribution as an icon pack.
- Components now take their sizes, colours, radii and shadows from a shadcn
  style. `Theme::new(Style::vega(), Palette::dark(), 10.0)` gives the classic
  look and `Style::nova()` the compact one, matching a web client set up with
  the same `style` in `components.json`; `Style::maia()`, `lyra()`, `mira()`,
  `luma()` and `rhea()` are available too, but are not covered by the snapshot
  tests.
- Controls look slightly different from 0.3 even with vega: destructive
  buttons and badges are light red tints instead of solid red, hovered Ghost
  and Outline buttons use the muted fill, and cards and popovers draw a thin
  outline ring instead of a border. Controls are 36 px high in vega and 32 px
  in nova, with 10 px side padding.
- `Theme` is `Clone` and no longer `Copy`, so code that reuses a theme value
  must call `.clone()`.
- `card_title` and `card_description` return the label's `Response`. Where
  such a call ends a block that must return nothing, add a `;`.
- The new `ButtonSize` and `BadgeVariant` variants (see New) break a
  `match` that lists every variant of these enums; add the new arms or a `_`
  arm.
- The `sm` to `4xl` corner radii scale with `radius` by a factor
  (`rounded-xl` is 1.4 x radius), as in current shadcn. At the default radius
  of 10 px, small, medium, large and extra-large corners look as before;
  larger radii now round more.
- `Popover::width(..)` and `Dialog::width(..)` set the width of the whole box
  including its padding, so existing callers get narrower content.
- Native egui widgets such as `ui.button(..)` take their height, padding and
  spacing from the style's default button and tabs gap: a native button is
  36 px high in vega and 32 px in nova, like the crate's own buttons.
  Applying a style that sets no button height brings back egui's default
  height instead of keeping the previous theme's.
- Disabled controls are drawn at half opacity both with `.enabled(false)` and
  inside a disabled area (`ui.add_enabled(false, ..)`, `Ui::disable`). A
  disabled slider or switch shows its track through the thumb.
- Buttons: a pressed button moves down by 1 px, a hovered Link button is
  underlined, and a disabled button is drawn at half opacity instead of being
  blended toward the page colour. Keyboard focus draws a 3 px ring at the
  button's edge and turns its border to the ring colour.
- Badges are 20 px high with fully rounded ends.
- Text inputs have a 3 px focus ring, and the border is the `input` colour at
  rest and the ring colour on keyboard focus. `.invalid(true)` draws a red
  border and ring, and `.enabled(false)` draws the field at half opacity and
  ignores typing.
- Checkboxes are a 16 px box with a 1 px border, a checkmark icon, a soft
  shadow and a 3 px focus ring, and the click target reaches 12 px to each
  side and 8 px above and below. `.invalid(true)` and `.enabled(false)` work as
  for inputs.
- Switches have a 32 x 18.4 px track, a round thumb inside the 1 px border
  and a 3 px focus ring, with the same enlarged click target as checkboxes.
  `.invalid(true)` and `.enabled(false)` work as for inputs, and the track or
  thumb casts the shadow the style sets (the track in vega).
- Labels are 14 px text with no extra line height, and a label shown with
  `Label::new(..).enabled(false)` is drawn at half opacity. Separators are
  exactly 1 px thick and add no spacing of their own.
- Helper text from `label::description`, popover headers and tooltips use the
  line height of the web version (20 px for 14 px text, 16 px for 12 px), so
  they take a little more room. Popover titles and descriptions take their
  own size and line height from the style, which makes the title larger in
  maia, lyra, mira, luma and rhea.
- Tabs: the tab list is as wide as its labels instead of filling the row, and
  the active tab shows a raised pill with a soft shadow. Left, Right, Home and
  End move the selection, and `.enabled(false)` draws the tabs at half opacity.
  A mouse click selects a tab without drawing the keyboard focus ring.
- Tooltips are inverted as in current shadcn: a foreground-coloured bubble
  with background-coloured text instead of the primary colour, with a small
  rounded arrow pointing at the trigger. Text wraps at 320 px.
- Popovers draw a thin outline ring instead of a border and a softer shadow,
  with padding and corners from the style. The default width is 288 px.
- Selects: the trigger has a chevron and is as wide as its widest option
  instead of the full field width, and the list has a thin outline ring and a
  check mark on the chosen row. The list takes Up, Down, Home, End, Enter, Esc
  and type-ahead keys until keyboard focus moves to another control.
- Screen readers get no name for a switch, checkbox or select on its own:
  name each one after its label with `.labelled_by(label.id)`, as the
  `settings` example does. A select reports its shown text as its value.
- Cards fill the available width, with padding, gaps and corners from the
  style.
- Dialogs have a thin outline ring and no shadow, a 384 px (nova) or 448 px
  (vega) wide panel, a close button that looks like a ghost icon button, and a
  lighter backdrop without blur. In nova the footer is a tinted strip reaching
  the panel edges.
- The `settings` example takes the style and palette on the command line, for
  example `cargo run --example settings -- nova light` (default `vega dark`).
  `SettingsState` has new `slider` and `region` fields.
- A `match` on a preset enum (`StyleName`, `BaseColor`, `ThemeColor`,
  `FontName`, `IconLibrary`, `MenuColor`, `MenuAccent`, `RadiusChoice`) or
  on `Role`, `TabsOrientation` or `PresetCodeError` needs a `_ =>` arm, and
  each preset enum's `ALL` is a slice (`&'static [Self]`), not an array. New
  shadcn styles, colours, fonts and icon sets can then arrive in a minor
  release.

### Removed
- `Theme::dark()` and `Theme::light()`: use `Theme::vega_dark()` /
  `Theme::vega_light()` or `Theme::new(..)`. `Theme::shadow_md()` and
  `shadow_lg()` are gone too; use `Theme::shadow(ShadowLevel)`.
- `Metrics` fields that a style now sets (control heights, paddings, gap, card
  padding, dialog and popover width, ring width), and `Palette::shadow` and
  `Palette::overlay`. Set such values on the `Style` in your theme setup
  instead.
- The new-york look; there is no compatibility style.
- `components::shared::mix_toward`, `hover_fill` and `focus_ring`: the
  components take hover and focus colours from the style.
- `components::icon::{CHECK, CHEVRON_DOWN, CLOSE, LUCIDE_SLOTS}`: use
  `icons::lucide::{CHECK, CHEVRON_DOWN, X, SLOTS}`, which also gives you every
  other Lucide icon (2141 in all).

### Fixed
- Every `Theme::apply_with_fonts` that passed the context's current fonts
  added the body font to the font list once more, so the list grew with each
  call. After such a call switched from a monospace body font to a sans one,
  code and other monospace text still showed in the old body font. Both
  are fixed.
- On a desktop with a light system theme, an app that called `Theme::apply`
  showed egui's own light look instead of the shadcn theme, and the theme
  also vanished when the desktop switched between light and dark. The theme
  now holds in both cases.
- Faded and disabled icons are evenly transparent: line ends, crossings and
  filled shapes no longer show darker spots.
- An app that called `Theme::apply` while drawing a frame crashed with
  "FontFamily::Name(\"shadcn-heading-semibold\") is not bound to any fonts"
  as soon as a window or popup showed a heading in that frame. Such a heading
  now shows in the body font for that one frame.
- An app that replaced the fonts with `ctx.set_fonts` after `Theme::apply`
  crashed with "FontFamily::Name(\"shadcn-heading-semibold\") is not bound to
  any fonts" on the next heading or the next `Theme::apply`. Headings now fall
  back to the regular text font with a log warning, and a later
  `Theme::apply` adds the theme's fonts back and keeps the app's own.
- Screen readers announced a standalone icon by its internal name ("mic",
  "power") and ignored a label attached with `labelled_by`. An `IconView` now
  has no name of its own; name it with `.labelled_by(label.id)`.
- In a `layout::row`, items placed before a taller item, such as a label
  before a large button, sat about 2 px above the row's centre. They are now
  centred. A row shrinks back when its tallest item goes, except a row that
  holds a vertical separator or a scroll area, which keeps its tallest height.

## 0.3.2 - 2026-10-09

### Changed
- The skill, the gotchas and the registry README told you to call
  `Theme::apply` every frame. They now say to call it once, right after the
  egui context is created (for eframe, in the `run_native` app creator), and
  again only when the theme changes. Re-applying every frame silently undid
  any other code's style changes, so such bugs never showed.
- The settings example applies the theme in the eframe app creator instead
  of on every frame.

## 0.3.1 - 2026-10-09

### Fixed
- Ghost and Outline buttons kept their normal text colour on hover while the
  background turned to `accent`. With a palette whose `accent` is bright,
  the label became unreadable. Hovered, they now use `accent_foreground`
  for text and icon, as shadcn does.
- `Stroke::new(2.0, …)` in the checkbox drew a `float_literal_f32_fallback`
  warning from newer Rust compilers, which will become an error.

## 0.3.0 - 2026-10-09

### Added
- Dialog, Popover and Tooltip components in the shadcn style. A dialog closes
  on Esc, a click on the dimmed backdrop, or its × button. A popover opens and
  closes when its trigger is clicked and closes when you click elsewhere.
  Their colors, shadows and widths are theme settings (`Palette.overlay`,
  `Metrics.dialog_width`, `Metrics.popover_width`).
- Icons without an image loader. The new `icongen` tool (in `tools/icongen/`)
  turns Lucide or other outline SVG icons into Rust constants, and
  `IconView` / `Button::icon` draw them in the theme's colors and sizes. A
  project includes only the icons it lists and gains no runtime dependency.
  Dots drawn as tiny lines, as in Lucide's `circle-alert`, stay visible.
- `Button::enabled(false)` greys a button out halfway toward the page
  background and stops it from reacting to clicks or taking focus, like
  shadcn's `disabled` buttons. Screen readers announce it as disabled.
- `Input::id_source("…")` gives a text field a fixed id, so code elsewhere can
  focus it or a test can find it, however many widgets come before it.
- Sizes are now part of the theme: `Theme.metrics` holds text sizes, control
  heights, paddings, the default gap, card padding, field width, icon size and
  border/ring widths. Change one value and every component follows.
- `Palette.shadow` sets the card shadow color.
- Badges are announced to screen readers.
- The skill now requires colors and text sizes to come from the theme, and
  gives a check to run over a project's own UI files before calling a screen
  done. New components must also report themselves to screen readers.
- `layout::wrap_row` lays items out in a row that wraps onto the next line when
  it runs out of width, with the same gap between items and between lines.

### Changed
- **Breaking:** code that builds a `Theme` or `Palette` field by field must add
  `metrics` (use `Metrics::default()`) and `shadow`. Projects using
  `Theme::light()`/`dark()` are unaffected.
- The destructive badge's text uses the `destructive_foreground` color instead
  of pure white.
- The skill's advice on animations no longer contradicts itself: a fade timed
  by hand vanishes instead of fading unless it repaints while it runs.
- A focused text field's border now turns the `ring` colour, like shadcn's
  inputs, instead of staying the plain `border` colour under the focus ring.

## 0.2.1 - 2026-10-01

### Added
- **New `references/cpu-rendering.md`** for apps that drive egui themselves and
  rasterize without a GPU (softbuffer): what to change when one keystroke pins a
  CPU core, a new screen shows up only after the mouse moves, or toggles animate
  in visible steps. It also covers `is_pointer_over_egui()` reporting `true`
  everywhere under a `CentralPanel`, and the Wayland connect segfault from
  `egui-winit`'s default clipboard feature. The skill now triggers on these
  symptoms too.

## 0.2.0 - 2026-06-14

### Changed
- **Slimmed the runtime dependency surface to just `egui` + `egui_extras`.**
  `egui_extras` now uses `default-features = false` (it only needs
  `StripBuilder`/`Size`); the previous `all_loaders` feature pulled `image`,
  `resvg`/`usvg`, and `ehttp` into every consumer for no reason. `eframe`,
  `egui_kittest`, and `wgpu` are dev-only — they never reach a shipped binary.
- **Migrated to the non-deprecated egui/eframe 0.34 `&mut Ui` model.**
  `reference::settings_ui` now takes `&mut egui::Ui` (uses
  `CentralPanel::show_inside`); the example uses `eframe::run_ui_native`; the
  snapshot harness uses `Harness::builder().build_ui`. All `#[allow(deprecated)]`
  removed. **Breaking:** `settings_ui(ctx, …)` → `settings_ui(ui, …)`.
- **Reworked `SKILL.md`** into two clearly separated parts — *Using the
  components* vs *Porting a new component* — and added a **dependency-discipline
  standard** (default: add no runtime dep; heavy crates like eframe/image/wgpu
  stay dev-only; ask before adding one). README and plugin manifests reframed as
  backend-agnostic (plain egui widgets; no eframe/wgpu coupling).

### Fixed
- **Vertical text centering** in buttons, inputs, badges, and tabs. Oxanium's
  ascent reserves more empty space above the cap than below the baseline, so
  egui's row-box centering left single-line text ~2.5 px too high. Corrected
  globally with `FontTweak { y_offset_factor: 0.18 }` on all three font weights.

### Added
- **Component gallery.** `reference::settings_ui` is now a three-tab screen
  (Account / Notifications / Components) that exercises every ported component:
  Button (all variants + sizes), Input (incl. password), Select, Label, Switch,
  Checkbox, Badge (all variants), Card, Tabs, Separator.
- Snapshot coverage for all three tabs; refreshed README screenshots and added a
  component-gallery shot.

## 0.1.0 - 2026-06-13

### Added
- Initial release: shadcn-v4 (new-york / OKLCH) design tokens and theme for egui,
  a flexbox-substitute layout layer (`StripBuilder` helpers), and themed
  components (Button, Input, Label, Card, Tabs, Switch, Checkbox, Select,
  Separator, Badge).
- Layout-first `SKILL.md` workflow with reference tables (token / layout /
  component maps) and a headless `egui_kittest` render-verification loop.
- Reference settings-form-with-tabs screen + snapshot eval; Claude Code plugin
  manifest and marketplace listing.
