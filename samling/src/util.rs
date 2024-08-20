use std::io;

use bytes::{Bytes, BytesMut};
use futures::{pin_mut, stream::BoxStream, StreamExt, TryStreamExt};
#[cfg(feature = "fs")]
use tokio_util::io::ReaderStream;

use crate::{File, FileStore, Path};

pub async fn copy<S, T>(source: Path<S>, target: Path<T>) -> io::Result<()>
where
    S: FileStore,
    T: FileStore,
{
    let file = source.open().await?;
    let meta = source.metadata().await?;

    let reader = file.reader().await?;
    pin_mut!(reader);

    let mut output = BytesMut::with_capacity(meta.size as usize);

    while let Some(next) = reader.try_next().await? {
        output.extend(next);
    }

    target.write(output.freeze()).await?;

    Ok(())
}

#[cfg(feature = "fs")]
pub async fn file_stream(path: &std::path::Path) -> io::Result<ReaderStream<tokio::fs::File>> {
    let mut opts = tokio::fs::OpenOptions::new();
    let file = opts.read(true).open(path).await?;
    Ok(ReaderStream::new(file))
}