use shadcn_egui::components::label::label;
use shadcn_egui::components::select::Select;
use shadcn_egui::components::slider::Slider;
use shadcn_egui::components::tabs::Tabs;
use shadcn_egui::reference::{settings_ui, SettingsState};
use shadcn_egui::style::Style;
use shadcn_egui::{layout, Palette, Theme};

type StyleFn = fn() -> Style;

/// Every exported style, in the order the Style select lists them.
const STYLES: [(&str, StyleFn); 7] = [
    ("vega", Style::vega),
    ("nova", Style::nova),
    ("maia", Style::maia),
    ("lyra", Style::lyra),
    ("mira", Style::mira),
    ("luma", Style::luma),
    ("rhea", Style::rhea),
];
const MODES: [&str; 2] = ["Dark", "Light"];
const DEFAULT_RADIUS: f32 = 10.0;

struct SettingsApp {
    state: SettingsState,
    /// Toolbar values; `applied` is what the context currently shows.
    style: usize,
    light: usize,
    radius: f32,
    applied: (usize, usize, f32),
}

impl SettingsApp {
    fn apply(&mut self, ctx: &egui::Context) {
        Theme::new(
            STYLES[self.style].1(),
            Palette::light(),
            Palette::dark(),
            self.radius,
        )
        .apply(ctx);
        ctx.set_theme(if self.light == 1 {
            egui::Theme::Light
        } else {
            egui::Theme::Dark
        });
        self.applied = (self.style, self.light, self.radius);
    }

    /// Style, mode and radius controls, built from the crate's own components.
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let t = Theme::current(ui.ctx());
        let names = STYLES.map(|(name, _)| name);
        egui::Panel::top("theme_toolbar")
            .frame(
                egui::Frame::new()
                    .fill(t.palette().background)
                    .inner_margin(egui::Margin::symmetric(32, 12)),
            )
            .show_inside(ui, |ui| {
                layout::row(ui, 16.0, |ui| {
                    label(ui, "Style");
                    ui.add(Select::new("theme_style", &mut self.style, &names));
                    label(ui, "Mode");
                    ui.add(Tabs::new(&mut self.light, &MODES));
                    label(ui, "Radius");
                    ui.add(
                        Slider::new(&mut self.radius, 0.0..=20.0)
                            .step(1.0)
                            .label("Radius"),
                    );
                });
            });
    }
}

impl eframe::App for SettingsApp {
    // eframe hands us a root &mut Ui; settings_ui fills it via a
    // CentralPanel::show_inside, so no ctx-level panel management is needed.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Apply before anything reads the theme, and only when a toolbar
        // value changed, never every frame.
        if (self.style, self.light, self.radius) != self.applied {
            self.apply(ui.ctx());
        }
        self.toolbar(ui);
        if (self.style, self.light, self.radius) != self.applied {
            // A toolbar value changed during this pass, after the theme was
            // read: redo the pass, which applies the new theme first.
            ui.ctx().request_discard("theme changed");
        }
        settings_ui(ui, &mut self.state);
    }
}

fn main() -> eframe::Result<()> {
    // `settings [vega|nova|maia|lyra|mira|luma|rhea] [dark|light]` sets the
    // initial toolbar values.
    let args: Vec<String> = std::env::args().collect();
    let style = args
        .get(1)
        .and_then(|a| STYLES.iter().position(|(name, _)| name == a))
        .unwrap_or(0);
    let light = usize::from(args.get(2).map(String::as_str) == Some("light"));
    eframe::run_native(
        "shadcn-egui settings",
        eframe::NativeOptions::default(),
        Box::new(move |cc| {
            let mut app = SettingsApp {
                state: SettingsState::default(),
                style,
                light,
                radius: DEFAULT_RADIUS,
                applied: (style, light, DEFAULT_RADIUS),
            };
            app.apply(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}
