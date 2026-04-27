use super::*;

#[derive(Debug, Clone)]
struct AggregatedSearchHit {
    note_id: Uuid,
    score: f32,
    sources: Vec<SearchSourceDTO>,
}

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
        })?;

        self.rebuild_note_embeddings()
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

    pub fn search_semantic(&self, query: &str, limit: usize) -> Result<Vec<NoteDTO>, ServiceError> {
        let uuids = self.with_read(|tx, _reader| {
            let results = self.semantic_index.search(tx, query, limit)?;
            Ok(results
                .into_iter()
                .map(|item| Uuid::from_bytes(item.note_id))
                .collect::<Vec<_>>())
        })?;

        self.with_read(|_tx, reader| {
            uuids
                .iter()
                .map(|id| reader.get_by_id(id)?.ok_or(ServiceError::InvalidId))
                .map(|note| self.note_to_dto(note?, reader))
                .collect()
        })
    }

    pub fn search_fusion(
        &self,
        query: &str,
        limit: usize,
        fuzzy_limit: Option<usize>,
        semantic_limit: Option<usize>,
    ) -> Result<Vec<SearchResultDTO>, ServiceError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let fuzzy_limit = fuzzy_limit.unwrap_or_else(|| self.note_searcher.total_items() as usize);
        let semantic_limit = semantic_limit.unwrap_or(limit);

        let fuzzy_results = self.note_searcher.search(query, fuzzy_limit, None);
        let semantic_results =
            self.with_read(|tx, _reader| self.semantic_index.search(tx, query, semantic_limit))?;

        let mut hits = HashMap::<Uuid, AggregatedSearchHit>::new();
        let fuzzy_len = fuzzy_results.items.len();

        for (index, item) in fuzzy_results.items.into_iter().enumerate() {
            let score = Self::rank_score(index, fuzzy_len);
            hits.entry(item.id)
                .and_modify(|hit| {
                    hit.score += score;
                    if !hit.sources.contains(&SearchSourceDTO::Fuzzy) {
                        hit.sources.push(SearchSourceDTO::Fuzzy);
                    }
                })
                .or_insert_with(|| AggregatedSearchHit {
                    note_id: item.id,
                    score,
                    sources: vec![SearchSourceDTO::Fuzzy],
                });
        }

        for item in semantic_results {
            let note_id = Uuid::from_bytes(item.note_id);
            hits.entry(note_id)
                .and_modify(|hit| {
                    hit.score += item.score;
                    if !hit.sources.contains(&SearchSourceDTO::Semantic) {
                        hit.sources.push(SearchSourceDTO::Semantic);
                    }
                })
                .or_insert_with(|| AggregatedSearchHit {
                    note_id,
                    score: item.score,
                    sources: vec![SearchSourceDTO::Semantic],
                });
        }

        let mut ranked_hits = hits.into_values().collect::<Vec<_>>();
        ranked_hits.sort_by(|a, b| b.score.total_cmp(&a.score));
        ranked_hits.truncate(limit);

        self.with_read(|_tx, reader| {
            ranked_hits
                .into_iter()
                .map(|hit| {
                    let note = reader
                        .get_by_id(&hit.note_id)?
                        .ok_or(ServiceError::InvalidId)?;
                    let note = self.note_to_dto(note, reader)?;

                    Ok(SearchResultDTO {
                        note,
                        score: hit.score,
                        sources: hit.sources,
                    })
                })
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

    fn rank_score(index: usize, total: usize) -> f32 {
        if total == 0 {
            0.0
        } else {
            (total - index) as f32 / total as f32
        }
    }
}
