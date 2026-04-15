use std::{
    collections::{BTreeMap, BTreeSet, HashSet, VecDeque},
    io::{self, ErrorKind},
    ops::Bound,
};

use redb::{ReadTransaction, WriteTransaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::{
        dagstorage::{DagReader, DagStore},
        kvstore::{KvReader, KvStore},
        onetomany::{OneToMany, OneToManyReader},
        setstore::{SetReader, SetStore},
        types::BlockId,
        vector::VectorStore,
    },
    error::NoteError,
    models::{
        tag::{Tag, TagReader, TagSyncRecord, TagWriter},
        util::random_id,
    },
    search::types::Searchable,
    text::sanitize_search_text,
};

mod entity;
mod reader;
mod record;

#[cfg(test)]
mod tests;

pub(crate) use entity::{Note, NoteRef};
pub(crate) use reader::NoteReader;
pub(crate) use record::{EditLinkRecord, NoteRecord, NoteVersionRecord, ReplyLinkRecord};

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct NoteBlock {
    pub content: String,
    pub short_id: [u8; 8],
    pub tags: Vec<Uuid>,
}

const NOTE_STORE: KvStore<BlockId, NoteBlock> = KvStore::new("NoteBlocks");
const ID_ALIAS: KvStore<[u8; 8], BlockId> = KvStore::new("IdAlias");
const NOTE_EDIT: DagStore = DagStore::new("NoteEditForward", "NoteEditRev");
const NOTE_LINK: DagStore = DagStore::new("NoteLinkForward", "NoteLinkRev");
const NOTE_DELETE: SetStore<BlockId> = SetStore::new("NoteDeleted");
const NOTE_TAG_INDEX: OneToMany<BlockId, BlockId> = OneToMany::new("TagToNotes");
const NOTE_VECTOR_INDEX: VectorStore<Vec<f32>> = VectorStore::new("NoteVectors", 384);

fn invalid_note_record(message: impl Into<String>) -> redb::Error {
    redb::Error::Io(io::Error::new(ErrorKind::InvalidData, message.into()))
}
