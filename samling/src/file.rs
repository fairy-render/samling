use bytes::Bytes;
use futures::Stream;
use mime::Mime;
use relative_path::RelativePathBuf;
use std::future::Future;
use std::io;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub path: RelativePathBuf,
    pub size: u64,
    pub mime: Mime,
}

pub trait File {
    type Body: Stream<Item = Result<Bytes, io::Error>>;
    fn read_range(
        &self,
        range: std::ops::Range<u64>,
    ) -> impl Future<Output = Result<Bytes, io::Error>> + Send;

    fn reader(&self) -> impl Future<Output = Result<Self::Body, io::Error>> + Send;

    fn url(&self) -> Option<Url> {
        None
    }
}
