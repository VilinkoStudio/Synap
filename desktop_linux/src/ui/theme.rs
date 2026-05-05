use crate::domain::Theme;

pub fn apply_theme(theme: Theme) {
    let manager = adw::StyleManager::default();
    match theme {
        Theme::Auto => manager.set_color_scheme(adw::ColorScheme::Default),
        Theme::Light => manager.set_color_scheme(adw::ColorScheme::ForceLight),
        Theme::Dark => manager.set_color_scheme(adw::ColorScheme::ForceDark),
    }
}
