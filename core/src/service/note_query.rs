use super::*;

impl SynapService {
    pub(crate) fn note_to_dto(
        &self,
        value: Note,
        reader: &NoteReader<'_>,
    ) -> Result<NoteDTO, ServiceError> {
        let view = NoteView::new(reader, value);
        view.to_dto().map_err(Into::into)
    }

    pub(crate) fn note_ref_to_dto(
        &self,
        note_ref: NoteRef,
        reader: &NoteReader<'_>,
    ) -> Result<NoteDTO, ServiceError> {
        let note = note_ref
            .hydrate(reader)?
            .ok_or(ServiceError::NotFound(note_ref.get_id().to_string()))?;
        self.note_to_dto(note, reader)
    }

    pub(crate) fn encode_timeline_cursor(note_id: Uuid) -> String {
        note_id.to_string()
    }

    pub(crate) fn decode_timeline_cursor(cursor: Option<&str>) -> Result<Option<Uuid>, ServiceError> {
        cursor.map(Self::parse_id).transpose()
    }

    pub(crate) fn finalize_note_page(mut notes: Vec<NoteDTO>, limit: usize) -> TimelineNotesPageDTO {
        let has_more = notes.len() > limit;
        if has_more {
            notes.pop();
        }

        let next_cursor = if has_more {
            notes
                .last()
                .and_then(|note| Self::parse_id(&note.id).ok())
                .map(Self::encode_timeline_cursor)
        } else {
            None
        };

        TimelineNotesPageDTO { notes, next_cursor }
    }

    pub(crate) fn timeline_bounds(
        direction: TimelineDirection,
        cursor: Option<Uuid>,
    ) -> (Bound<Uuid>, Bound<Uuid>, bool) {
        match (direction, cursor) {
            (TimelineDirection::Older, Some(anchor)) => {
                (Bound::Unbounded, Bound::Excluded(anchor), true)
            }
            (TimelineDirection::Newer, Some(anchor)) => {
                (Bound::Excluded(anchor), Bound::Unbounded, false)
            }
            (TimelineDirection::Older, None) => (Bound::Unbounded, Bound::Unbounded, true),
            (TimelineDirection::Newer, None) => (Bound::Unbounded, Bound::Unbounded, false),
        }
    }

    pub(crate) fn session_span_to_dto(
        &self,
        session: &SessionSpan,
        timeline: &TimelineView<'_>,
        reader: &NoteReader<'_>,
    ) -> Result<TimelineSessionDTO, ServiceError> {
        let notes = timeline
            .refs_in_session(session)?
            .map(|res| -> Result<NoteDTO, ServiceError> {
                let note_ref = res.map_err(ServiceError::from)?;
                self.note_ref_to_dto(note_ref, reader)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(TimelineSessionDTO {
            started_at: session.started_at(),
            ended_at: session.ended_at(),
            note_count: notes.len() as u32,
            notes,
        })
    }

    pub(crate) fn require_note_ref(
        reader: &NoteReader<'_>,
        id: Uuid,
        original: &str,
    ) -> Result<NoteRef, ServiceError> {
        reader
            .get_ref_by_id(&id)?
            .ok_or(ServiceError::NotFound(original.to_string()))
    }

    pub(crate) fn require_live_note_ref(
        reader: &NoteReader<'_>,
        id: Uuid,
        original: &str,
    ) -> Result<NoteRef, ServiceError> {
        let note_ref = Self::require_note_ref(reader, id, original)?;
        if note_ref.is_deleted() {
            return Err(ServiceError::NotFound(original.to_string()));
        }
        Ok(note_ref)
    }

    pub(crate) fn require_note(
        reader: &NoteReader<'_>,
        id: Uuid,
        original: &str,
    ) -> Result<Note, ServiceError> {
        reader
            .get_by_id(&id)?
            .ok_or(ServiceError::NotFound(original.to_string()))
    }

    pub(crate) fn require_live_note(
        reader: &NoteReader<'_>,
        id: Uuid,
        original: &str,
    ) -> Result<Note, ServiceError> {
        let note = Self::require_note(reader, id, original)?;
        if note.is_deleted() {
            return Err(ServiceError::NotFound(original.to_string()));
        }
        Ok(note)
    }

    pub(crate) fn resolve_tag(tx: &ReadTransaction, content: &str) -> Result<Option<Tag>, ServiceError> {
        TagReader::new(tx)?
            .find_by_content(content)
            .map_err(Into::into)
    }

    pub(crate) fn is_latest_version(
        reader: &NoteReader<'_>,
        note_ref: NoteRef,
    ) -> Result<bool, ServiceError> {
        Ok(!reader
            .has_next_version(&note_ref.get_id())
            .map_err(redb::Error::from)?)
    }

    pub(crate) fn matches_selected_tags(
        note: &Note,
        selected_tag_ids: &HashSet<Uuid>,
        include_untagged: bool,
    ) -> bool {
        if note.tags().is_empty() {
            return include_untagged;
        }

        note.tags()
            .iter()
            .any(|tag_id| selected_tag_ids.contains(tag_id))
    }

    /// 获取单条笔记的完整视图
    pub fn get_note(&self, id_or_short_id: &str) -> Result<NoteDTO, ServiceError> {
        self.with_read(|_tx, reader| {
            let note = match id_or_short_id.len() {
                36_usize | 32_usize => reader.get_by_id(&Self::parse_id(id_or_short_id)?)?,
                8 => reader.get_by_short_id(id_or_short_id.as_bytes().try_into()?)?,
                _ => return Err(ServiceError::InvalidId),
            };
            let note = note.ok_or(ServiceError::NotFound(id_or_short_id.to_string()))?;
            if note.is_deleted() {
                return Err(ServiceError::NotFound(id_or_short_id.to_string()));
            }
            self.note_to_dto(note, reader)
        })
    }

    /// 获取子节点（瀑布流/无限滚动）核心接口！
    /// cursor: 前端传列表里最后一条数据的 Uuid。如果是第一次加载，传 None。
    pub fn get_replies(
        &self,
        parent_id: &str,
        cursor: Option<String>,
        limit: usize,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(parent_id)?;
            let note = Self::require_live_note(reader, uuid, parent_id)?;
            let cursor_uuid = cursor.map(|c| Self::parse_id(&c)).transpose()?;
            let view = NoteView::new(reader, note);
            let mut children_iter = view.children_refs()?;

            if let Some(target_id) = cursor_uuid {
                // 使用 .by_ref() 借用迭代器，不断消耗元素，直到找到游标
                for res in children_iter.by_ref() {
                    let child_ref = res.map_err(ServiceError::from)?;

                    if child_ref.get_id() == target_id {
                        break;
                    }
                }
            }

            children_iter
                .take(limit)
                .map(|res| -> Result<NoteDTO, ServiceError> {
                    let child_ref = res.map_err(ServiceError::from)?;
                    self.note_ref_to_dto(child_ref, reader)
                })
                .collect::<Result<Vec<_>, _>>()
        })
    }

    pub fn get_recent_notes_page(
        &self,
        cursor: Option<&str>,
        direction: TimelineDirection,
        limit: Option<usize>,
    ) -> Result<TimelineNotesPageDTO, ServiceError> {
        self.with_read(|_tx, reader| {
            let limit = limit.unwrap_or(20);
            let cursor_uuid = Self::decode_timeline_cursor(cursor)?;
            let timeline = TimelineView::new(reader);
            let mut notes = Vec::with_capacity(limit.saturating_add(1));
            let note_refs: Box<dyn Iterator<Item = Result<NoteRef, crate::error::NoteError>> + '_> =
                match (direction, cursor_uuid) {
                    (TimelineDirection::Older, Some(anchor)) => {
                        let split = timeline.split_refs_from(TimelinePoint::NoteId(anchor))?;
                        split.older
                    }
                    (TimelineDirection::Newer, Some(anchor)) => {
                        let split = timeline.split_refs_from(TimelinePoint::NoteId(anchor))?;
                        split.newer
                    }
                    (TimelineDirection::Older, None) => Box::new(timeline.recent_refs()?),
                    (TimelineDirection::Newer, None) => Box::new(timeline.oldest_refs()?),
                };

            for res in note_refs {
                let note_ref = res.map_err(ServiceError::from)?;
                if !Self::is_latest_version(reader, note_ref)? {
                    continue;
                }

                notes.push(self.note_ref_to_dto(note_ref, reader)?);
                if notes.len() > limit {
                    break;
                }
            }

            Ok(Self::finalize_note_page(notes, limit))
        })
    }

    pub fn get_recent_note(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        self.get_recent_notes_page(cursor, TimelineDirection::Older, limit)
            .map(|page| page.notes)
    }

    pub fn get_recent_sessions(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<TimelineSessionsPageDTO, ServiceError> {
        self.with_read(|_tx, reader| {
            let limit = limit.unwrap_or(20);
            let cursor_uuid = cursor.map(Self::parse_id).transpose()?;
            let timeline = TimelineView::new(reader);
            let session_iter = timeline.recent_session_spans(DEFAULT_SESSION_DETECTION_CONFIG)?;
            let mut cursor_seen = cursor_uuid.is_none();
            let mut spans = Vec::with_capacity(limit.saturating_add(1));

            for session in session_iter {
                if !cursor_seen {
                    if cursor_uuid
                        .as_ref()
                        .is_some_and(|target_id| session.cursor() == *target_id)
                    {
                        cursor_seen = true;
                    }
                    continue;
                }

                spans.push(session);
                if spans.len() > limit {
                    break;
                }
            }

            let has_more = spans.len() > limit;
            if has_more {
                spans.pop();
            }

            let next_cursor = if has_more {
                spans.last().map(|session| session.cursor().to_string())
            } else {
                None
            };

            let sessions = spans
                .iter()
                .map(|session| self.session_span_to_dto(session, &timeline, reader))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(TimelineSessionsPageDTO {
                sessions,
                next_cursor,
            })
        })
    }

    pub fn get_origins(&self, child_id: &str) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let child_uuid = Self::parse_id(child_id)?;
            let child = Self::require_live_note(reader, child_uuid, child_id)?;
            let view = NoteView::new(reader, child);
            let parents_iter = view.parents_refs()?;

            parents_iter
                .map(|res| -> Result<NoteDTO, ServiceError> {
                    let parent_ref = res.map_err(ServiceError::from)?;
                    self.note_ref_to_dto(parent_ref, reader)
                })
                .collect::<Result<Vec<_>, _>>()
        })
    }

    pub fn get_previous_versions(&self, note_id: &str) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(note_id)?;
            let note = Self::require_live_note(reader, uuid, note_id)?;
            let view = NoteView::new(reader, note);
            let versions = view.history_refs()?;
            let mut results = Vec::new();

            for res in versions {
                let version_ref = res.map_err(ServiceError::from)?;
                results.push(self.note_ref_to_dto(version_ref, reader)?);
            }

            Ok(results)
        })
    }

    pub fn get_next_versions(&self, note_id: &str) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(note_id)?;
            let note = Self::require_live_note(reader, uuid, note_id)?;
            let view = NoteView::new(reader, note);
            let versions = view.next_version_refs()?;
            let mut results = Vec::new();

            for res in versions {
                let version_ref = res.map_err(ServiceError::from)?;
                results.push(self.note_ref_to_dto(version_ref, reader)?);
            }

            Ok(results)
        })
    }

    pub fn get_other_versions(&self, note_id: &str) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(note_id)?;
            let note = Self::require_live_note(reader, uuid, note_id)?;
            let view = NoteView::new(reader, note);
            let versions = view.other_versions_refs()?;
            let mut results = Vec::new();

            for res in versions {
                let version_ref = res.map_err(ServiceError::from)?;
                results.push(self.note_ref_to_dto(version_ref, reader)?);
            }

            Ok(results)
        })
    }

    pub fn get_deleted_notes(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let limit = limit.unwrap_or(20);
            let cursor_uuid = cursor.map(Self::parse_id).transpose()?;
            let mut cursor_seen = cursor_uuid.is_none();
            let mut notes = Vec::with_capacity(limit);
            let deleted_ids = reader.deleted_note_ids().map_err(redb::Error::from)?;

            for deleted_id in deleted_ids.rev() {
                let deleted_id = deleted_id.map_err(redb::Error::from)?;

                if !cursor_seen {
                    if cursor_uuid
                        .as_ref()
                        .is_some_and(|target_id| deleted_id == *target_id)
                    {
                        cursor_seen = true;
                    }
                    continue;
                }

                let note_ref = Self::require_note_ref(reader, deleted_id, &deleted_id.to_string())?;
                notes.push(self.note_ref_to_dto(note_ref, reader)?);
                if notes.len() == limit {
                    break;
                }
            }

            Ok(notes)
        })
    }

    /// tag-centric 的便捷查询。
    ///
    /// 它复用的是 `tag -> note` 关系，因此更适合标签页、标签搜索结果等
    /// “从标签出发”的场景；如果需求是首页/时间流那种“全局时间轴 + 游标”
    /// 过滤，请走 `get_filtered_notes`，不要和这里混用。
    pub fn get_notes_by_tag(
        &self,
        tag: &str,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        self.with_read(|tx, reader| {
            let limit = limit.unwrap_or(20);
            let cursor_uuid = cursor.map(Self::parse_id).transpose()?;
            let Some(tag) = Self::resolve_tag(tx, tag)? else {
                return Ok(Vec::new());
            };

            let tagged_iter = reader.latest_notes_with_tag(&tag)?;
            let mut cursor_seen = cursor_uuid.is_none();
            let mut notes = Vec::with_capacity(limit);

            for note in tagged_iter {
                let note = note?;

                if !cursor_seen {
                    if cursor_uuid
                        .as_ref()
                        .is_some_and(|target_id| note.get_id() == *target_id)
                    {
                        cursor_seen = true;
                    }
                    continue;
                }

                notes.push(self.note_to_dto(note, reader)?);
                if notes.len() == limit {
                    break;
                }
            }

            Ok(notes)
        })
    }

    /// 时间轴过滤页。
    ///
    /// 新接口返回服务端维护的分页 token，而不是要求调用方自己记最后一条
    /// note 的 id。对于 normal feed，会优先走时间轴的双向切分能力；
    /// 对 deleted/all 这类不完全等价于 live timeline 的视图，则退回到
    /// 基于时间范围的原始扫描。
    pub fn get_filtered_notes_page(
        &self,
        selected_tags: Vec<String>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        status: FilteredNoteStatus,
        cursor: Option<&str>,
        direction: TimelineDirection,
        limit: Option<usize>,
    ) -> Result<TimelineNotesPageDTO, ServiceError> {
        let limit = limit.unwrap_or(20);

        if !tag_filter_enabled {
            return match status {
                FilteredNoteStatus::Normal => {
                    self.get_recent_notes_page(cursor, direction, Some(limit))
                }
                FilteredNoteStatus::Deleted => self.with_read(|_tx, reader| {
                    let cursor_uuid = Self::decode_timeline_cursor(cursor)?;
                    let (start, end, reverse) = Self::timeline_bounds(direction, cursor_uuid);
                    let deleted_ids: Box<
                        dyn Iterator<Item = Result<Uuid, redb::StorageError>> + '_,
                    > = if reverse {
                        Box::new(
                            reader
                                .deleted_note_ids_range(start, end)
                                .map_err(redb::Error::from)?
                                .rev(),
                        )
                    } else {
                        Box::new(
                            reader
                                .deleted_note_ids_range(start, end)
                                .map_err(redb::Error::from)?,
                        )
                    };
                    let mut notes = Vec::with_capacity(limit.saturating_add(1));

                    for note_id in deleted_ids {
                        let note_id = note_id.map_err(redb::Error::from)?;
                        let note_ref =
                            Self::require_note_ref(reader, note_id, &note_id.to_string())?;
                        notes.push(self.note_ref_to_dto(note_ref, reader)?);
                        if notes.len() > limit {
                            break;
                        }
                    }

                    Ok(Self::finalize_note_page(notes, limit))
                }),
                FilteredNoteStatus::All => self.with_read(|_tx, reader| {
                    let cursor_uuid = Self::decode_timeline_cursor(cursor)?;
                    let (start, end, reverse) = Self::timeline_bounds(direction, cursor_uuid);
                    let note_ids: Box<dyn Iterator<Item = Result<Uuid, redb::StorageError>> + '_> =
                        if reverse {
                            Box::new(
                                reader
                                    .note_by_time_range(start, end)
                                    .map_err(redb::Error::from)?
                                    .rev(),
                            )
                        } else {
                            Box::new(
                                reader
                                    .note_by_time_range(start, end)
                                    .map_err(redb::Error::from)?,
                            )
                        };
                    let mut notes = Vec::with_capacity(limit.saturating_add(1));

                    for note_id in note_ids {
                        let note_id = note_id.map_err(redb::Error::from)?;
                        let note_ref =
                            Self::require_note_ref(reader, note_id, &note_id.to_string())?;

                        if !Self::is_latest_version(reader, note_ref)? {
                            continue;
                        }

                        notes.push(self.note_ref_to_dto(note_ref, reader)?);
                        if notes.len() > limit {
                            break;
                        }
                    }

                    Ok(Self::finalize_note_page(notes, limit))
                }),
            };
        }

        let selected_tag_ids = Self::normalize_tag_inputs(selected_tags)
            .into_iter()
            .filter_map(|tag| Tag::id_for_content(&tag))
            .collect::<HashSet<_>>();

        if selected_tag_ids.is_empty() && !include_untagged {
            return Ok(TimelineNotesPageDTO {
                notes: Vec::new(),
                next_cursor: None,
            });
        }

        self.with_read(|_tx, reader| {
            let cursor_uuid = Self::decode_timeline_cursor(cursor)?;
            let mut notes = Vec::with_capacity(limit.saturating_add(1));

            let mut maybe_push = |note_ref: NoteRef| -> Result<bool, ServiceError> {
                if !Self::is_latest_version(reader, note_ref)? {
                    return Ok(false);
                }

                let note = note_ref
                    .hydrate(reader)?
                    .ok_or(ServiceError::NotFound(note_ref.get_id().to_string()))?;

                if !Self::matches_selected_tags(&note, &selected_tag_ids, include_untagged) {
                    return Ok(false);
                }

                notes.push(self.note_to_dto(note, reader)?);
                Ok(notes.len() > limit)
            };

            match status {
                FilteredNoteStatus::Normal => {
                    let timeline = TimelineView::new(reader);
                    let note_refs: Box<
                        dyn Iterator<Item = Result<NoteRef, crate::error::NoteError>> + '_,
                    > = match (direction, cursor_uuid) {
                        (TimelineDirection::Older, Some(anchor)) => {
                            let split = timeline.split_refs_from(TimelinePoint::NoteId(anchor))?;
                            split.older
                        }
                        (TimelineDirection::Newer, Some(anchor)) => {
                            let split = timeline.split_refs_from(TimelinePoint::NoteId(anchor))?;
                            split.newer
                        }
                        (TimelineDirection::Older, None) => Box::new(timeline.recent_refs()?),
                        (TimelineDirection::Newer, None) => Box::new(timeline.oldest_refs()?),
                    };

                    for note_ref in note_refs {
                        if maybe_push(note_ref.map_err(ServiceError::from)?)? {
                            break;
                        }
                    }
                }
                FilteredNoteStatus::Deleted => {
                    let (start, end, reverse) = Self::timeline_bounds(direction, cursor_uuid);
                    let deleted_ids: Box<
                        dyn Iterator<Item = Result<Uuid, redb::StorageError>> + '_,
                    > = if reverse {
                        Box::new(
                            reader
                                .deleted_note_ids_range(start, end)
                                .map_err(redb::Error::from)?
                                .rev(),
                        )
                    } else {
                        Box::new(
                            reader
                                .deleted_note_ids_range(start, end)
                                .map_err(redb::Error::from)?,
                        )
                    };

                    for note_id in deleted_ids {
                        let note_id = note_id.map_err(redb::Error::from)?;
                        let note_ref =
                            Self::require_note_ref(reader, note_id, &note_id.to_string())?;
                        if maybe_push(note_ref)? {
                            break;
                        }
                    }
                }
                FilteredNoteStatus::All => {
                    let (start, end, reverse) = Self::timeline_bounds(direction, cursor_uuid);
                    let note_ids: Box<dyn Iterator<Item = Result<Uuid, redb::StorageError>> + '_> =
                        if reverse {
                            Box::new(
                                reader
                                    .note_by_time_range(start, end)
                                    .map_err(redb::Error::from)?
                                    .rev(),
                            )
                        } else {
                            Box::new(
                                reader
                                    .note_by_time_range(start, end)
                                    .map_err(redb::Error::from)?,
                            )
                        };

                    for note_id in note_ids {
                        let note_id = note_id.map_err(redb::Error::from)?;
                        let note_ref =
                            Self::require_note_ref(reader, note_id, &note_id.to_string())?;
                        if maybe_push(note_ref)? {
                            break;
                        }
                    }
                }
            }

            Ok(Self::finalize_note_page(notes, limit))
        })
    }

    /// 时间轴过滤的唯一入口。
    ///
    /// 兼容旧的“只向旧内容翻页”调用；新的分页 token 入口请走
    /// `get_filtered_notes_page`。
    pub fn get_filtered_notes(
        &self,
        selected_tags: Vec<String>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        status: FilteredNoteStatus,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        self.get_filtered_notes_page(
            selected_tags,
            include_untagged,
            tag_filter_enabled,
            status,
            cursor,
            TimelineDirection::Older,
            limit,
        )
        .map(|page| page.notes)
    }
}
