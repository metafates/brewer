use std::path::PathBuf;
use std::process::exit;

use clap::Parser;

use brewer_engine::Engine;

use crate::cli::{Cli, Commands};

mod cli;
mod pretty;

fn run() -> anyhow::Result<bool> {
    let c = Cli::parse();

    match c.command {
        Commands::Which(cmd) => {
            let mut engine = get_engine()?;
            let state = engine.cache_or_latest()?;

            Ok(cmd.run(state)?)
        }
        Commands::Update(cmd) => {
            let engine = get_engine()?;

            cmd.run(engine)?;

            Ok(true)
        }
        Commands::List(cmd) => {
            let mut engine = get_engine()?;
            let state = engine.cache_or_latest()?;

            cmd.run(state)?;

            Ok(true)
        }
        Commands::Info(cmd) => {
            let mut engine = get_engine()?;
            let state = engine.cache_or_latest()?;

            Ok(cmd.run(state)?)
        }
        Commands::Search(cmd) => {
            let mut engine = get_engine()?;
            let state = engine.cache_or_latest()?;

            Ok(cmd.run(state)?)
        }
    }
}

fn get_engine() -> anyhow::Result<Engine> {
    let store = brewer_engine::store::Store::open(PathBuf::from("brewer.db").as_path())?;

    let engine = brewer_engine::EngineBuilder::default()
        .store(store)
        .build()?;

    Ok(engine)
}

fn main() {
    match run() {
        Ok(success) => if success { exit(0) } else { exit(1) },
        Err(e) => {
            eprintln!("error: {}", e);
            exit(1)
        }
    }
}
