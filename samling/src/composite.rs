use std::{collections::HashMap, io};

use futures::{StreamExt, TryStreamExt};
use relative_path::{RelativePath, RelativePathBuf};

use crate::{boxed::filestore_box, BoxFile, BoxFileStore, File, FileStore};

#[derive(Default)]
pub struct Composite {
    routes: HashMap<String, Vec<BoxFileStore>>,
}

impl Composite {
    pub fn register<T>(&mut self, mut mount: &str, filestore: T)
    where
        T: FileStore + Send + Sync + 'static,
        T::File: Send + Sync + 'static,
        <T::File as File>::Body: Send,
    {
        if mount.starts_with("/") {
            mount = &mount[1..];
        }

        self.routes
            .entry(mount.to_string())
            .or_default()
            .push(filestore_box(filestore));
    }

    async fn find<'a>(
        &self,
        name: &'a RelativePath,
    ) -> io::Result<(&Vec<BoxFileStore>, &'a RelativePath)> {
        let path = RelativePath::new(name);

        let Some(mut parent) = path.parent() else {
            return Err(io::ErrorKind::NotFound.into());
        };

        loop {
            if let Some(routes) = self.routes.get(parent.as_str()) {
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
}

impl FileStore for Composite {
    type File = BoxFile;

    fn metadata(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<crate::Metadata, std::io::Error>> + Send
    {
        async move {
            let (fs, path) = self.find(path).await?;
            fs.metadata(path).await
        }
    }

    fn open_file(
        &self,
        path: &relative_path::RelativePath,
    ) -> impl futures::prelude::Future<Output = Result<Self::File, std::io::Error>> + Send {
        async move {
            let (fs, path) = self.find(path).await?;
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
            let stream = self.routes.iter().map(|(k, v)| async move {
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
