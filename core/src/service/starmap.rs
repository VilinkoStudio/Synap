use super::*;

impl SynapService {
    pub fn get_starmap(&self) -> Result<Vec<StarmapPointDTO>, ServiceError> {
        self.ensure_starmap_model_ready()?;
        self.with_read(|tx, reader| StarmapView::new(tx, reader).points())
    }

    pub fn rebuild_starmap_full(&self) -> Result<Vec<StarmapPointDTO>, ServiceError> {
        self.rebuild_starmap_full_cache()?;
        self.get_starmap()
    }
}
