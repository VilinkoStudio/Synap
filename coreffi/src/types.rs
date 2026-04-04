//! FFI-compatible type conversions for Synap.

use synap_core::dto::{
    NoteDTO as CoreNoteDto,
    TimelineNotesPageDTO as CoreTimelineNotesPageDto,
    TimelineSessionDTO as CoreTimelineSessionDto,
    TimelineSessionsPageDTO as CoreTimelineSessionsPageDto,
};
use synap_core::service::FilteredNoteStatus as CoreFilteredNoteStatus;
use synap_core::service::TimelineDirection as CoreTimelineDirection;
use synap_core::BuildInfo as CoreBuildInfo;

/// A note DTO that is friendly to UniFFI/Kotlin consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteDTO {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub deleted: bool,
}

impl From<CoreNoteDto> for NoteDTO {
    fn from(note: CoreNoteDto) -> Self {
        Self {
            id: note.id,
            content: note.content,
            tags: note.tags,
            created_at: note.created_at as i64,
            deleted: note.deleted,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineNotesPageDTO {
    pub notes: Vec<NoteDTO>,
    pub next_cursor: Option<String>,
}

impl From<CoreTimelineNotesPageDto> for TimelineNotesPageDTO {
    fn from(page: CoreTimelineNotesPageDto) -> Self {
        Self {
            notes: page.notes.into_iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineSessionDTO {
    pub started_at: i64,
    pub ended_at: i64,
    pub note_count: u32,
    pub notes: Vec<NoteDTO>,
}

impl From<CoreTimelineSessionDto> for TimelineSessionDTO {
    fn from(session: CoreTimelineSessionDto) -> Self {
        Self {
            started_at: session.started_at as i64,
            ended_at: session.ended_at as i64,
            note_count: session.note_count,
            notes: session.notes.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineSessionsPageDTO {
    pub sessions: Vec<TimelineSessionDTO>,
    pub next_cursor: Option<String>,
}

impl From<CoreTimelineSessionsPageDto> for TimelineSessionsPageDTO {
    fn from(page: CoreTimelineSessionsPageDto) -> Self {
        Self {
            sessions: page.sessions.into_iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildInfo {
    pub crate_version: String,
    pub git_branch: String,
    pub git_commit: String,
    pub git_short_commit: String,
    pub git_tag: Option<String>,
    pub display_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilteredNoteStatus {
    All,
    Normal,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineDirection {
    Older,
    Newer,
}

impl From<FilteredNoteStatus> for CoreFilteredNoteStatus {
    fn from(status: FilteredNoteStatus) -> Self {
        match status {
            FilteredNoteStatus::All => CoreFilteredNoteStatus::All,
            FilteredNoteStatus::Normal => CoreFilteredNoteStatus::Normal,
            FilteredNoteStatus::Deleted => CoreFilteredNoteStatus::Deleted,
        }
    }
}

impl From<TimelineDirection> for CoreTimelineDirection {
    fn from(direction: TimelineDirection) -> Self {
        match direction {
            TimelineDirection::Older => CoreTimelineDirection::Older,
            TimelineDirection::Newer => CoreTimelineDirection::Newer,
        }
    }
}

impl From<CoreBuildInfo> for BuildInfo {
    fn from(info: CoreBuildInfo) -> Self {
        let display_version = info.display_version();
        Self {
            crate_version: info.crate_version.to_string(),
            git_branch: info.git_branch.to_string(),
            git_commit: info.git_commit.to_string(),
            git_short_commit: info.git_short_commit.to_string(),
            git_tag: info.git_tag.map(ToString::to_string),
            display_version,
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
            deleted: false,
        };

        let ffi_note: NoteDTO = core_note.into();
        assert_eq!(ffi_note.id, "0195f9a8-d085-7f9d-a604-469e0f91d0e3");
        assert_eq!(ffi_note.content, "Test content");
        assert_eq!(ffi_note.tags, vec!["rust", "android"]);
        assert_eq!(ffi_note.created_at, 1_742_165_200_000);
        assert!(!ffi_note.deleted);
    }

    #[test]
    fn test_build_info_conversion() {
        let core_info = CoreBuildInfo {
            crate_version: "0.1.0",
            git_branch: "main",
            git_commit: "abcdef0123456789",
            git_short_commit: "abcdef012345",
            git_tag: Some("v0.1.0"),
        };

        let ffi_info: BuildInfo = core_info.into();
        assert_eq!(ffi_info.crate_version, "0.1.0");
        assert_eq!(ffi_info.git_branch, "main");
        assert_eq!(ffi_info.git_commit, "abcdef0123456789");
        assert_eq!(ffi_info.git_short_commit, "abcdef012345");
        assert_eq!(ffi_info.git_tag.as_deref(), Some("v0.1.0"));
        assert_eq!(ffi_info.display_version, "v0.1.0 (abcdef012345)");
    }

    #[test]
    fn test_filtered_note_status_conversion() {
        assert_eq!(
            CoreFilteredNoteStatus::from(FilteredNoteStatus::All),
            CoreFilteredNoteStatus::All
        );
        assert_eq!(
            CoreFilteredNoteStatus::from(FilteredNoteStatus::Normal),
            CoreFilteredNoteStatus::Normal
        );
        assert_eq!(
            CoreFilteredNoteStatus::from(FilteredNoteStatus::Deleted),
            CoreFilteredNoteStatus::Deleted
        );
    }
}
