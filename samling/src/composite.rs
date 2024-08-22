use relative_path::{RelativePath, RelativePathBuf};
use std::{
    collections::HashMap,
    io::{self},
};

use crate::{
    boxed::{async_filestore_box, filestore_box, BoxFile, BoxFileStore},
    AsyncFile, AsyncFileStore, BoxAsyncFile, BoxAsyncFileStore, File, FileStore,
};

#[derive(Default)]
pub struct SyncComposite {
    routes: HashMap<RelativePathBuf, Vec<BoxFileStore>>,
}

impl SyncComposite {
    pub fn register<T>(&mut self, mut mount: &str, filestore: T)
    where
        T: FileStore + Send + Sync + 'static,
        T::File: Send + Sync + 'static,
        <T::File as File>::Body: Send,
        T::List: Send,
    {
        if mount.starts_with("/") {
            mount = &mount[1..];
        }

        self.routes
            .entry(mount.into())
            .or_default()
            .push(filestore_box(filestore));
    }
}

impl FileStore for SyncComposite {
    type File = BoxFile;

    type List = Box<dyn Iterator<Item = io::Result<RelativePathBuf>> + Send>;

    fn metadata(&self, path: &RelativePath) -> Result<crate::Metadata, io::Error> {
        self.routes.metadata(path)
    }

    fn open_file(&self, path: &RelativePath) -> Result<Self::File, io::Error> {
        self.routes.open_file(path)
    }

    fn rm_file(&self, path: &RelativePath) -> Result<(), io::Error> {
        self.routes.rm_file(path)
    }

    fn write_file(&self, path: &RelativePath, init: crate::FileInit) -> Result<(), io::Error> {
        self.routes.write_file(path, init)
    }

    fn list(&self) -> Self::List {
        self.routes.list()
    }
}

#[derive(Default)]
pub struct AsyncComposite {
    routes: HashMap<RelativePathBuf, Vec<BoxAsyncFileStore>>,
}

impl AsyncComposite {
    pub fn register<T>(&mut self, mut mount: &str, filestore: T)
    where
        T: AsyncFileStore + Send + Sync + 'static,
        T::File: Send + Sync + 'static,
        <T::File as AsyncFile>::Body: Send,
    {
        if mount.starts_with("/") {
            mount = &mount[1..];
        }

        self.routes
            .entry(mount.into())
            .or_default()
            .push(async_filestore_box(filestore));
    }
}

impl AsyncFileStore for AsyncComposite {
    type File = BoxAsyncFile;

    fn metadata(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<crate::Metadata, std::io::Error>> + Send
    {
        self.routes.metadata(path)
    }

    fn open_file(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<Self::File, std::io::Error>> + Send {
        self.routes.open_file(path)
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
        self.routes.list()
    }
}
