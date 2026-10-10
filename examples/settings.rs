use egui_shadcn::reference::{settings_ui, SettingsState};
use egui_shadcn::style::Style;
use egui_shadcn::{Palette, Theme};

struct SettingsApp {
    state: SettingsState,
}

impl eframe::App for SettingsApp {
    // eframe hands us a root &mut Ui; settings_ui fills it via a
    // CentralPanel::show_inside, so no ctx-level panel management is needed.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        settings_ui(ui, &mut self.state);
    }
}

fn main() -> eframe::Result<()> {
    // `settings [vega|nova] [dark|light]`
    let args: Vec<String> = std::env::args().collect();
    let style = match args.get(1).map(String::as_str) {
        Some("nova") => Style::nova(),
        _ => Style::vega(),
    };
    let palette = if args.get(2).map(String::as_str) == Some("light") { Palette::light() } else { Palette::dark() };
    let theme = Theme::new(style, palette, 10.0);
    eframe::run_native(
        "egui-shadcn settings",
        eframe::NativeOptions::default(),
        Box::new(move |cc| {
            // Once, when the context is created: the theme does not change
            // while the app runs.
            theme.apply(&cc.egui_ctx);
            Ok(Box::new(SettingsApp { state: SettingsState::default() }))
        }),
    )
}
