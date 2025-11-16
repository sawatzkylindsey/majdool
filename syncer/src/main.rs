mod listen;
use listen::SourceListener;
use majdool_lib::model::poc_psql;

use blarg::{derive::*, CommandLineParser, Parameter, Scalar};
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

    let source_listener = SourceListener::new(|path| {
        println!("callback: {path:?}");
    });
    source_listener.listen(source).await;

    poc_psql().await;

    println!("Doners!");
}
