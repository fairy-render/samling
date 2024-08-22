use std::{collections::HashMap, io, iter::Flatten, sync::Arc};

use bytes::Bytes;
use core::future::Future;
use futures::{stream::BoxStream, Stream, StreamExt, TryStreamExt};
use relative_path::{RelativePath, RelativePathBuf};

use crate::{
    boxed::{async_filestore_box, filestore_box, BoxFileStore},
    either::Either,
    file::{AsyncFile, Metadata},
    BoxAsyncFileStore, File,
};

#[non_exhaustive]
pub enum FileInit {
    Bytes(Bytes),
    Stream(BoxStream<'static, io::Result<Bytes>>),
    #[cfg(feature = "fs")]
    Path(std::path::PathBuf),
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
impl From<std::path::PathBuf> for FileInit {
    fn from(value: std::path::PathBuf) -> Self {
        FileInit::Path(value)
    }
}

impl From<BoxStream<'static, io::Result<Bytes>>> for FileInit {
    fn from(value: BoxStream<'static, io::Result<Bytes>>) -> Self {
        FileInit::Stream(value)
    }
}

pub trait AsyncFileStore {
    type File: AsyncFile;

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

impl<'a, T> AsyncFileStore for &'a T
where
    T: AsyncFileStore,
{
    type File = T::File;

    fn metadata(
        &self,
        path: &RelativePath,
    ) -> impl Future<Output = Result<Metadata, io::Error>> + Send {
        (*self).metadata(path)
    }

    fn open_file(
        &self,
        path: &RelativePath,
    ) -> impl Future<Output = Result<Self::File, io::Error>> + Send {
        (*self).open_file(path)
    }

    fn rm_file(&self, path: &RelativePath) -> impl Future<Output = Result<(), io::Error>> + Send {
        (*self).rm_file(path)
    }

    fn write_file(
        &self,
        path: &RelativePath,
        init: FileInit,
    ) -> impl Future<Output = Result<(), io::Error>> + Send {
        (*self).write_file(path, init)
    }

    fn list(
        &self,
    ) -> impl Future<
        Output = Result<BoxStream<'static, Result<RelativePathBuf, io::Error>>, io::Error>,
    > + Send {
        (*self).list()
    }
}

// Sync

pub trait FileStore {
    type File: File;
    type List: Iterator<Item = Result<RelativePathBuf, io::Error>>;

    fn metadata(&self, path: &RelativePath) -> Result<Metadata, io::Error>;

    fn open_file(&self, path: &RelativePath) -> Result<Self::File, io::Error>;

    fn rm_file(&self, path: &RelativePath) -> Result<(), io::Error>;

    fn write_file(&self, path: &RelativePath, init: FileInit) -> Result<(), io::Error>;

    fn list(&self) -> Self::List;

    fn exists(&self, path: &RelativePath) -> bool {
        self.metadata(path).is_ok()
    }
}

impl<'a, T> FileStore for &'a T
where
    T: FileStore + Send + Sync,
    T::File: Send,
{
    type File = T::File;

    type List = T::List;

    fn metadata(&self, path: &RelativePath) -> Result<Metadata, io::Error> {
        (*self).metadata(path)
    }

    fn open_file(&self, path: &RelativePath) -> Result<Self::File, io::Error> {
        (*self).open_file(path)
    }

    fn rm_file(&self, path: &RelativePath) -> Result<(), io::Error> {
        (*self).rm_file(path)
    }

    fn write_file(&self, path: &RelativePath, init: FileInit) -> Result<(), io::Error> {
        (*self).write_file(path, init)
    }

    fn list(&self) -> Self::List {
        (*self).list()
    }
}

pub trait FileStoreExt: FileStore {
    fn boxed(self) -> BoxFileStore
    where
        Self: Sized + Send + Sync + 'static,
        Self::List: Send + 'static,
        Self::File: Send + Sync + 'static,
        <Self::File as File>::Body: Send + 'static,
    {
        filestore_box(self)
    }
}

impl<T> FileStoreExt for T where T: FileStore {}

pub trait AsyncFileStoreExt: AsyncFileStore {
    fn boxed(self) -> BoxAsyncFileStore
    where
        Self: Sized + Send + Sync + 'static,
        Self::File: Send + Sync + 'static,
        <Self::File as AsyncFile>::Body: Send + 'static,
    {
        async_filestore_box(self)
    }
}

impl<T> AsyncFileStoreExt for T where T: AsyncFileStore {}

// Vec

impl<T> AsyncFileStore for Vec<T>
where
    T: AsyncFileStore + Send + Sync,
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

impl<T> FileStore for Vec<T>
where
    T: FileStore,
{
    type File = T::File;

    type List = Flatten<std::vec::IntoIter<T::List>>;

    fn metadata(&self, path: &RelativePath) -> Result<Metadata, io::Error> {
        for fs in self {
            if fs.exists(path) {
                return fs.metadata(path);
            }
        }
        Err(io::Error::new(io::ErrorKind::NotFound, ""))
    }

    fn open_file(&self, path: &RelativePath) -> Result<Self::File, io::Error> {
        for fs in self {
            if fs.exists(path) {
                return fs.open_file(path);
            }
        }
        Err(io::Error::new(io::ErrorKind::NotFound, ""))
    }

    fn rm_file(&self, path: &RelativePath) -> Result<(), io::Error> {
        for fs in self {
            if fs.exists(path) {
                return fs.rm_file(path);
            }
        }
        Err(io::Error::new(io::ErrorKind::NotFound, ""))
    }

    fn write_file(&self, path: &RelativePath, init: FileInit) -> Result<(), io::Error> {
        for fs in self {
            if fs.exists(path) {
                return fs.write_file(path, init);
            }
        }
        Err(io::Error::new(io::ErrorKind::NotFound, ""))
    }

    fn list(&self) -> Self::List {
        let list = self.iter().map(|m| m.list()).collect::<Vec<_>>();
        list.into_iter().flatten()
    }
}

// HashMap

fn find<'a, T>(
    routes: &'a HashMap<RelativePathBuf, Vec<T>>,
    name: &'a RelativePath,
) -> io::Result<(&'a Vec<T>, &'a RelativePath)>
where
    T: FileStore,
{
    let path = RelativePath::new(name);

    let Some(mut parent) = path.parent() else {
        return Err(io::ErrorKind::NotFound.into());
    };

    loop {
        if let Some(routes) = routes.get(parent) {
            let path = path.strip_prefix(parent.as_str()).expect("prefix");
            if routes.exists(path) {
                return Ok((routes, path));
            }
        }

        if let Some(p) = parent.parent() {
            parent = p;
        } else {
            break;
        }
    }

    Err(io::ErrorKind::NotFound.into())
}

async fn find_async<'a, T>(
    routes: &'a HashMap<RelativePathBuf, Vec<T>>,
    name: &'a RelativePath,
) -> io::Result<(&'a Vec<T>, &'a RelativePath)>
where
    T: AsyncFileStore + Send + Sync,
    T::File: Send,
{
    let path = RelativePath::new(name);

    let Some(mut parent) = path.parent() else {
        return Err(io::ErrorKind::NotFound.into());
    };

    loop {
        if let Some(routes) = routes.get(parent) {
            let path = path.strip_prefix(parent.as_str()).expect("prefix");
            if routes.exists(path).await {
                return Ok((routes, path));
            }
        }

        if let Some(p) = parent.parent() {
            parent = p;
        } else {
            break;
        }
    }

    Err(io::ErrorKind::NotFound.into())
}

impl<T> FileStore for HashMap<RelativePathBuf, Vec<T>>
where
    T: FileStore,
{
    type File = T::File;

    type List = Box<dyn Iterator<Item = io::Result<RelativePathBuf>> + Send>;

    fn metadata(&self, path: &RelativePath) -> Result<Metadata, io::Error> {
        let (fs, path) = find(&self, path)?;
        fs.metadata(path)
    }

    fn open_file(&self, path: &RelativePath) -> Result<Self::File, io::Error> {
        let (fs, path) = find(&self, path)?;
        fs.open_file(path)
    }

    fn rm_file(&self, path: &RelativePath) -> Result<(), io::Error> {
        let (fs, path) = find(&self, path)?;
        fs.rm_file(path)
    }

    fn write_file(&self, path: &RelativePath, init: FileInit) -> Result<(), io::Error> {
        let (fs, path) = find(&self, path)?;
        fs.write_file(path, init)
    }

    fn list(&self) -> Self::List {
        todo!()
    }
}

impl<T> AsyncFileStore for HashMap<RelativePathBuf, Vec<T>>
where
    T: AsyncFileStore + Send + Sync,
    T::File: Send,
{
    type File = T::File;

    fn metadata(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<crate::Metadata, std::io::Error>> + Send
    {
        async move {
            let (fs, path) = find_async(self, path).await?;
            fs.metadata(path).await
        }
    }

    fn open_file(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<Self::File, std::io::Error>> + Send {
        async move {
            let (fs, path) = find_async(self, path).await?;
            fs.open_file(path).await
        }
    }

    fn rm_file(
        &self,
        _path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<(), std::io::Error>> + Send {
        async move { Err(std::io::ErrorKind::PermissionDenied.into()) }
    }

    fn write_file(
        &self,
        _path: &relative_path::RelativePath,
        _init: crate::FileInit,
    ) -> impl futures::prelude::Future<Output = Result<(), std::io::Error>> + Send {
        async move { Err(std::io::ErrorKind::PermissionDenied.into()) }
    }

    fn list(
        &self,
    ) -> impl futures::prelude::Future<
        Output = Result<
            futures::prelude::stream::BoxStream<
                'static,
                Result<relative_path::RelativePathBuf, std::io::Error>,
            >,
            std::io::Error,
        >,
    > + Send {
        async move {
            let stream = self.iter().map(|(k, v)| async move {
                let k = k.clone();
                let mut stream = v.list().await?;
                let ret = async_stream::stream! {
                  let path = RelativePathBuf::from(k);
                  while let Some(next) = stream.next().await {
                    yield next.map(|next| path.join(next));
                  }

                };

                io::Result::Ok(ret)
            });

            let ret = futures::stream::iter(stream)
                .then(|m| m)
                .try_collect::<Vec<_>>()
                .await?;

            Ok(futures::stream::iter(ret).flatten().boxed())
        }
    }
}

impl<T> FileStore for Arc<T>
where
    T: FileStore,
{
    type File = T::File;

    type List = T::List;

    fn metadata(&self, path: &RelativePath) -> Result<Metadata, io::Error> {
        (**self).metadata(path)
    }

    fn open_file(&self, path: &RelativePath) -> Result<Self::File, io::Error> {
        (**self).open_file(path)
    }

    fn rm_file(&self, path: &RelativePath) -> Result<(), io::Error> {
        (**self).rm_file(path)
    }

    fn write_file(&self, path: &RelativePath, init: FileInit) -> Result<(), io::Error> {
        (**self).write_file(path, init)
    }

    fn list(&self) -> Self::List {
        (**self).list()
    }
}
