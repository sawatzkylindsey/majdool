mod listen;
use listen::SourceListener;

use blarg::{CommandLineParser, Parameter, Scalar, derive::*};
use majdool_lib::db::database::tmp_initialize;
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

    let media_db = tmp_initialize().await;

    let source_listener = SourceListener::new(|path| {
        println!("callback: {path:?}");
    });
    source_listener.listen(source).await;

    println!("Doners!");
}
