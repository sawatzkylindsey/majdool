use blarg::{CommandLineParser, Parameter, Scalar, derive::*};
use majdool_lib::db::database::tmp_initialize;
use majdool_lib::fs::fsutil::compute_file_hash;
use std::path::Path;

#[derive(Default, BlargParser)]
#[blarg(program = "majdool_manual_syncer")]
struct Args {
    #[blarg(help = "Source file path to sync from")]
    source: String,
    #[blarg(help = "Target file path to sync to")]
    target: String,
}

#[tokio::main]
async fn main() {
    let args: Args = Args::blarg_parse();
    let source = Path::new(&args.source);
    let target = Path::new(&args.target);

    if !source.exists() || !source.is_file() {
        panic!("invalid source path (must exist and be a file): {source:?}")
    }

    if target.exists() {
        panic!("invalid target path (must not exist): {target:?}")
    }

    let mut media_db = tmp_initialize().await;

    let hash = compute_file_hash(&source).await.unwrap();
    let result1 = media_db.media_lookup(hash).await;
    println!("lookup {:?}", result1);
    let result2 = media_db.media_insert(&hash).await;
    println!("insert {:?}", result2);
    let result3 = media_db.media_sync(result2.unwrap(), &target).await;
    println!("sync {:?}", result3);
    let result4 = media_db.media_lookup(hash).await;
    println!("lookup {:?}", result4);
}
