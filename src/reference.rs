//! The reference screen: a settings form with tabs that doubles as a gallery of
//! every ported component. Built entirely from the egui_shadcn theme + layout
//! helpers + components. Reused by the snapshot eval and the `settings` example.

use crate::components::badge::{badge, BadgeVariant};
use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::components::card::{card_description, card_title};
use crate::components::checkbox::checkbox;
use crate::components::input::Input;
use crate::components::label::label;
use crate::components::select::Select;
use crate::components::separator::separator;
use crate::components::slider::Slider;
use crate::components::switch::toggle;
use crate::components::tabs::Tabs;
use crate::{layout, Theme};

pub struct SettingsState {
    pub tab: usize,
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: usize,
    pub marketing_emails: bool,
    pub security_emails: bool,
    pub weekly_digest: bool,
    /// Volume slider on the components tab, 0..=100.
    pub slider: f32,
    /// Region select on the components tab; an index past the options shows its placeholder.
    pub region: usize,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            tab: 0,
            name: String::new(),
            email: String::new(),
            password: String::new(),
            role: 0,
            marketing_emails: false,
            security_emails: false,
            weekly_digest: false,
            slider: 50.0,
            region: usize::MAX,
        }
    }
}

/// Render the settings screen into the given root `Ui`. `Theme::apply` must
/// have been called on the context (once, when it was created). The caller
/// owns the root `Ui` (e.g. from eframe's `App::ui`); this fills it with a
/// `CentralPanel`.
pub fn settings_ui(ui: &mut egui::Ui, state: &mut SettingsState) {
    let t = Theme::current(ui.ctx());
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(t.palette.background)
                .inner_margin(egui::Margin::same(32)),
        )
        .show_inside(ui, |ui| {
            layout::vstack(ui, 24.0, |ui| {
                card_title(ui, "Settings");
                card_description(ui, "Manage your account settings and preferences.");
                // The gap between the tab list and its content comes from the style.
                layout::vstack(ui, Tabs::content_gap(ui), |ui| {
                    ui.add(Tabs::new(
                        &mut state.tab,
                        &["Account", "Notifications", "Components"],
                    ));
                    match state.tab {
                        0 => account_tab(ui, state),
                        1 => notifications_tab(ui, state),
                        _ => components_tab(ui, state),
                    }
                });
            });
        });
}

/// A labelled control: a 6px-gap stack of `label` over the control. The
/// label names the control for screen readers (`labelled_by`).
fn field(ui: &mut egui::Ui, name: &str, control: impl FnOnce(&mut egui::Ui) -> egui::Response) {
    layout::vstack(ui, 6.0, |ui| {
        let name = label(ui, name);
        let _ = control(ui).labelled_by(name.id);
    });
}

/// A label on the left, its control on the right, the label naming the
/// control for screen readers (`labelled_by`).
fn setting_row(
    ui: &mut egui::Ui,
    name: &str,
    control: impl FnOnce(&mut egui::Ui) -> egui::Response,
) {
    let label_id = std::cell::Cell::new(egui::Id::NULL);
    layout::space_between(
        ui,
        |ui| label_id.set(label(ui, name).id),
        |ui| {
            let _ = control(ui).labelled_by(label_id.get());
        },
    );
}

fn account_tab(ui: &mut egui::Ui, state: &mut SettingsState) {
    layout::card(ui, |ui| {
        layout::vstack(ui, 16.0, |ui| {
            card_title(ui, "Account");
            card_description(ui, "Update your account details.");
            separator(ui);
            field(ui, "Name", |ui| {
                ui.add(
                    Input::new(&mut state.name)
                        .hint("Your name")
                        .max_width(f32::INFINITY),
                )
            });
            field(ui, "Email", |ui| {
                ui.add(
                    Input::new(&mut state.email)
                        .hint("you@example.com")
                        .max_width(f32::INFINITY),
                )
            });
            field(ui, "Password", |ui| {
                ui.add(
                    Input::new(&mut state.password)
                        .hint("••••••••")
                        .password(true)
                        .max_width(f32::INFINITY),
                )
            });
            field(ui, "Role", |ui| {
                ui.add(Select::new(
                    "role",
                    &mut state.role,
                    &["Member", "Admin", "Owner"],
                ))
            });
            separator(ui);
            layout::space_between(
                ui,
                |_ui| {},
                // right_to_left: the first widget added lands rightmost, so add the
                // primary action (Save) first to keep it on the right of Cancel.
                |ui| {
                    let _ = ui.add(Button::new("Save").variant(ButtonVariant::Default));
                    let _ = ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
                },
            );
        });
    });
}

fn notifications_tab(ui: &mut egui::Ui, state: &mut SettingsState) {
    layout::card(ui, |ui| {
        layout::vstack(ui, 16.0, |ui| {
            card_title(ui, "Notifications");
            card_description(ui, "Choose what you want to be notified about.");
            separator(ui);
            setting_row(ui, "Marketing emails", |ui| {
                toggle(ui, &mut state.marketing_emails)
            });
            setting_row(ui, "Security emails", |ui| {
                toggle(ui, &mut state.security_emails)
            });
            setting_row(ui, "Weekly digest", |ui| {
                checkbox(ui, &mut state.weekly_digest)
            });
            separator(ui);
            label(ui, "Channel status");
            layout::row(ui, 8.0, |ui| {
                badge(ui, "Active", BadgeVariant::Default);
                badge(ui, "Beta", BadgeVariant::Secondary);
                badge(ui, "Deprecated", BadgeVariant::Destructive);
                badge(ui, "Optional", BadgeVariant::Outline);
            });
        });
    });
}

fn components_tab(ui: &mut egui::Ui, state: &mut SettingsState) {
    layout::card(ui, |ui| {
        layout::vstack(ui, 16.0, |ui| {
            card_title(ui, "Components");
            card_description(ui, "Buttons, badges, a slider and a select.");
            separator(ui);
            label(ui, "Button variants");
            layout::row(ui, 8.0, |ui| {
                let _ = ui.add(Button::new("Default").variant(ButtonVariant::Default));
                let _ = ui.add(Button::new("Secondary").variant(ButtonVariant::Secondary));
                let _ = ui.add(Button::new("Destructive").variant(ButtonVariant::Destructive));
            });
            layout::row(ui, 8.0, |ui| {
                let _ = ui.add(Button::new("Outline").variant(ButtonVariant::Outline));
                let _ = ui.add(Button::new("Ghost").variant(ButtonVariant::Ghost));
                let _ = ui.add(Button::new("Link").variant(ButtonVariant::Link));
            });
            label(ui, "Button sizes");
            layout::row(ui, 8.0, |ui| {
                let _ = ui.add(
                    Button::new("Small")
                        .variant(ButtonVariant::Secondary)
                        .size(ButtonSize::Sm),
                );
                let _ = ui.add(
                    Button::new("Default")
                        .variant(ButtonVariant::Secondary)
                        .size(ButtonSize::Default),
                );
                let _ = ui.add(
                    Button::new("Large")
                        .variant(ButtonVariant::Secondary)
                        .size(ButtonSize::Lg),
                );
            });
            separator(ui);
            label(ui, "Badges");
            layout::row(ui, 8.0, |ui| {
                badge(ui, "Default", BadgeVariant::Default);
                badge(ui, "Secondary", BadgeVariant::Secondary);
                badge(ui, "Destructive", BadgeVariant::Destructive);
                badge(ui, "Outline", BadgeVariant::Outline);
            });
            separator(ui);
            field(ui, "Volume", |ui| {
                ui.add(Slider::new(&mut state.slider, 0.0..=100.0).label("Volume"))
            });
            field(ui, "Region", |ui| {
                ui.add(
                    Select::new("region", &mut state.region, &["Europe", "Americas", "Asia"])
                        .placeholder("Select a region"),
                )
            });
        });
    });
}
