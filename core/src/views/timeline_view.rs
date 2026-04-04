use std::ops::Bound;

use crate::{
    error::NoteError,
    models::note::{Note, NoteReader, NoteRef},
    views::note_view::NoteView,
};
use redb::{Database, ReadableDatabase};
use tempfile::NamedTempFile;
use uuid::{Builder, Uuid};

#[derive(Debug, Clone)]
pub struct Session {
    notes: Vec<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionDetectionConfig {
    pub split_gap_ms: u64,
}

impl SessionDetectionConfig {
    pub const fn new(split_gap_ms: u64) -> Self {
        Self { split_gap_ms }
    }
}

impl Default for SessionDetectionConfig {
    fn default() -> Self {
        Self::new(5 * 60 * 1000)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionSpan {
    oldest_id: Uuid,
    newest_id: Uuid,
    started_at: u64,
    ended_at: u64,
    count: usize,
}

pub type TimelineRefIter<'a> = Box<dyn Iterator<Item = Result<NoteRef, NoteError>> + 'a>;
pub type SessionSpanIter = Box<dyn Iterator<Item = SessionSpan>>;

pub struct TimelineSplit<'a> {
    pub newer: TimelineRefIter<'a>,
    pub older: TimelineRefIter<'a>,
}

pub struct SessionSpanSplit {
    pub current: Option<SessionSpan>,
    pub newer: SessionSpanIter,
    pub older: SessionSpanIter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelinePoint {
    NoteId(Uuid),
    TimestampMs(u64),
}

impl Session {
    pub fn start(&self) -> u64 {
        self.notes.first().copied().unwrap_or(0)
    }

    pub fn end(&self) -> u64 {
        self.notes.last().copied().unwrap_or(0)
    }

    pub fn count(&self) -> usize {
        self.notes.len()
    }

    pub fn duration(&self) -> u64 {
        self.end().saturating_sub(self.start())
    }
}

impl SessionSpan {
    pub fn cursor(&self) -> Uuid {
        self.oldest_id
    }

    pub fn oldest_id(&self) -> Uuid {
        self.oldest_id
    }

    pub fn newest_id(&self) -> Uuid {
        self.newest_id
    }

    pub fn started_at(&self) -> u64 {
        self.started_at
    }

    pub fn ended_at(&self) -> u64 {
        self.ended_at
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn duration(&self) -> u64 {
        self.ended_at.saturating_sub(self.started_at)
    }

    fn contains_point(&self, point: TimelinePoint) -> bool {
        match point {
            TimelinePoint::NoteId(id) => self.oldest_id <= id && id <= self.newest_id,
            TimelinePoint::TimestampMs(ms) => self.started_at <= ms && ms <= self.ended_at,
        }
    }

    fn is_before_point(&self, point: TimelinePoint) -> bool {
        match point {
            TimelinePoint::NoteId(id) => self.newest_id < id,
            TimelinePoint::TimestampMs(ms) => self.ended_at < ms,
        }
    }
}

pub struct TimelineView<'a> {
    reader: &'a NoteReader<'a>,
}

impl<'a> TimelineView<'a> {
    pub fn new(reader: &'a NoteReader<'a>) -> Self {
        Self { reader }
    }

    /// 获取全局的最新动态 (按时间倒序)
    pub fn recent(
        &self,
    ) -> Result<impl Iterator<Item = Result<NoteView<'_, 'a>, NoteError>> + '_, redb::Error> {
        let refs = self.recent_refs()?;

        let views = refs.filter_map(|note_ref_res| match note_ref_res {
            Ok(note_ref) => Some(NoteView::from_ref(self.reader, note_ref)),
            Err(e) => Some(Err(e)),
        });

        Ok(views)
    }

    pub fn recent_refs(
        &self,
    ) -> Result<impl Iterator<Item = Result<NoteRef, NoteError>> + '_, redb::Error> {
        Ok(self.live_refs(self.reader.note_by_time()?.rev()))
    }

    pub fn oldest_refs(
        &self,
    ) -> Result<impl Iterator<Item = Result<NoteRef, NoteError>> + '_, redb::Error> {
        Ok(self.live_refs(self.reader.note_by_time()?))
    }

    pub fn split_refs_from(
        &'a self,
        point: TimelinePoint,
    ) -> Result<TimelineSplit<'a>, redb::Error> {
        let older_iter = self
            .reader
            .note_by_time_range(Bound::Unbounded, point.older_end())?;
        let newer_iter = self
            .reader
            .note_by_time_range(point.newer_start(), Bound::Unbounded)?;

        Ok(TimelineSplit {
            newer: Box::new(self.live_refs(newer_iter)),
            older: Box::new(self.live_refs(older_iter.rev())),
        })
    }

    pub fn detect_sessions(
        &self,
        split_gap_ms: u64,
    ) -> Result<impl Iterator<Item = Session> + '_, redb::Error> {
        let timestamps: Vec<u64> = self
            .reader
            .note_by_time()?
            .filter_map(|res| res.ok())
            .filter_map(|uuid| uuid.get_timestamp().map(|ts| ts.to_unix().0 * 1000))
            .collect();

        let sessions = detect_sessions_from_timestamps(timestamps.into_iter(), split_gap_ms);

        Ok(sessions)
    }

    pub fn session_spans(
        &self,
        config: SessionDetectionConfig,
    ) -> Result<Vec<SessionSpan>, redb::Error> {
        let samples = self.visible_timeline_samples()?;
        Ok(detect_session_spans_from_samples(samples.into_iter(), config))
    }

    pub fn recent_session_spans(
        &self,
        config: SessionDetectionConfig,
    ) -> Result<impl Iterator<Item = SessionSpan> + '_, redb::Error> {
        let spans = self.session_spans(config)?;
        Ok(spans.into_iter().rev())
    }

    pub fn split_session_spans_from(
        &self,
        point: TimelinePoint,
        config: SessionDetectionConfig,
    ) -> Result<SessionSpanSplit, redb::Error> {
        let spans = self.session_spans(config)?;
        Ok(split_session_spans(spans, point))
    }

    pub fn refs_in_session(
        &'a self,
        session: &SessionSpan,
    ) -> Result<impl Iterator<Item = Result<NoteRef, NoteError>> + 'a, redb::Error> {
        let raw_iter = self
            .reader
            .note_by_time_range(
                Bound::Included(session.oldest_id()),
                Bound::Included(session.newest_id()),
            )?
            .rev();

        Ok(self.visible_refs(raw_iter))
    }

    fn live_refs<I>(&'a self, raw_iter: I) -> impl Iterator<Item = Result<NoteRef, NoteError>> + 'a
    where
        I: Iterator<Item = Result<Uuid, redb::StorageError>> + 'a,
    {
        raw_iter.filter_map(|res| match res {
            Ok(uuid) => match self.reader.is_deleted(&uuid) {
                Ok(true) => None,
                Ok(false) => Some(Ok(NoteRef::new(uuid, false))),
                Err(e) => Some(Err(NoteError::Db(e.into()))),
            },
            Err(e) => Some(Err(NoteError::Db(e.into()))),
        })
    }

    fn visible_refs<I>(
        &'a self,
        raw_iter: I,
    ) -> impl Iterator<Item = Result<NoteRef, NoteError>> + 'a
    where
        I: Iterator<Item = Result<Uuid, redb::StorageError>> + 'a,
    {
        raw_iter.filter_map(|res| match res {
            Ok(uuid) => match self.reader.is_deleted(&uuid) {
                Ok(true) => None,
                Ok(false) => match self.reader.has_next_version(&uuid) {
                    Ok(true) => None,
                    Ok(false) => Some(Ok(NoteRef::new(uuid, false))),
                    Err(e) => Some(Err(NoteError::Db(e.into()))),
                },
                Err(e) => Some(Err(NoteError::Db(e.into()))),
            },
            Err(e) => Some(Err(NoteError::Db(e.into()))),
        })
    }

    fn visible_timeline_samples(&self) -> Result<Vec<TimelineSample>, redb::Error> {
        let mut samples = Vec::new();

        for uuid_res in self.reader.note_by_time()? {
            let uuid = uuid_res?;
            if self.reader.is_deleted(&uuid)? {
                continue;
            }
            if self
                .reader
                .has_next_version(&uuid)
                .map_err(redb::Error::from)?
            {
                continue;
            }

            let Some(timestamp_ms) = uuid_timestamp_ms(uuid) else {
                continue;
            };

            samples.push(TimelineSample { id: uuid, timestamp_ms });
        }

        Ok(samples)
    }
}

impl TimelinePoint {
    fn newer_start(self) -> Bound<Uuid> {
        match self {
            TimelinePoint::NoteId(id) => Bound::Excluded(id),
            TimelinePoint::TimestampMs(ms) => Bound::Excluded(uuid_for_timestamp_ms_max(ms)),
        }
    }

    fn older_end(self) -> Bound<Uuid> {
        match self {
            TimelinePoint::NoteId(id) => Bound::Excluded(id),
            TimelinePoint::TimestampMs(ms) => Bound::Included(uuid_for_timestamp_ms_max(ms)),
        }
    }
}

fn uuid_for_timestamp_ms_min(timestamp_ms: u64) -> Uuid {
    Builder::from_unix_timestamp_millis(timestamp_ms, &[0; 10]).into_uuid()
}

fn uuid_for_timestamp_ms_max(timestamp_ms: u64) -> Uuid {
    Builder::from_unix_timestamp_millis(timestamp_ms, &[0xFF; 10]).into_uuid()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimelineSample {
    id: Uuid,
    timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct SessionAccumulator {
    oldest_id: Uuid,
    newest_id: Uuid,
    started_at: u64,
    ended_at: u64,
    count: usize,
}

impl SessionAccumulator {
    fn new(sample: TimelineSample) -> Self {
        Self {
            oldest_id: sample.id,
            newest_id: sample.id,
            started_at: sample.timestamp_ms,
            ended_at: sample.timestamp_ms,
            count: 1,
        }
    }

    fn push(&mut self, sample: TimelineSample) {
        self.newest_id = sample.id;
        self.ended_at = sample.timestamp_ms;
        self.count += 1;
    }

    fn finish(self) -> SessionSpan {
        SessionSpan {
            oldest_id: self.oldest_id,
            newest_id: self.newest_id,
            started_at: self.started_at,
            ended_at: self.ended_at,
            count: self.count,
        }
    }
}

fn uuid_timestamp_ms(uuid: Uuid) -> Option<u64> {
    uuid.get_timestamp().map(|ts| {
        let (seconds, nanos) = ts.to_unix();
        seconds * 1000 + nanos as u64 / 1_000_000
    })
}

fn detect_session_spans_from_samples(
    samples: impl Iterator<Item = TimelineSample>,
    config: SessionDetectionConfig,
) -> Vec<SessionSpan> {
    let mut samples = samples.into_iter();
    let mut sessions = Vec::new();
    let Some(first) = samples.next() else {
        return sessions;
    };

    let mut current = SessionAccumulator::new(first);
    let mut prev_ts = first.timestamp_ms;

    for sample in samples {
        let gap = sample.timestamp_ms.saturating_sub(prev_ts);

        if gap > config.split_gap_ms {
            sessions.push(current.finish());
            current = SessionAccumulator::new(sample);
        } else {
            current.push(sample);
        }

        prev_ts = sample.timestamp_ms;
    }

    sessions.push(current.finish());
    sessions
}

fn detect_sessions_from_timestamps(
    timestamps: impl Iterator<Item = u64>,
    split_gap_ms: u64,
) -> impl Iterator<Item = Session> {
    let mut sessions = Vec::new();
    let mut timestamps = timestamps.into_iter();
    let Some(first) = timestamps.next() else {
        return sessions.into_iter();
    };

    let mut current = Session { notes: vec![first] };
    let mut prev_ts = first;

    for ts in timestamps {
        let gap = ts.saturating_sub(prev_ts);
        if gap > split_gap_ms {
            sessions.push(current);
            current = Session { notes: Vec::new() };
        }

        current.notes.push(ts);
        prev_ts = ts;
    }

    sessions.push(current);
    sessions.into_iter()
}

fn split_session_spans(spans: Vec<SessionSpan>, point: TimelinePoint) -> SessionSpanSplit {
    let current_index = spans.iter().position(|session| session.contains_point(point));

    let (current, older, newer) = match current_index {
        Some(index) => (
            Some(spans[index]),
            spans[..index].iter().rev().copied().collect::<Vec<_>>(),
            spans[index + 1..].to_vec(),
        ),
        None => {
            let pivot = spans.partition_point(|session| session.is_before_point(point));
            (
                None,
                spans[..pivot].iter().rev().copied().collect::<Vec<_>>(),
                spans[pivot..].to_vec(),
            )
        }
    };

    SessionSpanSplit {
        current,
        newer: Box::new(newer.into_iter()),
        older: Box::new(older.into_iter()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::note::NoteSyncRecord;
    use std::{thread::sleep, time::Duration};

    fn ts_ms(seconds: u64) -> u64 {
        seconds * 1000
    }

    fn temp_db() -> Database {
        let f = NamedTempFile::new().unwrap();
        Database::create(f.path()).unwrap()
    }

    fn create_note_sequence() -> (Database, Vec<Uuid>) {
        let db = temp_db();
        let wtx = db.begin_write().unwrap();
        Note::init_schema(&wtx).unwrap();

        let first = Note::create(&wtx, "first".to_string(), vec![]).unwrap();
        sleep(Duration::from_millis(2));
        let second = Note::create(&wtx, "second".to_string(), vec![]).unwrap();
        sleep(Duration::from_millis(2));
        let third = Note::create(&wtx, "third".to_string(), vec![]).unwrap();
        sleep(Duration::from_millis(2));
        let fourth = Note::create(&wtx, "fourth".to_string(), vec![]).unwrap();
        wtx.commit().unwrap();

        (
            db,
            vec![
                first.get_id(),
                second.get_id(),
                third.get_id(),
                fourth.get_id(),
            ],
        )
    }

    fn import_note_at(wtx: &redb::WriteTransaction, timestamp_ms: u64, seed: u8) -> Note {
        let mut random = [seed; 10];
        random[9] = seed.wrapping_mul(7).wrapping_add(1);

        Note::import(
            wtx,
            NoteSyncRecord {
                id: Builder::from_unix_timestamp_millis(timestamp_ms, &random).into_uuid(),
                content: format!("note-{seed}"),
                short_id: [seed; 8],
                tags: vec![],
            },
        )
        .unwrap()
    }

    fn create_spaced_note_sequence(timestamps_ms: &[u64]) -> (Database, Vec<Uuid>) {
        let db = temp_db();
        let wtx = db.begin_write().unwrap();
        Note::init_schema(&wtx).unwrap();

        let ids = timestamps_ms
            .iter()
            .enumerate()
            .map(|(idx, timestamp_ms)| import_note_at(&wtx, *timestamp_ms, idx as u8 + 1).get_id())
            .collect();

        wtx.commit().unwrap();
        (db, ids)
    }

    #[test]
    fn test_detect_sessions_basic() {
        let timestamps = vec![
            ts_ms(0),
            ts_ms(120),
            ts_ms(240),
            ts_ms(360),
            ts_ms(500),
            ts_ms(620),
            ts_ms(740),
            ts_ms(860),
        ];

        let sessions: Vec<_> =
            detect_sessions_from_timestamps(timestamps.into_iter(), 5 * 60 * 1000).collect();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].count(), 8);
    }

    #[test]
    fn test_detect_sessions_with_big_gap() {
        let timestamps = vec![
            ts_ms(0),
            ts_ms(120),
            ts_ms(240),
            ts_ms(10000),
            ts_ms(10120),
            ts_ms(10240),
        ];

        let sessions: Vec<_> =
            detect_sessions_from_timestamps(timestamps.into_iter(), 5 * 60 * 1000).collect();

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].count(), 3);
        assert_eq!(sessions[1].count(), 3);
    }

    #[test]
    fn test_detect_sessions_with_fixed_global_gap() {
        let timestamps = vec![
            ts_ms(0),
            ts_ms(60),
            ts_ms(120),
            ts_ms(180),
            ts_ms(240),
            ts_ms(1200),
            ts_ms(1260),
            ts_ms(1320),
        ];

        let sessions: Vec<_> =
            detect_sessions_from_timestamps(timestamps.into_iter(), 5 * 60 * 1000).collect();

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].count(), 5);
        assert_eq!(sessions[1].count(), 3);
    }

    #[test]
    fn test_detect_sessions_single_note() {
        let timestamps = vec![ts_ms(0)];

        let sessions: Vec<_> =
            detect_sessions_from_timestamps(timestamps.into_iter(), 5 * 60 * 1000).collect();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].count(), 1);
    }

    #[test]
    fn test_detect_sessions_empty() {
        let timestamps = vec![];

        let sessions: Vec<_> =
            detect_sessions_from_timestamps(timestamps.into_iter(), 5 * 60 * 1000).collect();

        assert!(sessions.is_empty());
    }

    #[test]
    fn test_session_properties() {
        let session = Session {
            notes: vec![ts_ms(0), ts_ms(600), ts_ms(1200)],
        };

        assert_eq!(session.start(), ts_ms(0));
        assert_eq!(session.end(), ts_ms(1200));
        assert_eq!(session.count(), 3);
        assert_eq!(session.duration(), ts_ms(1200));
    }

    #[test]
    fn test_detect_session_spans_from_samples_basic() {
        let samples = vec![
            TimelineSample {
                id: Uuid::from_u128(1),
                timestamp_ms: ts_ms(0),
            },
            TimelineSample {
                id: Uuid::from_u128(2),
                timestamp_ms: ts_ms(120),
            },
            TimelineSample {
                id: Uuid::from_u128(3),
                timestamp_ms: ts_ms(240),
            },
        ];

        let sessions = detect_session_spans_from_samples(
            samples.into_iter(),
            SessionDetectionConfig::default(),
        );

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].oldest_id(), Uuid::from_u128(1));
        assert_eq!(sessions[0].newest_id(), Uuid::from_u128(3));
        assert_eq!(sessions[0].count(), 3);
    }

    #[test]
    fn test_detect_session_spans_from_samples_with_gap() {
        let samples = vec![
            TimelineSample {
                id: Uuid::from_u128(1),
                timestamp_ms: ts_ms(0),
            },
            TimelineSample {
                id: Uuid::from_u128(2),
                timestamp_ms: ts_ms(60),
            },
            TimelineSample {
                id: Uuid::from_u128(3),
                timestamp_ms: ts_ms(10_000),
            },
            TimelineSample {
                id: Uuid::from_u128(4),
                timestamp_ms: ts_ms(10_060),
            },
        ];

        let sessions = detect_session_spans_from_samples(
            samples.into_iter(),
            SessionDetectionConfig::default(),
        );

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].count(), 2);
        assert_eq!(sessions[1].count(), 2);
        assert_eq!(sessions[0].oldest_id(), Uuid::from_u128(1));
        assert_eq!(sessions[1].oldest_id(), Uuid::from_u128(3));
    }

    #[test]
    fn test_split_session_spans_from_note_id_returns_current_and_both_directions() {
        let (db, ids) = create_spaced_note_sequence(&[
            ts_ms(0),
            ts_ms(60),
            ts_ms(600),
            ts_ms(660),
            ts_ms(1500),
        ]);

        let rtx = db.begin_read().unwrap();
        let reader = NoteReader::new(&rtx).unwrap();
        let timeline = TimelineView::new(&reader);

        let split = timeline
            .split_session_spans_from(
                TimelinePoint::NoteId(ids[2]),
                SessionDetectionConfig::default(),
            )
            .unwrap();

        let current = split.current.unwrap();
        let older: Vec<_> = split.older.collect();
        let newer: Vec<_> = split.newer.collect();

        assert_eq!(current.oldest_id(), ids[2]);
        assert_eq!(current.newest_id(), ids[3]);
        assert_eq!(current.count(), 2);
        assert_eq!(older.len(), 1);
        assert_eq!(older[0].oldest_id(), ids[0]);
        assert_eq!(older[0].newest_id(), ids[1]);
        assert_eq!(newer.len(), 1);
        assert_eq!(newer[0].oldest_id(), ids[4]);
    }

    #[test]
    fn test_split_session_spans_from_timestamp_between_sessions_has_no_current() {
        let (db, ids) = create_spaced_note_sequence(&[
            ts_ms(0),
            ts_ms(60),
            ts_ms(600),
            ts_ms(660),
            ts_ms(1500),
        ]);

        let rtx = db.begin_read().unwrap();
        let reader = NoteReader::new(&rtx).unwrap();
        let timeline = TimelineView::new(&reader);

        let split = timeline
            .split_session_spans_from(
                TimelinePoint::TimestampMs(ts_ms(300)),
                SessionDetectionConfig::default(),
            )
            .unwrap();

        let older: Vec<_> = split.older.collect();
        let newer: Vec<_> = split.newer.collect();

        assert!(split.current.is_none());
        assert_eq!(older.len(), 1);
        assert_eq!(older[0].oldest_id(), ids[0]);
        assert_eq!(older[0].newest_id(), ids[1]);
        assert_eq!(newer.len(), 2);
        assert_eq!(newer[0].oldest_id(), ids[2]);
        assert_eq!(newer[0].newest_id(), ids[3]);
        assert_eq!(newer[1].oldest_id(), ids[4]);
    }

    #[test]
    fn test_split_refs_from_note_id_returns_two_directions() {
        let (db, ids) = create_note_sequence();

        let rtx = db.begin_read().unwrap();
        let reader = NoteReader::new(&rtx).unwrap();
        let timeline = TimelineView::new(&reader);

        let split = timeline
            .split_refs_from(TimelinePoint::NoteId(ids[2]))
            .unwrap();

        let newer: Vec<_> = split.newer.map(|res| res.unwrap().get_id()).collect();
        let older: Vec<_> = split.older.map(|res| res.unwrap().get_id()).collect();

        assert_eq!(newer, vec![ids[3]]);
        assert_eq!(older, vec![ids[1], ids[0]]);
    }

    #[test]
    fn test_split_refs_from_timestamp_partitions_around_time() {
        let (db, ids) = create_note_sequence();
        let timestamp = ids[1].get_timestamp().unwrap();
        let (seconds, nanos) = timestamp.to_unix();
        let timestamp_ms = seconds * 1000 + nanos as u64 / 1_000_000;

        let rtx = db.begin_read().unwrap();
        let reader = NoteReader::new(&rtx).unwrap();
        let timeline = TimelineView::new(&reader);

        let split = timeline
            .split_refs_from(TimelinePoint::TimestampMs(timestamp_ms))
            .unwrap();

        let newer: Vec<_> = split.newer.map(|res| res.unwrap().get_id()).collect();
        let older: Vec<_> = split.older.map(|res| res.unwrap().get_id()).collect();

        assert_eq!(newer, vec![ids[2], ids[3]]);
        assert_eq!(older, vec![ids[1], ids[0]]);
    }

    #[test]
    fn test_split_refs_from_skips_deleted_notes() {
        let db = temp_db();
        let wtx = db.begin_write().unwrap();
        Note::init_schema(&wtx).unwrap();

        let first = Note::create(&wtx, "first".to_string(), vec![]).unwrap();
        sleep(Duration::from_millis(2));
        let second = Note::create(&wtx, "second".to_string(), vec![]).unwrap();
        sleep(Duration::from_millis(2));
        let third = Note::create(&wtx, "third".to_string(), vec![]).unwrap();
        sleep(Duration::from_millis(2));
        let fourth = Note::create(&wtx, "fourth".to_string(), vec![]).unwrap();
        second.del(&wtx).unwrap();
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = NoteReader::new(&rtx).unwrap();
        let timeline = TimelineView::new(&reader);

        let split = timeline
            .split_refs_from(TimelinePoint::NoteId(third.get_id()))
            .unwrap();

        let newer: Vec<_> = split.newer.map(|res| res.unwrap().get_id()).collect();
        let older: Vec<_> = split.older.map(|res| res.unwrap().get_id()).collect();

        assert_eq!(newer, vec![fourth.get_id()]);
        assert_eq!(older, vec![first.get_id()]);
    }

    #[test]
    fn test_timestamp_uuid_bounds_cover_same_millisecond() {
        let timestamp_ms = 1_742_165_200_000;
        let min = uuid_for_timestamp_ms_min(timestamp_ms);
        let max = uuid_for_timestamp_ms_max(timestamp_ms);

        assert!(min <= max);
        assert_eq!(
            min.get_timestamp().unwrap().to_unix().0 * 1000,
            timestamp_ms
        );
        assert_eq!(
            max.get_timestamp().unwrap().to_unix().0 * 1000,
            timestamp_ms
        );
    }

    #[test]
    fn test_recent_session_spans_uses_visible_notes_only() {
        let db = temp_db();
        let wtx = db.begin_write().unwrap();
        Note::init_schema(&wtx).unwrap();

        let original = Note::create(&wtx, "first".to_string(), vec![]).unwrap();
        sleep(Duration::from_millis(2));
        let edited = original
            .edit(&wtx, "first edited".to_string(), vec![])
            .unwrap();
        sleep(Duration::from_millis(2));
        let deleted = Note::create(&wtx, "deleted".to_string(), vec![]).unwrap();
        deleted.del(&wtx).unwrap();
        sleep(Duration::from_millis(2));
        let live = Note::create(&wtx, "live".to_string(), vec![]).unwrap();
        wtx.commit().unwrap();

        let rtx = db.begin_read().unwrap();
        let reader = NoteReader::new(&rtx).unwrap();
        let timeline = TimelineView::new(&reader);

        let sessions: Vec<_> = timeline
            .recent_session_spans(SessionDetectionConfig::default())
            .unwrap()
            .collect();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].count(), 2);

        let session_note_ids: Vec<_> = timeline
            .refs_in_session(&sessions[0])
            .unwrap()
            .map(|res| res.unwrap().get_id())
            .collect();

        assert_eq!(session_note_ids, vec![live.get_id(), edited.get_id()]);
    }
}
