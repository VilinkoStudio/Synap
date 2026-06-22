use crate::domain::Theme;

const STYLE_CSS: &str = include_str!("style.css");
const EDITOR_CSS: &str = include_str!("editor/editor.css");

pub fn load_css() {
    let display = gtk::gdk::Display::default().expect("no display");

    let provider = gtk::CssProvider::new();
    provider.load_from_string(STYLE_CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let editor_provider = gtk::CssProvider::new();
    editor_provider.load_from_string(EDITOR_CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &editor_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn apply_theme(theme: Theme) {
    let manager = adw::StyleManager::default();
    match theme {
        Theme::Auto => manager.set_color_scheme(adw::ColorScheme::Default),
        Theme::Light => manager.set_color_scheme(adw::ColorScheme::ForceLight),
        Theme::Dark => manager.set_color_scheme(adw::ColorScheme::ForceDark),
    }
}
