use super::*;

impl SynapService {
    pub fn create_note(&self, content: String, tags: Vec<String>) -> Result<NoteDTO, ServiceError> {
        let note = self.with_write(|tx| {
            let tags = self.materialize_tags(tx, tags)?;
            Note::create(tx, content, tags).map_err(Into::into)
        })?;

        self.note_searcher.insert(note.clone());
        self.refresh_tag_indexes()?;

        self.with_read(|_tx, reader| self.note_to_dto(note.clone(), reader))
    }

    pub fn reply_note(
        &self,
        parent_id: &str,
        content: String,
        tags: Vec<String>,
    ) -> Result<NoteDTO, ServiceError> {
        let parent_id = Self::parse_id(parent_id)?;
        let parent_ref = self.with_read(|_tx, reader| {
            Self::require_live_note_ref(reader, parent_id, &parent_id.to_string())
        })?;

        let child = self.with_write(|tx| {
            let tags = self.materialize_tags(tx, tags)?;
            let child = Note::create(tx, content, tags)?;
            parent_ref.reply_to(tx, &child.get_id())?;
            Ok(child)
        })?;

        self.note_searcher.insert(child.clone());
        self.refresh_tag_indexes()?;

        self.with_read(|_tx, reader| self.note_to_dto(child.clone(), reader))
    }

    /// 进化操作
    pub fn edit_note(
        &self,
        target_id: &str,
        new_content: String,
        tags: Vec<String>,
    ) -> Result<NoteDTO, ServiceError> {
        let target_id = Self::parse_id(target_id)?;
        let note_ref = self.with_read(|_tx, reader| {
            Self::require_live_note_ref(reader, target_id, &target_id.to_string())
        })?;

        let edited = self.with_write(|tx| {
            let tags = self.materialize_tags(tx, tags)?;
            note_ref.edit(tx, new_content, tags).map_err(Into::into)
        })?;

        self.refresh_search_indexes()?;

        self.with_read(|_tx, reader| self.note_to_dto(edited.clone(), reader))
    }

    /// 召唤死神
    pub fn delete_note(&self, target_id: &str) -> Result<(), ServiceError> {
        let uuid = Self::parse_id(target_id)?;
        let note_ref = self
            .with_read(|_tx, reader| reader.get_ref_by_id(&uuid)?.ok_or(ServiceError::InvalidId))?;
        self.with_write(|tx| {
            note_ref.del(tx)?;
            Ok(())
        })?;
        self.refresh_search_indexes()?;
        Ok(())
    }

    pub fn restore_note(&self, target_id: &str) -> Result<(), ServiceError> {
        let uuid = Self::parse_id(target_id)?;
        let note_ref = self
            .with_read(|_tx, reader| reader.get_ref_by_id(&uuid)?.ok_or(ServiceError::InvalidId))?;
        self.with_write(|tx| {
            note_ref.restore(tx)?;
            Ok(())
        })?;
        self.refresh_search_indexes()?;
        Ok(())
    }
}
