use std::io;

use bytes::Bytes;
use core::future::Future;
use futures::{stream::BoxStream, Stream, StreamExt};
use relative_path::{RelativePath, RelativePathBuf};

use crate::{
    boxed::filestore_box,
    either::Either,
    file::{File, Metadata},
    BoxFileStore,
};

#[non_exhaustive]
pub enum FileInit {
    Bytes(Bytes),
    Stream(BoxStream<'static, io::Result<Bytes>>),
    #[cfg(feature = "fs")]
    Path(PathBuf),
}

impl FileInit {
    pub fn stream<T>(stream: T) -> FileInit
    where
        T: Stream<Item = io::Result<Bytes>> + Send + 'static,
    {
        FileInit::Stream(stream.boxed())
    }

    #[cfg(feature = "fs")]
    pub async fn into_stream(self) -> io::Result<impl Stream<Item = io::Result<Bytes>>> {
        let ret = match self {
            Self::Bytes(bs) => Either::Left(futures::stream::once(async move { Ok(bs) })),
            Self::Stream(bs) => Either::Right(Either::Left(bs)),

            Self::Path(path) => {
                Either::Right(Either::Right(crate::util::file_stream(&path).await?))
            }
        };

        Ok(ret)
    }

    #[cfg(not(feature = "fs"))]
    pub async fn into_stream(self) -> io::Result<impl Stream<Item = io::Result<Bytes>>> {
        let ret = match self {
            Self::Bytes(bs) => Either::Left(futures::stream::once(async move { Ok(bs) })),
            Self::Stream(bs) => Either::Right(bs),
        };

        Ok(ret)
    }
}

impl From<Bytes> for FileInit {
    fn from(value: Bytes) -> Self {
        FileInit::Bytes(value)
    }
}

impl From<Vec<u8>> for FileInit {
    fn from(value: Vec<u8>) -> Self {
        FileInit::Bytes(value.into())
    }
}

impl From<&'static [u8]> for FileInit {
    fn from(value: &'static [u8]) -> Self {
        FileInit::Bytes(Bytes::from_static(value))
    }
}

#[cfg(feature = "fs")]
impl From<PathBuf> for FileInit {
    fn from(value: PathBuf) -> Self {
        FileInit::Path(value)
    }
}

impl From<BoxStream<'static, io::Result<Bytes>>> for FileInit {
    fn from(value: BoxStream<'static, io::Result<Bytes>>) -> Self {
        FileInit::Stream(value)
    }
}

pub trait FileStore {
    type File: File;

    fn metadata(
        &self,
        path: &RelativePath,
    ) -> impl Future<Output = Result<Metadata, io::Error>> + Send;

    fn open_file(
        &self,
        path: &RelativePath,
    ) -> impl Future<Output = Result<Self::File, io::Error>> + Send;

    fn rm_file(&self, path: &RelativePath) -> impl Future<Output = Result<(), io::Error>> + Send;

    fn write_file(
        &self,
        path: &RelativePath,
        init: FileInit,
    ) -> impl Future<Output = Result<(), io::Error>> + Send;

    fn list(
        &self,
    ) -> impl Future<
        Output = Result<BoxStream<'static, Result<RelativePathBuf, io::Error>>, io::Error>,
    > + Send;

    fn exists(&self, path: &RelativePath) -> impl Future<Output = bool> + Send
    where
        Self: Sync,
    {
        async move { self.metadata(path).await.is_ok() }
    }
}

pub trait FileStoreExt: FileStore {
    fn boxed(self) -> BoxFileStore
    where
        Self: Sized + Send + Sync + 'static,
        Self::File: Send + Sync + 'static,
        <Self::File as File>::Body: Send + 'static,
    {
        filestore_box(self)
    }
}

impl<T> FileStoreExt for T where T: FileStore {}

// Vec

impl<T> FileStore for Vec<T>
where
    T: FileStore + Send + Sync,
    T::File: Send,
{
    type File = T::File;

    fn metadata(
        &self,
        path: &RelativePath,
    ) -> impl Future<Output = Result<Metadata, io::Error>> + Send {
        async move {
            for fs in self {
                if fs.exists(path).await {
                    return fs.metadata(path).await;
                }
            }
            Err(io::Error::new(io::ErrorKind::NotFound, ""))
        }
    }

    fn open_file(
        &self,
        path: &RelativePath,
    ) -> impl Future<Output = Result<Self::File, io::Error>> + Send {
        async move {
            for fs in self {
                if fs.exists(path).await {
                    return fs.open_file(path).await;
                }
            }
            Err(io::Error::new(io::ErrorKind::NotFound, ""))
        }
    }

    fn rm_file(&self, path: &RelativePath) -> impl Future<Output = Result<(), io::Error>> + Send {
        async move {
            for fs in self {
                if fs.exists(path).await {
                    return fs.rm_file(path).await;
                }
            }
            Err(io::Error::new(io::ErrorKind::NotFound, ""))
        }
    }

    fn write_file(
        &self,
        path: &RelativePath,
        init: FileInit,
    ) -> impl Future<Output = Result<(), io::Error>> + Send {
        async move {
            for fs in self {
                if fs.exists(path).await {
                    return fs.write_file(path, init).await;
                }
            }
            Err(io::Error::new(io::ErrorKind::NotFound, ""))
        }
    }

    fn list(
        &self,
    ) -> impl Future<
        Output = Result<BoxStream<'static, Result<RelativePathBuf, io::Error>>, io::Error>,
    > + Send {
        async move {
            let mut streams = Vec::default();

            for fs in self {
                streams.push(fs.list().await?);
            }

            Ok(futures::stream::iter(streams).flatten().boxed())
        }
    }
}
