use std::path::PathBuf;

use filestore::{fs::FsFileStore, Composite, FileStore};
use futures::StreamExt;
use relative_path::RelativePath;

#[derive(rust_embed::Embed)]
#[folder = "examples"]
struct Asset;

//
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut fs = Composite::default();

    fs.register(
        "/",
        FsFileStore::new(PathBuf::from("filestore")).await.unwrap(),
    );

    fs.register(
        "/mount",
        FsFileStore::new(PathBuf::from("packages")).await.unwrap(),
    );

    fs.register("/embed", filestore::embed::Embed::<Asset>::new());

    let file = fs
        .metadata(RelativePath::new("embed/filestore.rs"))
        .await
        .unwrap();

    println!("File {:?}", file);

    // let mut stream = fs.list().await.unwrap();

    // while let Some(next) = stream.next().await {
    //     println!("Next: {:?}", next);
    // }
}
