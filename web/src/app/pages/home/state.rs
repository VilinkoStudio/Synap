use synap_core::NoteDTO;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HomeMode {
    Empty,
    Settings,
    Viewing(NoteDTO),
    Editing(EditSession),
    Creating(EditorDraft),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditSession {
    pub original: NoteDTO,
    pub draft: EditorDraft,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EditorDraft {
    pub content: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingAction {
    View(NoteDTO),
    Edit(NoteDTO),
    Create,
    Settings,
}

impl EditorDraft {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_note(note: &NoteDTO) -> Self {
        Self {
            content: note.content.clone(),
            tags: note.tags.clone(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty() && self.tags.is_empty()
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.iter().any(|current| current == &tag) {
            self.tags.push(tag);
        }
    }

    pub fn remove_tag(&mut self, target: &str) {
        self.tags.retain(|tag| tag != target);
    }
}

impl EditSession {
    pub fn from_note(note: NoteDTO) -> Self {
        Self {
            draft: EditorDraft::from_note(&note),
            original: note,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.draft.content != self.original.content || self.draft.tags != self.original.tags
    }
}

impl HomeMode {
    pub fn active_note(&self) -> Option<NoteDTO> {
        match self {
            Self::Viewing(note) => Some(note.clone()),
            Self::Editing(session) => Some(session.original.clone()),
            Self::Empty | Self::Settings | Self::Creating(_) => None,
        }
    }

    pub fn draft(&self) -> Option<EditorDraft> {
        match self {
            Self::Editing(session) => Some(session.draft.clone()),
            Self::Creating(draft) => Some(draft.clone()),
            Self::Empty | Self::Settings | Self::Viewing(_) => None,
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self {
            Self::Editing(session) => session.is_dirty(),
            Self::Creating(draft) => !draft.is_empty(),
            Self::Empty | Self::Settings | Self::Viewing(_) => false,
        }
    }

    pub fn is_mutating(&self) -> bool {
        matches!(self, Self::Editing(_) | Self::Creating(_))
    }
}
