mod listen;
use listen::SourceListener;

use blarg::{CommandLineParser, Parameter, Scalar, derive::*};
use majdool_lib::database::tmp_initialize;
use std::path::Path;

#[derive(Default, BlargParser)]
#[blarg(program = "majdool_syncer")]
struct Args {
    #[blarg(help = "Source path to sync from")]
    source: String,
    #[blarg(help = "Target path to sync to")]
    target: String,
}

#[tokio::main]
async fn main() {
    let args: Args = Args::blarg_parse();
    let source = Path::new(&args.source);
    let target = Path::new(&args.target);

    if !source.exists() {
        panic!("invalid source path: {source:?}")
    }

    if !target.exists() {
        panic!("invalid target path: {target:?}")
    }

    let media_db = tmp_initialize().await;

    let source_listener = SourceListener::new(|path| {
        println!("callback: {path:?}");
    });
    source_listener.listen(source).await;

    println!("Doners!");
}
