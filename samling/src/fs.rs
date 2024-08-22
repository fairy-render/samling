use std::{
    io::{self, SeekFrom},
    os::unix::fs::MetadataExt,
    path::PathBuf,
};

use async_stream::try_stream;
use bytes::Bytes;
use futures::{stream::BoxStream, Future, TryStreamExt};
use relative_path::{RelativePath, RelativePathBuf};
use std::collections::VecDeque;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_util::io::ReaderStream;
use url::Url;

use crate::{AsyncFile, AsyncFileStore, File, FileInit, FileStore, Metadata};

impl AsyncFileStore for FsFileStore {
    type File = FsFile;

    fn metadata(
        &self,
        path: &RelativePath,
    ) -> impl Future<Output = Result<Metadata, std::io::Error>> + Send {
        async move {
            let meta = tokio::fs::metadata(path.to_logical_path(&self.root)).await?;

            if !meta.is_file() {
                return Err(io::Error::other("not a file"));
            }

            let mime = if let Some(ext) = path.extension() {
                mime_guess::from_ext(ext).first_or_octet_stream()
            } else {
                mime::APPLICATION_OCTET_STREAM
            };

            Ok(Metadata {
                path: path.to_relative_path_buf(),
                size: meta.size(),
                mime,
            })
        }
    }

    fn open_file(
        &self,
        path: &RelativePath,
    ) -> impl Future<Output = Result<Self::File, std::io::Error>> + Send {
        async move {
            let full_path = path.to_logical_path(&self.root);
            if !full_path.is_file() {
                panic!("file does not exits")
            }

            Ok(FsFile { path: full_path })
        }
    }

    fn rm_file(&self, path: &RelativePath) -> impl Future<Output = Result<(), io::Error>> + Send {
        async move {
            let full_path = path.to_logical_path(&self.root);
            if !full_path.is_file() {
                return Ok(());
            }

            tokio::fs::remove_file(full_path).await?;

            Ok(())
        }
    }

    fn write_file(
        &self,
        path: &RelativePath,
        init: FileInit,
    ) -> impl Future<Output = Result<(), io::Error>> + Send {
        async move {
            let full_path = path.to_logical_path(&self.root);
            match init {
                FileInit::Bytes(bs) => {
                    tokio::fs::write(&full_path, &bs).await?;
                }
                FileInit::Stream(mut stream) => {
                    let mut file = tokio::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&full_path)
                        .await?;

                    while let Some(mut next) = stream.try_next().await? {
                        file.write_all_buf(&mut next).await?;
                    }

                    file.flush().await?;
                }
                FileInit::Path(path) => {
                    tokio::fs::copy(path, full_path).await?;
                }
            }

            Ok(())
        }
    }

    fn list(
        &self,
    ) -> impl Future<
        Output = Result<BoxStream<'static, Result<RelativePathBuf, io::Error>>, io::Error>,
    > + Send {
        let root = self.root.clone();
        async move {
            let stream = try_stream! {

              let mut queue = VecDeque::default();
              queue.push_back(root.clone());
              loop {
                let Some(next) = queue.pop_front() else {
                  break;
                };

                let mut read_dir = tokio::fs::read_dir(&next).await?;

                while let Some(next) = read_dir.next_entry().await? {
                  let path = next.path();
                  if path.is_dir() {
                    queue.push_back(path);
                    continue;
                  }

                  let rel_path = pathdiff::diff_paths(&path, &root);

                  yield RelativePathBuf::from_path(rel_path.unwrap()).unwrap();
                }
              }

            };

            Ok(Box::pin(stream) as BoxStream<'static, Result<RelativePathBuf, io::Error>>)
        }
    }
}

impl AsyncFile for FsFile {
    type Body = ReaderStream<tokio::fs::File>;

    fn read_range(
        &self,
        range: std::ops::Range<u64>,
    ) -> impl Future<Output = Result<Bytes, std::io::Error>> + Send {
        async move {
            let mut opts = tokio::fs::OpenOptions::new();
            let mut file = opts.read(true).open(&self.path).await?;
            file.seek(SeekFrom::Start(range.start)).await?;

            let count = (range.end - range.start) as usize;
            let mut buf = vec![0; count];
            file.read_exact(&mut buf).await?;

            Ok(buf.into())
        }
    }

    fn reader(&self) -> impl Future<Output = Result<Self::Body, std::io::Error>> + Send {
        async move {
            let mut opts = tokio::fs::OpenOptions::new();
            let file = opts.read(true).open(&self.path).await?;
            Ok(ReaderStream::new(file))
        }
    }

    fn url(&self) -> Option<url::Url> {
        Url::from_file_path(&self.path).ok()
    }
}

// Sync

pub struct FsFileStore {
    root: PathBuf,
}

impl FsFileStore {
    pub fn new(path: PathBuf) -> Result<FsFileStore, io::Error> {
        Ok(FsFileStore {
            root: std::fs::canonicalize(path)?,
        })
    }

    pub async fn new_async(path: PathBuf) -> Result<FsFileStore, io::Error> {
        Ok(FsFileStore {
            root: tokio::fs::canonicalize(path).await?,
        })
    }
}

impl FileStore for FsFileStore {
    type File = FsFile;

    type List = Box<dyn Iterator<Item = io::Result<RelativePathBuf>> + Send>;

    fn metadata(&self, path: &RelativePath) -> Result<Metadata, io::Error> {
        let meta = std::fs::metadata(path.to_logical_path(&self.root))?;

        if !meta.is_file() {
            return Err(io::Error::other("not a file"));
        }

        let mime = if let Some(ext) = path.extension() {
            mime_guess::from_ext(ext).first_or_octet_stream()
        } else {
            mime::APPLICATION_OCTET_STREAM
        };

        Ok(Metadata {
            path: path.to_relative_path_buf(),
            size: meta.size(),
            mime,
        })
    }

    fn open_file(&self, path: &RelativePath) -> Result<Self::File, io::Error> {
        let full_path = path.to_logical_path(&self.root);
        if !full_path.is_file() {
            panic!("file does not exits")
        }

        Ok(FsFile { path: full_path })
    }

    fn rm_file(&self, path: &RelativePath) -> Result<(), io::Error> {
        let full_path = path.to_logical_path(&self.root);
        if !full_path.is_file() {
            return Ok(());
        }

        std::fs::remove_file(full_path)?;

        Ok(())
    }

    fn write_file(&self, path: &RelativePath, init: FileInit) -> Result<(), io::Error> {
        todo!()
    }

    fn list(&self) -> Self::List {
        let root = self.root.clone();
        Box::new(
            walkdir::WalkDir::new(&self.root)
                .into_iter()
                .filter_map(move |m| match m {
                    Ok(m) => {
                        let path = m.path();
                        let rel_path = pathdiff::diff_paths(&path, &root);

                        rel_path
                            .and_then(|m| RelativePathBuf::from_path(m).ok())
                            .map(Ok)
                    }
                    Err(err) => {
                        // Some(Err(err));
                        panic!()
                    }
                }),
        )
    }
}

pub struct FsFile {
    path: PathBuf,
}

impl File for FsFile {
    type Body = std::fs::File;

    fn read_range(&self, range: std::ops::Range<u64>) -> Result<Bytes, io::Error> {
        todo!()
    }

    fn reader(&self) -> Result<Self::Body, io::Error> {
        std::fs::OpenOptions::new().read(true).open(&self.path)
    }
}
