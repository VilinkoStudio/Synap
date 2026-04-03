use std::collections::VecDeque;

use crate::{
    error::NoteError,
    models::note::{NoteReader, NoteRef},
    views::note_view::NoteView,
};

#[derive(Debug, Clone)]
pub struct Session {
    notes: Vec<u64>,
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
        let raw_iter = self.reader.note_by_time()?.rev();

        let refs = raw_iter.filter_map(|res| match res {
            Ok(uuid) => match self.reader.is_deleted(&uuid) {
                Ok(true) => None,
                Ok(false) => Some(Ok(NoteRef::new(uuid, false))),
                Err(e) => Some(Err(NoteError::Db(e.into()))),
            },
            Err(e) => Some(Err(NoteError::Db(e.into()))),
        });

        Ok(refs)
    }

    pub fn detect_sessions(
        &self,
        multiplier: f64,
        window: usize,
        min_gap_ms: u64,
        max_gap_ms: u64,
    ) -> Result<impl Iterator<Item = Session> + '_, redb::Error> {
        let timestamps: Vec<u64> = self
            .reader
            .note_by_time()?
            .filter_map(|res| res.ok())
            .filter_map(|uuid| uuid.get_timestamp().map(|ts| ts.to_unix().0 * 1000))
            .collect();

        let sessions = detect_sessions_from_timestamps(
            timestamps.into_iter(),
            multiplier,
            window,
            min_gap_ms,
            max_gap_ms,
        );

        Ok(sessions)
    }
}

fn detect_sessions_from_timestamps(
    timestamps: impl Iterator<Item = u64>,
    multiplier: f64,
    window: usize,
    min_gap_ms: u64,
    max_gap_ms: u64,
) -> impl Iterator<Item = Session> {
    let mut recent_gaps: VecDeque<u64> = VecDeque::with_capacity(window);
    let mut current = Session { notes: Vec::new() };
    let mut prev_ts: Option<u64> = None;
    let mut sessions = Vec::new();

    for ts in timestamps {
        if let Some(prev) = prev_ts {
            let gap = ts.saturating_sub(prev);

            let threshold = if recent_gaps.len() >= window / 2 {
                let mut sorted_g: Vec<_> = recent_gaps.iter().copied().collect();
                sorted_g.sort();
                let median = sorted_g[sorted_g.len() / 2];
                let threshold = (median as f64 * multiplier) as u64;
                threshold.max(min_gap_ms).min(max_gap_ms)
            } else {
                min_gap_ms
            };

            if gap > threshold {
                if !current.notes.is_empty() {
                    sessions.push(current);
                    current = Session { notes: Vec::new() };
                }
            } else {
                recent_gaps.push_back(gap);
                if recent_gaps.len() > window {
                    recent_gaps.pop_front();
                }
            }
        }

        current.notes.push(ts);
        prev_ts = Some(ts);
    }

    if !current.notes.is_empty() {
        sessions.push(current);
    }

    sessions.into_iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts_ms(seconds: u64) -> u64 {
        seconds * 1000
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

        let sessions: Vec<_> = detect_sessions_from_timestamps(
            timestamps.into_iter(),
            3.0,
            8,
            5 * 60 * 1000,
            4 * 60 * 60 * 1000,
        )
        .collect();

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

        let sessions: Vec<_> = detect_sessions_from_timestamps(
            timestamps.into_iter(),
            3.0,
            8,
            5 * 60 * 1000,
            4 * 60 * 60 * 1000,
        )
        .collect();

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].count(), 3);
        assert_eq!(sessions[1].count(), 3);
    }

    #[test]
    fn test_detect_sessions_adaptive_threshold() {
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

        let sessions: Vec<_> = detect_sessions_from_timestamps(
            timestamps.into_iter(),
            3.0,
            8,
            5 * 60 * 1000,
            4 * 60 * 60 * 1000,
        )
        .collect();

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].count(), 5);
        assert_eq!(sessions[1].count(), 3);
    }

    #[test]
    fn test_detect_sessions_single_note() {
        let timestamps = vec![ts_ms(0)];

        let sessions: Vec<_> = detect_sessions_from_timestamps(
            timestamps.into_iter(),
            3.0,
            8,
            5 * 60 * 1000,
            4 * 60 * 60 * 1000,
        )
        .collect();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].count(), 1);
    }

    #[test]
    fn test_detect_sessions_empty() {
        let timestamps = vec![];

        let sessions: Vec<_> = detect_sessions_from_timestamps(
            timestamps.into_iter(),
            3.0,
            8,
            5 * 60 * 1000,
            4 * 60 * 60 * 1000,
        )
        .collect();

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
}
