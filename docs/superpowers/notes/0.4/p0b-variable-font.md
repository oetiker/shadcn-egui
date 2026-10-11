# P0b: variable-font ink proof

egui / epaint version: 0.34.3. Font: `assets/Oxanium-Variable.ttf`.
Test: `tests/variable_font_spike.rs` (one word, 40 px, black on white, rendered with egui_kittest + wgpu).

Printed line:

    ink 400=693135 500=815128 700=1072043 ratio700=1.547 ratio500=1.176

Conclusion: `FontTweak::coords` with `VariationCoords::new([(b"wght", w)])` sets the
wght axis per registered font. Ink grows 17.6 % at 500 and 54.7 % at 700 against 400.
D12 (no static instancing) holds.
