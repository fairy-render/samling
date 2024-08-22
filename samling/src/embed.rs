use std::{
    io::{self, Cursor},
    marker::PhantomData,
};

use bytes::Bytes;
use futures::StreamExt;
use relative_path::RelativePathBuf;

use crate::{AsyncFile, AsyncFileStore, File, FileStore, Metadata};

pub struct Embed<T>(PhantomData<T>);

impl<T> Clone for Embed<T> {
    fn clone(&self) -> Self {
        Embed(PhantomData)
    }
}

impl<T> Embed<T> {
    pub fn new() -> Embed<T> {
        Embed(PhantomData)
    }
}

#[cfg(debug_assertions)]
impl<T> AsyncFileStore for Embed<T>
where
    T: rust_embed::RustEmbed + Send + Sync + 'static,
{
    type File = EmbedFile<T>;

    fn metadata(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<crate::Metadata, std::io::Error>> + Send
    {
        async move {
            let this = self.clone();
            let path = path.to_relative_path_buf();

            tokio::task::spawn_blocking(move || <Embed<T> as FileStore>::metadata(&this, &path))
                .await
                .expect("spawn")
        }
    }

    fn open_file(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<Self::File, std::io::Error>> + Send {
        async move {
            let this = self.clone();
            let path = path.to_relative_path_buf();

            tokio::task::spawn_blocking(move || <Embed<T> as FileStore>::open_file(&this, &path))
                .await
                .expect("spawn")
        }
    }

    fn rm_file(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<(), std::io::Error>> + Send {
        async move {
            let this = self.clone();
            let path = path.to_relative_path_buf();

            tokio::task::spawn_blocking(move || <Embed<T> as FileStore>::rm_file(&this, &path))
                .await
                .expect("spawn")
        }
    }

    fn write_file(
        &self,
        path: &relative_path::RelativePath,
        init: crate::FileInit,
    ) -> impl futures::prelude::Future<Output = Result<(), std::io::Error>> + Send {
        async move {
            let this = self.clone();
            let path = path.to_relative_path_buf();

            tokio::task::spawn_blocking(move || {
                <Embed<T> as FileStore>::write_file(&this, &path, init)
            })
            .await
            .expect("spawn")
        }
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
            let stream = futures::stream::iter(
                T::iter()
                    .map(|m| Ok(RelativePathBuf::from(m.as_ref())))
                    .collect::<Vec<_>>(),
            );
            Ok(stream.boxed())
        }
    }
}

#[cfg(not(debug_assertions))]
impl<T> AsyncFileStore for Embed<T>
where
    T: rust_embed::RustEmbed + Send + Sync,
{
    type File = EmbedFile<T>;

    fn metadata(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<crate::Metadata, std::io::Error>> + Send
    {
        async move {
            let Some(found) = T::get(path.as_str()) else {
                todo!()
            };

            let meta = Metadata {
                size: found.data.len() as u64,
                mime: if let Some(ext) = path.extension() {
                    mime_guess::from_ext(ext).first_or_octet_stream()
                } else {
                    mime::APPLICATION_OCTET_STREAM
                },
                path: path.to_relative_path_buf(),
            };

            Ok(meta)
        }
    }

    fn open_file(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<Self::File, std::io::Error>> + Send {
        async move {
            T::get(path.as_str())
                .map(|m| EmbedFile(m, PhantomData))
                .ok_or_else(|| io::ErrorKind::NotFound.into())
        }
    }

    fn rm_file(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<(), std::io::Error>> + Send {
        async move { Err(io::ErrorKind::PermissionDenied.into()) }
    }

    fn write_file(
        &self,
        path: &relative_path::RelativePath,
        init: crate::FileInit,
    ) -> impl futures::prelude::Future<Output = Result<(), std::io::Error>> + Send {
        async move { Err(io::ErrorKind::PermissionDenied.into()) }
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
            let stream = futures::stream::iter(
                T::iter()
                    .map(|m| Ok(RelativePathBuf::from(m.as_ref())))
                    .collect::<Vec<_>>(),
            );
            Ok(stream.boxed())
        }
    }
}

impl<T> FileStore for Embed<T>
where
    T: rust_embed::RustEmbed + Send + Sync,
{
    type File = EmbedFile<T>;

    type List = Box<dyn Iterator<Item = io::Result<RelativePathBuf>> + Send>;

    fn metadata(&self, path: &relative_path::RelativePath) -> Result<Metadata, io::Error> {
        let Some(found) = T::get(path.as_str()) else {
            todo!()
        };

        let meta = Metadata {
            size: found.data.len() as u64,
            mime: if let Some(ext) = path.extension() {
                mime_guess::from_ext(ext).first_or_octet_stream()
            } else {
                mime::APPLICATION_OCTET_STREAM
            },
            path: path.to_relative_path_buf(),
        };

        Ok(meta)
    }

    fn open_file(&self, path: &relative_path::RelativePath) -> Result<Self::File, io::Error> {
        T::get(path.as_str())
            .map(|m| EmbedFile(m, PhantomData))
            .ok_or_else(|| io::ErrorKind::NotFound.into())
    }

    fn rm_file(&self, _path: &relative_path::RelativePath) -> Result<(), io::Error> {
        Err(io::ErrorKind::PermissionDenied.into())
    }

    fn write_file(
        &self,
        _path: &relative_path::RelativePath,
        _init: crate::FileInit,
    ) -> Result<(), io::Error> {
        Err(io::ErrorKind::PermissionDenied.into())
    }

    fn list(&self) -> Self::List {
        Box::new(
            T::iter()
                .map(|m| Ok(RelativePathBuf::from(m.as_ref())))
                .collect::<Vec<_>>()
                .into_iter(),
        )
    }
}

pub struct EmbedFile<T>(rust_embed::EmbeddedFile, PhantomData<T>);

impl<T: rust_embed::RustEmbed + Send + Sync> AsyncFile for EmbedFile<T> {
    type Body = futures::stream::Once<futures::future::Ready<io::Result<Bytes>>>;

    fn read_range(
        &self,
        range: std::ops::Range<u64>,
    ) -> impl futures::prelude::Future<Output = Result<bytes::Bytes, std::io::Error>> + Send {
        async move { todo!() }
    }

    fn reader(
        &self,
    ) -> impl futures::prelude::Future<Output = Result<Self::Body, std::io::Error>> + Send {
        let bytes = Bytes::from(self.0.data.to_vec());
        async move {
            let stream = futures::stream::once(futures::future::ok(bytes));
            Ok(stream)
        }
    }
}

impl<T: rust_embed::RustEmbed + Send + Sync> File for EmbedFile<T> {
    type Body = Cursor<Vec<u8>>;

    fn read_range(&self, range: std::ops::Range<u64>) -> Result<Bytes, io::Error> {
        todo!()
    }

    fn reader(&self) -> Result<Self::Body, io::Error> {
        Ok(Cursor::new(self.0.data.to_vec()))
    }
}
