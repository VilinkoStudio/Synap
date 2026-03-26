//! FFI-compatible type conversions for Synap.

use synap_core::dto::NoteDTO as CoreNoteDto;

/// A note DTO that is friendly to UniFFI/Kotlin consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteDTO {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: i64,
}

impl From<CoreNoteDto> for NoteDTO {
    fn from(note: CoreNoteDto) -> Self {
        Self {
            id: note.id,
            content: note.content,
            tags: note.tags,
            created_at: note.created_at as i64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_dto_conversion() {
        let core_note = CoreNoteDto {
            id: "0195f9a8-d085-7f9d-a604-469e0f91d0e3".to_string(),
            content: "Test content".to_string(),
            tags: vec!["rust".to_string(), "android".to_string()],
            created_at: 1_742_165_200_000,
        };

        let ffi_note: NoteDTO = core_note.into();
        assert_eq!(ffi_note.id, "0195f9a8-d085-7f9d-a604-469e0f91d0e3");
        assert_eq!(ffi_note.content, "Test content");
        assert_eq!(ffi_note.tags, vec!["rust", "android"]);
        assert_eq!(ffi_note.created_at, 1_742_165_200_000);
    }
}
