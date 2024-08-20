use std::io;

use bytes::Bytes;
use futures::{future::BoxFuture, stream::BoxStream};
use relative_path::{RelativePath, RelativePathBuf};
use url::Url;

use crate::{
    file::{File, Metadata},
    store::FileStore,
    FileInit,
};

pub trait DynamicFileStore {
    fn metadata<'a>(&'a self, path: &'a RelativePath)
        -> BoxFuture<'a, Result<Metadata, io::Error>>;
    fn open_file<'a>(&'a self, path: &'a RelativePath)
        -> BoxFuture<'a, Result<BoxFile, io::Error>>;

    fn rm_file<'a>(&'a self, path: &'a RelativePath) -> BoxFuture<'a, Result<(), io::Error>>;

    fn write_file<'a>(
        &'a self,
        path: &'a RelativePath,
        init: FileInit,
    ) -> BoxFuture<'a, Result<(), io::Error>>;

    fn list<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<BoxStream<'static, Result<RelativePathBuf, io::Error>>, io::Error>>;
}

pub trait DynamicFile {
    fn read_range<'a>(
        &'a self,
        range: std::ops::Range<u64>,
    ) -> BoxFuture<'a, Result<Bytes, io::Error>>;

    fn reader<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<BoxStream<'static, Result<Bytes, io::Error>>, io::Error>>;

    fn url(&self) -> Option<Url>;
}

pub fn filestore_box<T>(filestore: T) -> BoxFileStore
where
    T: FileStore + Send + Sync + 'static,
    T::File: Send + Sync + 'static,
    <T::File as File>::Body: Send + 'static,
{
    Box::new(DynamicFileStoreBox(filestore))
}

pub struct DynamicFileStoreBox<T>(T);

impl<T> DynamicFileStore for DynamicFileStoreBox<T>
where
    T: FileStore + Sync,
    T::File: Send + Sync + 'static,
    <T::File as File>::Body: Send + 'static,
{
    fn metadata<'a>(
        &'a self,
        path: &'a RelativePath,
    ) -> BoxFuture<'a, Result<Metadata, io::Error>> {
        Box::pin(self.0.metadata(path))
    }

    fn open_file<'a>(
        &'a self,
        path: &'a RelativePath,
    ) -> BoxFuture<'a, Result<BoxFile, io::Error>> {
        Box::pin(async move {
            let file = self.0.open_file(path).await?;
            Ok(Box::new(DynamicFileBox(file)) as BoxFile)
        })
    }

    fn rm_file<'a>(&'a self, path: &'a RelativePath) -> BoxFuture<'a, Result<(), io::Error>> {
        Box::pin(async move { self.0.rm_file(path).await })
    }

    fn write_file<'a>(
        &'a self,
        path: &'a RelativePath,
        init: FileInit,
    ) -> BoxFuture<'a, Result<(), io::Error>> {
        Box::pin(async move { self.0.write_file(path, init).await })
    }

    fn list<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<BoxStream<'static, Result<RelativePathBuf, io::Error>>, io::Error>>
    {
        Box::pin(self.0.list())
    }
}

pub struct DynamicFileBox<T>(T);

impl<T> DynamicFile for DynamicFileBox<T>
where
    T: File + Send + Sync,
    T::Body: Send + 'static,
{
    fn read_range<'a>(
        &'a self,
        range: std::ops::Range<u64>,
    ) -> BoxFuture<'a, Result<Bytes, io::Error>> {
        Box::pin(self.0.read_range(range))
    }

    fn reader<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<BoxStream<'static, Result<Bytes, io::Error>>, io::Error>> {
        Box::pin(async move {
            let body = self.0.reader().await?;
            Ok(Box::pin(body) as BoxStream<'static, Result<Bytes, io::Error>>)
        })
    }

    fn url(&self) -> Option<Url> {
        self.0.url()
    }
}

pub type BoxFileStore = Box<dyn DynamicFileStore + Send + Sync>;

pub type BoxFile = Box<dyn DynamicFile + Send + Sync>;

impl FileStore for BoxFileStore {
    type File = BoxFile;

    fn metadata(
        &self,
        path: &RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<Metadata, io::Error>> + Send {
        async move { (**self).metadata(path).await }
    }

    fn open_file(
        &self,
        path: &RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<Self::File, io::Error>> + Send {
        async move { (**self).open_file(path).await }
    }

    fn rm_file(
        &self,
        path: &RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<(), io::Error>> + Send {
        async move { (**self).rm_file(path).await }
    }

    fn write_file(
        &self,
        path: &RelativePath,
        init: FileInit,
    ) -> impl futures::prelude::Future<Output = Result<(), io::Error>> + Send {
        async move { (**self).write_file(path, init).await }
    }

    fn list(
        &self,
    ) -> impl futures::prelude::Future<
        Output = Result<BoxStream<'static, Result<RelativePathBuf, io::Error>>, io::Error>,
    > + Send {
        async move { (**self).list().await }
    }
}

impl File for BoxFile {
    type Body = BoxStream<'static, Result<Bytes, io::Error>>;

    fn read_range(
        &self,
        range: std::ops::Range<u64>,
    ) -> impl futures::prelude::Future<Output = Result<Bytes, io::Error>> + Send {
        async move { (**self).read_range(range).await }
    }

    fn reader(
        &self,
    ) -> impl futures::prelude::Future<Output = Result<Self::Body, io::Error>> + Send {
        async move { (**self).reader().await }
    }

    fn url(&self) -> Option<Url> {
        (**self).url()
    }
}
