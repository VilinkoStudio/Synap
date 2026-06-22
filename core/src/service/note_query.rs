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

    pub(crate) fn note_ref_to_version_dto(
        &self,
        base: &Note,
        note_ref: NoteRef,
        reader: &NoteReader<'_>,
    ) -> Result<NoteVersionDTO, ServiceError> {
        let version = note_ref
            .hydrate(reader)?
            .ok_or(ServiceError::NotFound(note_ref.get_id().to_string()))?;
        self.note_to_version_dto(base, version, reader)
    }

    pub(crate) fn note_to_version_dto(
        &self,
        base: &Note,
        version: Note,
        reader: &NoteReader<'_>,
    ) -> Result<NoteVersionDTO, ServiceError> {
        NoteVersionView::new(reader, base.clone(), version)
            .to_dto()
            .map_err(Into::into)
    }

    pub(crate) fn encode_timeline_cursor(note_id: Uuid) -> String {
        note_id.to_string()
    }

    pub(crate) fn decode_timeline_cursor(
        cursor: Option<&str>,
    ) -> Result<Option<Uuid>, ServiceError> {
        cursor.map(Self::parse_id).transpose()
    }

    pub(crate) fn timeline_cursor_for_timestamp_ms(
        timestamp_ms: u64,
        direction: TimelineDirection,
    ) -> Uuid {
        let entropy = match direction {
            TimelineDirection::Older => [0xFF; 10],
            TimelineDirection::Newer => [0; 10],
        };
        Builder::from_unix_timestamp_millis(timestamp_ms, &entropy).into_uuid()
    }

    pub(crate) fn finalize_note_page(
        mut notes: Vec<NoteDTO>,
        limit: usize,
    ) -> TimelineNotesPageDTO {
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

    pub(crate) fn timestamp_ms_from_id(id: Uuid) -> Result<u64, ServiceError> {
        let (seconds, nanos) = id.get_timestamp().ok_or(ServiceError::InvalidId)?.to_unix();
        Ok(seconds.saturating_mul(1000) + u64::from(nanos / 1_000_000))
    }

    pub(crate) fn matches_timeline_status(
        reader: &NoteReader<'_>,
        note_ref: NoteRef,
        status: FilteredNoteStatus,
    ) -> Result<bool, ServiceError> {
        match status {
            FilteredNoteStatus::Normal => Ok(!note_ref.is_deleted()),
            FilteredNoteStatus::Deleted => Ok(note_ref.is_deleted()),
            FilteredNoteStatus::All => Ok(true),
        }
        .and_then(|matches_status| {
            if !matches_status {
                return Ok(false);
            }
            Self::is_latest_version(reader, note_ref)
        })
    }

    pub(crate) fn filtered_timeline_note_refs<'a>(
        &'a self,
        reader: &'a NoteReader<'a>,
        selected_tag_ids: HashSet<Uuid>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        status: FilteredNoteStatus,
        cursor_uuid: Option<Uuid>,
        direction: TimelineDirection,
    ) -> Result<Box<dyn Iterator<Item = Result<NoteRef, ServiceError>> + 'a>, ServiceError> {
        let (start, end, reverse) = Self::timeline_bounds(direction, cursor_uuid);
        let raw_iter = reader
            .note_by_time_range(start, end)
            .map_err(redb::Error::from)?;
        let note_ids: Box<dyn Iterator<Item = Result<Uuid, redb::StorageError>> + 'a> = if reverse {
            Box::new(raw_iter.rev())
        } else {
            Box::new(raw_iter)
        };

        Ok(Box::new(note_ids.filter_map(move |note_id_res| {
            let note_id = match note_id_res {
                Ok(note_id) => note_id,
                Err(err) => return Some(Err(ServiceError::Db(err.into()))),
            };
            let note_ref = match Self::require_note_ref(reader, note_id, &note_id.to_string()) {
                Ok(note_ref) => note_ref,
                Err(err) => return Some(Err(err)),
            };

            match Self::matches_timeline_status(reader, note_ref, status) {
                Ok(false) => return None,
                Ok(true) => {}
                Err(err) => return Some(Err(err)),
            }

            if tag_filter_enabled {
                let note = match note_ref.hydrate(reader) {
                    Ok(Some(note)) => note,
                    Ok(None) => return Some(Err(ServiceError::NotFound(note_id.to_string()))),
                    Err(err) => return Some(Err(ServiceError::Db(err))),
                };

                if !Self::matches_selected_tags(&note, &selected_tag_ids, include_untagged) {
                    return None;
                }
            }

            Some(Ok(note_ref))
        })))
    }

    pub(crate) fn apply_timeline_groups(
        notes: &mut [NoteDTO],
        direction: TimelineDirection,
    ) -> Result<(), ServiceError> {
        if notes.is_empty() {
            return Ok(());
        }

        let mut current_start = 0_u64;
        let mut current_end = 0_u64;
        let mut current_count = 0_u32;
        let mut current_indices = Vec::new();
        let mut previous_ts: Option<u64> = None;
        let mut groups = Vec::<(u64, u64, u32, Vec<usize>)>::new();

        for (index, note) in notes.iter().enumerate() {
            let id = Self::parse_id(&note.id)?;
            let timestamp_ms = Self::timestamp_ms_from_id(id)?;
            let starts_new_group = previous_ts.is_none_or(|previous| {
                previous.abs_diff(timestamp_ms) > DEFAULT_SESSION_DETECTION_CONFIG.split_gap_ms
            });

            if starts_new_group {
                if !current_indices.is_empty() {
                    groups.push((current_start, current_end, current_count, current_indices));
                    current_indices = Vec::new();
                }
                current_start = timestamp_ms;
                current_end = timestamp_ms;
                current_count = 0;
            } else {
                current_start = current_start.min(timestamp_ms);
                current_end = current_end.max(timestamp_ms);
            }

            current_count += 1;
            current_indices.push(index);
            previous_ts = Some(timestamp_ms);
        }

        if !current_indices.is_empty() {
            groups.push((current_start, current_end, current_count, current_indices));
        }

        for (started_at, ended_at, note_count, indices) in groups {
            let header_index = match direction {
                TimelineDirection::Older => indices[0],
                TimelineDirection::Newer => *indices.last().expect("group has at least one note"),
            };

            for index in indices {
                notes[index].timeline_group = Some(TimelineGroupDTO {
                    starts_group: index == header_index,
                    started_at,
                    ended_at,
                    note_count,
                });
            }
        }

        Ok(())
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

    pub(crate) fn resolve_tag(
        tx: &ReadTransaction,
        content: &str,
    ) -> Result<Option<Tag>, ServiceError> {
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

    #[deprecated(note = "use get_timeline_notes_page with group_sessions = false")]
    pub fn get_recent_notes_page(
        &self,
        cursor: Option<&str>,
        direction: TimelineDirection,
        limit: Option<usize>,
    ) -> Result<TimelineNotesPageDTO, ServiceError> {
        self.get_timeline_notes_page(
            Vec::new(),
            true,
            false,
            FilteredNoteStatus::Normal,
            false,
            cursor,
            direction,
            limit,
        )
    }

    #[deprecated(note = "use get_timeline_notes_page with TimelineDirection::Older")]
    pub fn get_recent_note(
        &self,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        #[allow(deprecated)]
        self.get_recent_notes_page(cursor, TimelineDirection::Older, limit)
            .map(|page| page.notes)
    }

    #[deprecated(note = "use get_timeline_notes_page with group_sessions = true")]
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

    pub fn get_previous_versions(
        &self,
        note_id: &str,
    ) -> Result<Vec<NoteVersionDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(note_id)?;
            let note = Self::require_live_note(reader, uuid, note_id)?;
            let view = NoteView::new(reader, note);
            let versions = view.history_refs()?;
            let mut results = Vec::new();

            for res in versions {
                let version_ref = res.map_err(ServiceError::from)?;
                results.push(self.note_ref_to_version_dto(view.get_note(), version_ref, reader)?);
            }

            Ok(results)
        })
    }

    pub fn get_note_segment(
        &self,
        anchor_id: &str,
        direction: NoteSegmentDirectionDTO,
    ) -> Result<NoteSegmentDTO, ServiceError> {
        self.with_read(|_tx, reader| {
            let anchor_uuid = Self::parse_id(anchor_id)?;
            Self::require_live_note(reader, anchor_uuid, anchor_id)?;
            NoteSegmentView::new(
                reader,
                anchor_uuid,
                match direction {
                    NoteSegmentDirectionDTO::Forward => NoteSegmentDirection::Forward,
                    NoteSegmentDirectionDTO::Backward => NoteSegmentDirection::Backward,
                },
            )?
            .to_dto()
            .map_err(Into::into)
        })
    }

    pub fn get_note_neighbors(&self, note_id: &str) -> Result<NoteNeighborsDTO, ServiceError> {
        self.with_read(|_tx, reader| {
            let note_uuid = Self::parse_id(note_id)?;
            Self::require_live_note(reader, note_uuid, note_id)?;
            NoteSegmentView::new(reader, note_uuid, NoteSegmentDirection::Forward)?
                .neighbors_dto(note_uuid)
                .map_err(Into::into)
        })
    }

    pub fn get_next_versions(&self, note_id: &str) -> Result<Vec<NoteVersionDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(note_id)?;
            let note = Self::require_live_note(reader, uuid, note_id)?;
            let view = NoteView::new(reader, note);
            let versions = view.next_version_refs()?;
            let mut results = Vec::new();

            for res in versions {
                let version_ref = res.map_err(ServiceError::from)?;
                results.push(self.note_ref_to_version_dto(view.get_note(), version_ref, reader)?);
            }

            Ok(results)
        })
    }

    pub fn get_other_versions(&self, note_id: &str) -> Result<Vec<NoteVersionDTO>, ServiceError> {
        self.with_read(|_tx, reader| {
            let uuid = Self::parse_id(note_id)?;
            let note = Self::require_live_note(reader, uuid, note_id)?;
            let view = NoteView::new(reader, note);
            let versions = view.other_versions_refs()?;
            let mut results = Vec::new();

            for res in versions {
                let version_ref = res.map_err(ServiceError::from)?;
                results.push(self.note_ref_to_version_dto(view.get_note(), version_ref, reader)?);
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
    #[deprecated(note = "use get_timeline_notes_page")]
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

        let selected_tag_ids = Self::normalize_tag_inputs(selected_tags)
            .into_iter()
            .filter_map(|tag| Tag::id_for_content(&tag))
            .collect::<HashSet<_>>();

        if tag_filter_enabled && selected_tag_ids.is_empty() && !include_untagged {
            return Ok(TimelineNotesPageDTO {
                notes: Vec::new(),
                next_cursor: None,
            });
        }

        self.get_timeline_notes_page_with_tag_ids(
            selected_tag_ids,
            include_untagged,
            tag_filter_enabled,
            status,
            false,
            cursor,
            direction,
            Some(limit),
        )
    }

    pub fn get_timeline_notes_page(
        &self,
        selected_tags: Vec<String>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        status: FilteredNoteStatus,
        group_sessions: bool,
        cursor: Option<&str>,
        direction: TimelineDirection,
        limit: Option<usize>,
    ) -> Result<TimelineNotesPageDTO, ServiceError> {
        let selected_tag_ids = Self::normalize_tag_inputs(selected_tags)
            .into_iter()
            .filter_map(|tag| Tag::id_for_content(&tag))
            .collect::<HashSet<_>>();

        if tag_filter_enabled && selected_tag_ids.is_empty() && !include_untagged {
            return Ok(TimelineNotesPageDTO {
                notes: Vec::new(),
                next_cursor: None,
            });
        }

        self.get_timeline_notes_page_with_tag_ids(
            selected_tag_ids,
            include_untagged,
            tag_filter_enabled,
            status,
            group_sessions,
            cursor,
            direction,
            limit,
        )
    }

    pub fn get_timeline_notes_around(
        &self,
        selected_tags: Vec<String>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        status: FilteredNoteStatus,
        group_sessions: bool,
        timestamp_ms: u64,
        direction: TimelineDirection,
        limit: Option<usize>,
    ) -> Result<TimelineNotesPageDTO, ServiceError> {
        let cursor = Self::timeline_cursor_for_timestamp_ms(timestamp_ms, direction);
        self.get_timeline_notes_page(
            selected_tags,
            include_untagged,
            tag_filter_enabled,
            status,
            group_sessions,
            Some(&cursor.to_string()),
            direction,
            limit,
        )
    }

    pub fn get_timeline_density(
        &self,
        selected_tags: Vec<String>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        status: FilteredNoteStatus,
        start_ms: u64,
        end_ms: u64,
        bucket_ms: u64,
    ) -> Result<Vec<TimelineDensityPointDTO>, ServiceError> {
        let selected_tag_ids = Self::normalize_tag_inputs(selected_tags)
            .into_iter()
            .filter_map(|tag| Tag::id_for_content(&tag))
            .collect::<HashSet<_>>();

        if bucket_ms == 0 || end_ms <= start_ms {
            return Ok(Vec::new());
        }

        if tag_filter_enabled && selected_tag_ids.is_empty() && !include_untagged {
            return Ok(Self::empty_density_points(start_ms, end_ms, bucket_ms));
        }

        self.with_read(|_tx, reader| {
            let bucket_count = (((end_ms - start_ms) + bucket_ms - 1) / bucket_ms) as usize;
            let mut counts = vec![0_u32; bucket_count];
            let cursor = Self::timeline_cursor_for_timestamp_ms(end_ms, TimelineDirection::Older);
            let note_refs = self.filtered_timeline_note_refs(
                reader,
                selected_tag_ids,
                include_untagged,
                tag_filter_enabled,
                status,
                Some(cursor),
                TimelineDirection::Older,
            )?;

            for note_ref in note_refs {
                let note_ref = note_ref?;
                let timestamp_ms = Self::timestamp_ms_from_id(note_ref.get_id())?;
                if timestamp_ms < start_ms {
                    break;
                }
                if timestamp_ms >= end_ms {
                    continue;
                }
                let index = ((timestamp_ms - start_ms) / bucket_ms) as usize;
                if let Some(count) = counts.get_mut(index) {
                    *count = count.saturating_add(1);
                }
            }

            Ok(counts
                .into_iter()
                .enumerate()
                .map(|(index, note_count)| {
                    let started_at = start_ms + (index as u64).saturating_mul(bucket_ms);
                    let ended_at = (started_at + bucket_ms).min(end_ms).saturating_sub(1);
                    TimelineDensityPointDTO {
                        started_at,
                        ended_at,
                        note_count,
                    }
                })
                .collect())
        })
    }

    fn empty_density_points(
        start_ms: u64,
        end_ms: u64,
        bucket_ms: u64,
    ) -> Vec<TimelineDensityPointDTO> {
        let bucket_count = (((end_ms - start_ms) + bucket_ms - 1) / bucket_ms) as usize;
        (0..bucket_count)
            .map(|index| {
                let started_at = start_ms + (index as u64).saturating_mul(bucket_ms);
                let ended_at = (started_at + bucket_ms).min(end_ms).saturating_sub(1);
                TimelineDensityPointDTO {
                    started_at,
                    ended_at,
                    note_count: 0,
                }
            })
            .collect()
    }

    fn get_timeline_notes_page_with_tag_ids(
        &self,
        selected_tag_ids: HashSet<Uuid>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        status: FilteredNoteStatus,
        group_sessions: bool,
        cursor: Option<&str>,
        direction: TimelineDirection,
        limit: Option<usize>,
    ) -> Result<TimelineNotesPageDTO, ServiceError> {
        let limit = limit.unwrap_or(20);
        self.with_read(|_tx, reader| {
            let cursor_uuid = Self::decode_timeline_cursor(cursor)?;
            let mut notes = Vec::with_capacity(limit.saturating_add(1));

            let note_refs = self.filtered_timeline_note_refs(
                reader,
                selected_tag_ids,
                include_untagged,
                tag_filter_enabled,
                status,
                cursor_uuid,
                direction,
            )?;

            for note_ref in note_refs {
                notes.push(self.note_ref_to_dto(note_ref?, reader)?);
                if notes.len() > limit {
                    break;
                }
            }

            let mut page = Self::finalize_note_page(notes, limit);
            if group_sessions {
                Self::apply_timeline_groups(&mut page.notes, direction)?;
            }
            Ok(page)
        })
    }

    /// 时间轴过滤的唯一入口。
    ///
    /// 兼容旧的“只向旧内容翻页”调用；新的分页 token 入口请走
    /// `get_timeline_notes_page`。
    #[deprecated(note = "use get_timeline_notes_page")]
    pub fn get_filtered_notes(
        &self,
        selected_tags: Vec<String>,
        include_untagged: bool,
        tag_filter_enabled: bool,
        status: FilteredNoteStatus,
        cursor: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<NoteDTO>, ServiceError> {
        #[allow(deprecated)]
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
