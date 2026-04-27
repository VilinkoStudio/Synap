use std::collections::HashSet;

use similar::{ChangeTag, TextDiff};

use crate::{
    dto::{
        NoteContentDiffStatsDTO, NoteTagDiffDTO, NoteTextChangeDTO, NoteTextChangeKindDTO,
        NoteVersionDTO, NoteVersionDiffDTO,
    },
    error::NoteError,
    models::note::{Note, NoteReader},
    views::note_view::NoteView,
};

pub(crate) struct NoteVersionView<'a, 'b: 'a> {
    reader: &'a NoteReader<'b>,
    base: Note,
    version: Note,
}

impl<'a, 'b> NoteVersionView<'a, 'b> {
    pub(crate) fn new(reader: &'a NoteReader<'b>, base: Note, version: Note) -> Self {
        Self {
            reader,
            base,
            version,
        }
    }

    pub(crate) fn to_dto(&self) -> Result<NoteVersionDTO, NoteError> {
        let base_view = NoteView::new(self.reader, self.base.clone());
        let version_view = NoteView::new(self.reader, self.version.clone());
        let base_tags = base_view
            .tags()?
            .into_iter()
            .map(|tag| tag.get_content().to_string())
            .collect::<Vec<_>>();
        let version_tags = version_view
            .tags()?
            .into_iter()
            .map(|tag| tag.get_content().to_string())
            .collect::<Vec<_>>();
        let content = Self::diff_content(self.base.content(), self.version.content());
        let content_stats =
            Self::content_diff_stats(self.base.content(), self.version.content(), &content);

        Ok(NoteVersionDTO {
            note: version_view.to_dto()?,
            diff: NoteVersionDiffDTO {
                tags: Self::diff_tags(&base_tags, &version_tags),
                content_summary: Self::summarize_content_diff(&content),
                content_stats,
                content,
            },
        })
    }

    fn diff_tags(base_tags: &[String], version_tags: &[String]) -> NoteTagDiffDTO {
        let base_set = base_tags.iter().cloned().collect::<HashSet<_>>();
        let version_set = version_tags.iter().cloned().collect::<HashSet<_>>();

        let added = version_tags
            .iter()
            .filter(|tag| !base_set.contains(*tag))
            .cloned()
            .collect();
        let removed = base_tags
            .iter()
            .filter(|tag| !version_set.contains(*tag))
            .cloned()
            .collect();

        NoteTagDiffDTO { added, removed }
    }

    fn diff_content(base: &str, version: &str) -> Vec<NoteTextChangeDTO> {
        let diff = TextDiff::from_chars(base, version);
        let mut changes: Vec<NoteTextChangeDTO> = Vec::new();

        for change in diff.iter_all_changes() {
            let kind = match change.tag() {
                ChangeTag::Equal => NoteTextChangeKindDTO::Equal,
                ChangeTag::Insert => NoteTextChangeKindDTO::Insert,
                ChangeTag::Delete => NoteTextChangeKindDTO::Delete,
            };

            let value = change.to_string();
            if value.is_empty() {
                continue;
            }

            if let Some(last) = changes.last_mut() {
                if last.kind == kind {
                    last.value.push_str(&value);
                    continue;
                }
            }

            changes.push(NoteTextChangeDTO { kind, value });
        }

        changes
    }

    fn summarize_content_diff(changes: &[NoteTextChangeDTO]) -> Vec<NoteTextChangeDTO> {
        [NoteTextChangeKindDTO::Insert, NoteTextChangeKindDTO::Delete]
            .into_iter()
            .filter_map(|kind| {
                let fragments = changes
                    .iter()
                    .filter(|change| change.kind == kind)
                    .map(|change| Self::normalize_summary_fragment(&change.value))
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>();

                if fragments.is_empty() {
                    return None;
                }

                let joined = fragments.join(" / ");
                Some(NoteTextChangeDTO {
                    kind,
                    value: Self::truncate_summary(&joined, 120),
                })
            })
            .collect()
    }

    fn content_diff_stats(
        base: &str,
        version: &str,
        changes: &[NoteTextChangeDTO],
    ) -> NoteContentDiffStatsDTO {
        let inserted_chars = Self::accumulate_chars(changes, NoteTextChangeKindDTO::Insert);
        let deleted_chars = Self::accumulate_chars(changes, NoteTextChangeKindDTO::Delete);
        let (inserted_lines, deleted_lines) = Self::count_changed_lines(base, version);

        NoteContentDiffStatsDTO {
            inserted_chars,
            deleted_chars,
            inserted_lines,
            deleted_lines,
        }
    }

    fn accumulate_chars(changes: &[NoteTextChangeDTO], kind: NoteTextChangeKindDTO) -> u32 {
        changes
            .iter()
            .filter(|change| change.kind == kind)
            .fold(0_u32, |chars, change| {
                chars.saturating_add(change.value.chars().count() as u32)
            })
    }

    fn count_changed_lines(base: &str, version: &str) -> (u32, u32) {
        let base_lines = base.lines().collect::<Vec<_>>();
        let version_lines = version.lines().collect::<Vec<_>>();
        let common_lines = Self::lcs_line_count(&base_lines, &version_lines);

        (
            version_lines.len().saturating_sub(common_lines) as u32,
            base_lines.len().saturating_sub(common_lines) as u32,
        )
    }

    fn lcs_line_count(base_lines: &[&str], version_lines: &[&str]) -> usize {
        if base_lines.is_empty() || version_lines.is_empty() {
            return 0;
        }

        let mut prev = vec![0_usize; version_lines.len() + 1];
        let mut curr = vec![0_usize; version_lines.len() + 1];

        for base_line in base_lines {
            curr[0] = 0;

            for (j, version_line) in version_lines.iter().enumerate() {
                curr[j + 1] = if base_line == version_line {
                    prev[j] + 1
                } else {
                    prev[j + 1].max(curr[j])
                };
            }

            std::mem::swap(&mut prev, &mut curr);
        }

        prev[version_lines.len()]
    }

    fn normalize_summary_fragment(value: &str) -> String {
        value
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim_matches('/')
            .trim()
            .to_string()
    }

    fn truncate_summary(value: &str, max_chars: usize) -> String {
        let mut result = String::new();

        for (idx, ch) in value.chars().enumerate() {
            if idx >= max_chars {
                result.push('…');
                return result;
            }
            result.push(ch);
        }

        result
    }
}
