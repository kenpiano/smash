mod app;
mod backend;
mod editor;
mod lsp_task;
mod lsp_types;

use std::env;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    let file = args.get(1).map(PathBuf::from);

    if let Err(e) = editor::run_editor(file) {
        eprintln!("smash: {:#}", e);
        std::process::exit(1);
    }
}
