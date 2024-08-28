use std::{io, sync::Arc};

use futures::stream::BoxStream;
use relative_path::{RelativePath, RelativePathBuf};

use crate::{
    boxed::{async_filestore_box, filestore_box, BoxFileStore},
    AsyncFile, AsyncFileInit, AsyncFileStore, BoxAsyncFile, BoxAsyncFileStore, FileInit, Metadata,
};

#[derive(Clone)]
pub struct AsyncFiles {
    store: Arc<BoxAsyncFileStore>,
}

impl AsyncFiles {
    pub fn new<T>(store: T) -> AsyncFiles
    where
        T: AsyncFileStore + Send + Sync + 'static,
        T::File: Send + Sync + 'static,
        <T::File as AsyncFile>::Body: Send + 'static,
    {
        AsyncFiles {
            store: Arc::new(async_filestore_box(store)),
        }
    }

    pub async fn metadata(&self, path: impl AsRef<RelativePath>) -> Result<Metadata, io::Error> {
        self.store.metadata(path.as_ref()).await
    }

    pub async fn open_file(
        &self,
        path: impl AsRef<RelativePath>,
    ) -> Result<BoxAsyncFile, io::Error> {
        self.store.open_file(path.as_ref()).await
    }

    pub async fn rm_file(&self, path: impl AsRef<RelativePath>) -> Result<(), io::Error> {
        self.store.rm_file(path.as_ref()).await
    }

    pub async fn write_file(
        &self,
        path: impl AsRef<RelativePath>,
        init: impl Into<AsyncFileInit>,
    ) -> Result<(), io::Error> {
        self.store.write_file(path.as_ref(), init.into()).await
    }

    pub async fn list(
        &self,
    ) -> Result<BoxStream<'static, Result<RelativePathBuf, io::Error>>, io::Error> {
        self.store.list().await
    }
}

// #[derive(Clone)]
// pub struct Files {
//     store: Arc<BoxFileStore>,
// }

// impl Files {
//     pub fn new<T>(store: T) -> Files
//     where
//         T: AsyncFileStore + Send + Sync + 'static,
//         T::File: Send + Sync + 'static,
//         <T::File as AsyncFile>::Body: Send + 'static,
//     {
//         Files {
//             store: Arc::new(async_filestore_box(store)),
//         }
//     }

//     pub async fn metadata(&self, path: impl AsRef<RelativePath>) -> Result<Metadata, io::Error> {
//         self.store.metadata(path.as_ref()).await
//     }

//     pub async fn open_file(
//         &self,
//         path: impl AsRef<RelativePath>,
//     ) -> Result<BoxAsyncFile, io::Error> {
//         self.store.open_file(path.as_ref()).await
//     }

//     pub async fn rm_file(&self, path: impl AsRef<RelativePath>) -> Result<(), io::Error> {
//         self.store.rm_file(path.as_ref()).await
//     }

//     pub async fn write_file(
//         &self,
//         path: impl AsRef<RelativePath>,
//         init: impl Into<FileInit>,
//     ) -> Result<(), io::Error> {
//         self.store.write_file(path.as_ref(), init.into()).await
//     }

//     pub async fn list(
//         &self,
//     ) -> Result<BoxStream<'static, Result<RelativePathBuf, io::Error>>, io::Error> {
//         self.store.list().await
//     }
// }
