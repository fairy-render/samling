use std::path::PathBuf;

use futures::StreamExt;
use relative_path::RelativePath;
use samling::{fs::FsFileStore, FileStore, SyncComposite};

#[derive(rust_embed::Embed)]
#[folder = "examples"]
struct Asset;

//
fn main() {
    let mut fs = SyncComposite::default();

    fs.register("/", FsFileStore::new(PathBuf::from("samling")).unwrap());

    // fs.register(
    //     "/mount",
    //     FsFileStore::new(PathBuf::from("packages")).unwrap(),
    // );

    // fs.register("/embed", samling::embed::Embed::<Asset>::new());

    let file = fs.metadata(RelativePath::new("src/util.rs")).unwrap();

    println!("File {:?}", file);

    // let mut stream = fs.list().await.unwrap();

    // while let Some(next) = stream.next().await {
    //     println!("Next: {:?}", next);
    // }
}
