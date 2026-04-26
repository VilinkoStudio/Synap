use super::*;

impl SynapService {
    pub fn get_starmap(&self) -> Result<Vec<StarmapPointDTO>, ServiceError> {
        self.with_read(|tx, reader| StarmapView::new(tx, reader).points())
    }
}
