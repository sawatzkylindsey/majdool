use notify::event::{CreateKind, ModifyKind};
use notify::{Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

pub struct SourceListener<C: Fn(PathBuf) -> ()> {
    tx: mpsc::Sender<Result<Event, Error>>,
    rx: mpsc::Receiver<Result<Event, Error>>,
    callback: C,
}

impl<C: Fn(PathBuf) -> ()> SourceListener<C> {
    pub fn new(callback: C) -> Self {
        let (tx, rx) = mpsc::channel(100);

        SourceListener { tx, rx, callback }
    }
}

impl<C: Fn(PathBuf) -> () + Send> SourceListener<C> {
    pub async fn listen(mut self, source: &Path) {
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                self.tx.blocking_send(res).unwrap();
            },
            notify::Config::default(),
        )
        .unwrap();

        watcher.watch(source, RecursiveMode::Recursive).unwrap();

        while let Some(res) = self.rx.recv().await {
            match res {
                Ok(event) => {
                    match event.kind {
                        EventKind::Create(CreateKind::File) => match accept_single_path(event) {
                            Ok(path) => {
                                (self.callback)(path);
                            }
                            Err(reason) => {
                                println!("DLQ: {reason}")
                            }
                        },
                        EventKind::Modify(ModifyKind::Data(_)) => match accept_single_path(event) {
                            Ok(path) => {
                                (self.callback)(path);
                            }
                            Err(reason) => {
                                println!("DLQ: {reason}")
                            }
                        },
                        EventKind::Access(_)
                        | EventKind::Remove(_)
                        | EventKind::Modify(ModifyKind::Metadata(_)) => {
                            // The 'do nothing' events.
                            // These don't need to be DLQ'ed, because we already understand they don't have an applicable handling response from our listener.
                        }
                        _ => {
                            println!("DLQ: unhandled event: {:?}", event)
                        }
                    }
                }
                Err(e) => println!("error: {:?}", e),
            }
        }
    }
}

fn accept_single_path<'a>(mut event: Event) -> Result<PathBuf, String> {
    // Right now, I'm not understanding what kinds of events could have multiple paths.
    // Let's just accept 1-path events, and "DLQ" the rest.
    match event.paths.len() {
        1 => Ok(event.paths.remove(0)),
        0 => Err(format!("no-path create event: {event:?}")),
        _ => Err(format!("multiple-path create event: {event:?}")),
    }
}
