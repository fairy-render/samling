use std::io;

use relative_path::{RelativePath, RelativePathBuf};

use crate::{FileInit, FileStore, Metadata};

#[derive(Debug, Clone)]
pub struct Path<T> {
    store: T,
    path: RelativePathBuf,
}

impl<T> core::ops::Deref for Path<T> {
    type Target = RelativePath;
    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl<T> Path<T>
where
    T: FileStore,
{
    pub fn new(store: T, path: RelativePathBuf) -> Path<T> {
        Path { store, path }
    }

    pub async fn metadata(&self) -> io::Result<Metadata> {
        self.store.metadata(&self.path).await
    }

    pub async fn open(&self) -> io::Result<T::File> {
        self.store.open_file(&self.path).await
    }

    pub async fn write(&self, body: impl Into<FileInit>) -> io::Result<()> {
        self.store.write_file(&self.path, body.into()).await
    }
}
