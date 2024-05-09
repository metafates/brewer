use std::path::PathBuf;

use clap::Parser;

use crate::cli::Cli;

mod cli;

fn run() -> anyhow::Result<()> {
    let store = brewer_engine::store::Store::open(PathBuf::from("brewer.db").as_path())?;

    let mut engine = brewer_engine::EngineBuilder::default()
        .store(store)
        .build()?;

    let state = engine.cache_or_latest()?;

    let c = Cli::parse();

    match c.command {
        cli::Commands::Which(which) => {
            for (_, f) in state.formulae.all {
                if f.executables.contains(&which.name) {
                    println!("{} {}", f.base.name, f.base.versions.stable);
                    println!("{}", f.base.desc);

                    println!();
                }
            }

            Ok(())
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e)
    }
}
