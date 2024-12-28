use std::io;

use bytes::{Bytes, BytesMut};
use futures::{pin_mut, TryStreamExt};
#[cfg(feature = "fs")]
use tokio_util::io::ReaderStream;

use crate::{AsyncFile, AsyncFileStore, Path};

pub async fn copy<S, T>(source: Path<S>, target: Path<T>) -> io::Result<()>
where
    S: AsyncFileStore,
    T: AsyncFileStore,
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

pub async fn read<T: AsyncFile>(file: &mut T) -> io::Result<Bytes> {
    let mut output = BytesMut::new();
    let reader = file.reader().await?;
    pin_mut!(reader);

    while let Some(next) = reader.try_next().await? {
        output.extend(next);
    }

    Ok(output.freeze())
}
