#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use std::{path::PathBuf, sync::Arc};

    use axum::{
        routing::{get, post},
        Extension, Router,
    };
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use synap_core::SynapService;
    use web::app::*;

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    let db_path = std::env::var("SYNAP_WEB_DB").unwrap_or_else(|_| "synap-web.redb".to_string());
    let service: SharedService = Arc::new(AppState {
        service: std::sync::Mutex::new(Some(
            SynapService::new(Some(db_path.clone())).expect("failed to initialize Synap service"),
        )),
        db_path: PathBuf::from(db_path),
    });
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        .route("/api/settings/export", get(export_db_handler))
        .route("/api/settings/import", post(import_db_handler))
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let service = service.clone();
                move || provide_context(service.clone())
            },
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .layer(Extension(service))
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
