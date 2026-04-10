use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    Default,
    Primary,
    Danger,
    Ghost,
}

impl ButtonVariant {
    fn class_name(self) -> &'static str {
        match self {
            Self::Default => "ui-button--default",
            Self::Primary => "ui-button--primary",
            Self::Danger => "ui-button--danger",
            Self::Ghost => "ui-button--ghost",
        }
    }
}

fn join_classes(base: &str, variant: &str, extra: &str) -> String {
    let mut classes = String::from(base);
    classes.push(' ');
    classes.push_str(variant);

    let extra = extra.trim();
    if !extra.is_empty() {
        classes.push(' ');
        classes.push_str(extra);
    }

    classes
}

#[component]
pub fn Button(
    children: Children,
    on_click: Callback<()>,
    #[prop(optional)] variant: Option<ButtonVariant>,
    #[prop(optional, into)] class: String,
    #[prop(optional, into)] title: Option<String>,
    #[prop(optional, into)] disabled: MaybeProp<bool>,
    #[prop(optional)] stop_propagation: bool,
) -> impl IntoView {
    let variant = variant.unwrap_or(ButtonVariant::Default);
    let class_name = join_classes("ui-button", variant.class_name(), &class);

    view! {
        <button
            class=class_name
            type="button"
            title=title
            disabled=disabled
            on:click=move |ev| {
                if stop_propagation {
                    ev.stop_propagation();
                }
                on_click.run(());
            }
        >
            {children()}
        </button>
    }
}

#[component]
pub fn IconButton(
    children: Children,
    on_click: Callback<()>,
    #[prop(optional)] variant: Option<ButtonVariant>,
    #[prop(optional, into)] class: String,
    #[prop(optional, into)] title: Option<String>,
    #[prop(optional, into)] disabled: MaybeProp<bool>,
    #[prop(optional)] stop_propagation: bool,
) -> impl IntoView {
    let variant = variant.unwrap_or(ButtonVariant::Ghost);
    let class_name = join_classes("ui-icon-button", variant.class_name(), &class);

    view! {
        <button
            class=class_name
            type="button"
            title=title
            disabled=disabled
            on:click=move |ev| {
                if stop_propagation {
                    ev.stop_propagation();
                }
                on_click.run(());
            }
        >
            {children()}
        </button>
    }
}
