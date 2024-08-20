mod boxed;
mod composite;
mod either;
mod file;
mod files;
mod path;
mod store;
pub mod util;

#[cfg(feature = "embed")]
pub mod embed;

#[cfg(feature = "fs")]
pub mod fs;

pub use self::{
    boxed::{BoxFile, BoxFileStore},
    composite::*,
    file::*,
    files::Files,
    path::*,
    store::*,
};

pub use url::Url;
