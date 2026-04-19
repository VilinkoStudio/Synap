use super::*;

impl SynapService {
    fn init_search(&self) -> Result<(), ServiceError> {
        self.note_searcher.clear();
        self.tag_searcher.clear();

        self.with_read(|tx, reader| {
            let timeline = TimelineView::new(reader);
            let mut notes = Vec::new();

            for note_ref_res in timeline.recent_refs()? {
                let note_ref = note_ref_res.map_err(ServiceError::from)?;
                if Self::is_latest_version(reader, note_ref)? {
                    let note = note_ref
                        .hydrate(reader)?
                        .ok_or(ServiceError::NotFound(note_ref.get_id().to_string()))?;
                    notes.push(note);
                }
            }

            self.note_searcher.insert_batch(notes.into_iter());

            let tag_reader = TagReader::new(tx)?;
            let tags = tag_reader
                .all()
                .map_err(redb::Error::from)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(redb::Error::from)?;
            self.tag_searcher.insert_batch(tags.into_iter());

            Ok(())
        })
    }

    pub(crate) fn refresh_search_indexes(&self) -> Result<(), ServiceError> {
        self.init_search()?;
        self.rebuild_tag_recommender()
    }

    /// 横向检索
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<NoteDTO>, ServiceError> {
        let search_res = self.note_searcher.search(query, limit, None);
        let uuids = search_res.items;
        self.with_read(|_tx, reader| {
            uuids
                .iter()
                .map(|id| reader.get_by_id(&id.id)?.ok_or(ServiceError::InvalidId))
                .map(|note| self.note_to_dto(note?, reader))
                .collect()
        })
    }

    pub fn search_tags(&self, query: &str, limit: usize) -> Result<Vec<String>, ServiceError> {
        let search_res = self.tag_searcher.search(query, limit, None);
        let ids = search_res.items;

        self.with_read(|tx, _reader| {
            let tag_reader = TagReader::new(tx)?;
            let mut seen = HashSet::new();

            ids.into_iter()
                .filter_map(|item| match tag_reader.get_by_id(&item.id) {
                    Ok(Some(tag)) => {
                        let content = tag.get_content().to_string();
                        seen.insert(content.clone()).then_some(Ok(content))
                    }
                    Ok(None) => None,
                    Err(err) => Some(Err(ServiceError::Db(err))),
                })
                .collect()
        })
    }

    pub fn recommend_tag(&self, content: &str, limit: usize) -> Result<Vec<String>, ServiceError> {
        Ok(self.tag_recommender.recommend_tag(content, limit))
    }

    pub fn get_all_tags(&self) -> Result<Vec<String>, ServiceError> {
        self.with_read(|tx, _reader| {
            let tag_reader = TagReader::new(tx)?;
            let mut tags = tag_reader
                .all()
                .map_err(redb::Error::from)?
                .map(|tag| tag.map(|tag| tag.get_content().to_string()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(redb::Error::from)?;

            tags.sort();
            Ok(tags)
        })
    }
}
