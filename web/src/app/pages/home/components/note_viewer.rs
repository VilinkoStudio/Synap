use leptos::prelude::*;
use synap_core::NoteDTO;

#[component]
pub fn NoteViewer(note: Memo<Option<NoteDTO>>) -> impl IntoView {
    view! {
        <div class="note-viewer">
            {move || {
                let Some(note) = note.get() else {
                    return view! { <div class="empty-state">"选择一条笔记开始查看"</div> }.into_any();
                };

                let tags = note.tags;
                let content = note.content;
                let has_tags = !tags.is_empty();

                if has_tags {
                    view! {
                        <div class="tag-bar tag-bar-readonly">
                            {tags
                                .into_iter()
                                .map(|tag| view! { <span class="tag-pill tag-pill-readonly">{tag}</span> })
                                .collect_view()}
                        </div>

                        <article class="note-viewer-body">{content}</article>
                    }
                        .into_any()
                } else {
                    view! { <article class="note-viewer-body">{content}</article> }.into_any()
                }
            }}
        </div>
    }
}
