use leptos::prelude::*;

use crate::app::ui::{Button, ButtonVariant};

#[component]
pub fn ConfirmDialog(
    #[prop(into)] title: String,
    #[prop(into)] description: String,
    #[prop(into)] confirm_label: String,
    #[prop(into)] cancel_label: String,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
    #[prop(optional)] danger: bool,
) -> impl IntoView {
    let confirm_variant = if danger {
        ButtonVariant::Danger
    } else {
        ButtonVariant::Primary
    };

    view! {
        <div class="confirm-overlay" role="presentation">
            <div class="confirm-dialog" role="alertdialog" aria-modal="true">
                <div class="confirm-dialog-copy">
                    <h3>{title}</h3>
                    <p>{description}</p>
                </div>

                <div class="confirm-dialog-actions">
                    <Button variant=ButtonVariant::Ghost on_click=on_cancel>
                        {cancel_label}
                    </Button>
                    <Button variant=confirm_variant on_click=on_confirm>
                        {confirm_label}
                    </Button>
                </div>
            </div>
        </div>
    }
}
