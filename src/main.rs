mod model;
mod listen;

use std::path::Path;
use model::{Media, poc_psql};
use listen::SourceListener;

use blarg::{derive::*, CommandLineParser, Parameter, Scalar};

#[derive(Default, BlargParser)]
#[blarg(program = "majdool_uploader")]
struct Args {
    #[blarg(help = "Source path to upload from")]
    source: String,
}

#[tokio::main]
async fn main() {
    let args: Args = Args::blarg_parse();
    let source = Path::new(&args.source);

    if !source.exists() {
        panic!("invalid path: {source:?}")
    }

    let source_listener = SourceListener::new(|path| {
        println!("callback: {path:?}");
    });
    source_listener.listen(source).await;

    poc_psql().await;

    println!("Doners!");
}
