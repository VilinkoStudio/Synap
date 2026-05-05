mod app;
mod core;
mod domain;
mod ui;
mod usecase;

use std::rc::Rc;

use crate::core::{DesktopCore, SynapCoreAdapter};

const APP_ID: &str = "io.synap.desktop_linux";

fn main() {
    if std::env::var_os("GSK_RENDERER").is_none() {
        unsafe {
            std::env::set_var("GSK_RENDERER", "cairo");
        }
    }

    adw::init().expect("Failed to init libadwaita");

    let manager = adw::StyleManager::default();
    manager.set_color_scheme(adw::ColorScheme::Default);

    let core = match SynapCoreAdapter::new_from_env() {
        Ok(core) => Rc::new(core) as Rc<dyn DesktopCore>,
        Err(error) => {
            eprintln!("failed to initialize desktop core: {error}");
            std::process::exit(1);
        }
    };

    let app = relm4::RelmApp::new(APP_ID);
    app.run::<app::App>(core);
}
