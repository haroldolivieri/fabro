mod codec;
mod record_id;
mod repository;
mod transaction;

pub(crate) use codec::{Codec, JsonCodec, MarkerCodec, RawBytesCodec};
pub(crate) use repository::Repository;
pub(crate) use transaction::transaction;

use crate::Result;

pub(crate) trait Record: Sized + Send + Sync + 'static {
    type Id: RecordId;
    type Codec: Codec<Self>;

    const PREFIX: &'static str;

    fn id(&self) -> Self::Id;
}

pub(crate) trait RecordId: Sized {
    fn key_segments(&self) -> Vec<String>;

    fn from_key_segments(segs: &[&str]) -> Result<Self>;
}
