use relative_path::RelativePathBuf;

use crate::{AsyncFileStore, FileStore};

pub struct Prefixed<T> {
    inner: T,
    mount: RelativePathBuf,
}

impl<T> Prefixed<T> {
    pub fn new(fs: T, mount: impl Into<RelativePathBuf>) -> Prefixed<T> {
        Prefixed {
            inner: fs,
            mount: mount.into(),
        }
    }
}

impl<T> FileStore for Prefixed<T>
where
    T: FileStore,
{
    type File = T::File;

    type List = T::List;

    fn metadata(
        &self,
        path: &relative_path::RelativePath,
    ) -> Result<crate::Metadata, std::io::Error> {
        self.inner.metadata(&self.mount.join(path))
    }

    fn open_file(&self, path: &relative_path::RelativePath) -> Result<Self::File, std::io::Error> {
        self.inner.open_file(&self.mount.join(path))
    }

    fn rm_file(&self, path: &relative_path::RelativePath) -> Result<(), std::io::Error> {
        self.inner.rm_file(&self.mount.join(path))
    }

    fn write_file(
        &self,
        path: &relative_path::RelativePath,
        init: crate::FileInit,
    ) -> Result<(), std::io::Error> {
        self.inner.write_file(&self.mount.join(path), init)
    }

    fn list(&self) -> Self::List {
        self.inner.list()
    }
}

impl<T> AsyncFileStore for Prefixed<T>
where
    T: AsyncFileStore + Send + Sync,
{
    type File = T::File;

    fn metadata(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<crate::Metadata, std::io::Error>> + Send
    {
        async move { self.inner.metadata(&self.mount.join(path)).await }
    }

    fn open_file(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<Self::File, std::io::Error>> + Send {
        async move { self.inner.open_file(&self.mount.join(path)).await }
    }

    fn rm_file(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<(), std::io::Error>> + Send {
        async move { self.inner.rm_file(&self.mount.join(path)).await }
    }

    fn write_file(
        &self,
        path: &relative_path::RelativePath,
        init: crate::FileInit,
    ) -> impl futures::prelude::Future<Output = Result<(), std::io::Error>> + Send {
        async move { self.inner.write_file(&self.mount.join(path), init).await }
    }

    fn list(
        &self,
    ) -> impl futures::prelude::Future<
        Output = Result<
            futures::prelude::stream::BoxStream<'static, Result<RelativePathBuf, std::io::Error>>,
            std::io::Error,
        >,
    > + Send {
        async move { self.inner.list().await }
    }
}
