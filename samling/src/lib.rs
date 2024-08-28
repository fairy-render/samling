mod boxed;
mod composite;
mod either;
mod file;
mod files;
mod path;
mod prefix;
mod store;
pub mod util;

#[cfg(feature = "embed")]
pub mod embed;

#[cfg(feature = "fs")]
pub mod fs;

pub use self::{
    boxed::{BoxAsyncFile, BoxAsyncFileStore, BoxFile, BoxFileStore},
    composite::*,
    file::*,
    files::AsyncFiles,
    path::*,
    prefix::Prefixed,
    store::*,
};

pub use url::Url;
