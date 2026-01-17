mod listen;

use blarg::{CommandLineParser, Parameter, Scalar, derive::*};
use listen::SourceListener;
use majdool_lib::db::database::{MediaIndexDatabase, tmp_initialize};
use majdool_lib::fs::filesystem::MediaFilesystem;
use majdool_lib::media::MediaSystem;
use std::path::Path;

#[derive(Default, BlargParser)]
#[blarg(program = "majdool_syncer")]
struct Args {
    #[blarg(help = "Source directory path to sync from")]
    source: String,
    #[blarg(help = "Target directory path to sync to")]
    target: String,
}

#[tokio::main]
async fn main() {
    let args: Args = Args::blarg_parse();
    let source = Path::new(&args.source);
    let target = Path::new(&args.target);

    if !source.exists() || !source.is_dir() {
        panic!("invalid source path (must exist and be a directory): {source:?}")
    }

    if !target.exists() || !target.is_dir() {
        panic!("invalid target path (must exist and be a directory): {target:?}")
    }

    let pool = tmp_initialize().await;
    let media_db = MediaIndexDatabase::new(pool);
    let media_fs = MediaFilesystem::new(target.to_path_buf()).unwrap();
    let media_system = MediaSystem::new(media_db, media_fs);

    let source_listener = SourceListener::new(|path| async {
        println!("callback: {path:?}");
        match &media_system.flush_file(path).await {
            Ok(_) => {
                println!("it flushed");
            }
            Err(e) => {
                println!("it failed: {}", e);
            }
        };
    });
    source_listener.listen(source).await;

    println!("Doners!");
}
