use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::exit;

use clap::Parser;

use brewer_engine::{Engine, State};

use crate::cli::{Cli, Commands, Which};

mod cli;

fn run() -> anyhow::Result<bool> {
    let store = brewer_engine::store::Store::open(PathBuf::from("brewer.db").as_path())?;

    let mut engine = brewer_engine::EngineBuilder::default()
        .store(store)
        .build()?;

    let state = engine.cache_or_latest()?;

    let c = Cli::parse();

    match c.command {
        Commands::Which(which) => {
            Ok(cmd_which(which, state)?)
        }
        Commands::Update => {
            cmd_update(engine)?;

            Ok(true)
        }
    }
}

fn cmd_update(mut engine: Engine) -> anyhow::Result<()> {
    let state = engine.latest()?;

    engine.update_cache(&state)?;

    println!("Updated, found {} formulae and {} casks", state.formulae.all.len(), state.casks.all.len());

    Ok(())
}

fn cmd_which(args: Which, state: State) -> anyhow::Result<bool> {
    let suitable: Vec<_> = state
        .formulae
        .all
        .into_iter()
        .filter_map(|(_, f)| {
            if f.executables.contains(&args.name) {
                Some(f)
            } else {
                None
            }
        })
        .collect();

    if suitable.is_empty() {
        return Ok(false);
    }

    let mut buf = BufWriter::new(std::io::stdout());

    for (i, f) in suitable.iter().enumerate() {
        writeln!(buf, "{} {}", f.base.name, f.base.versions.stable)?;
        writeln!(buf, "{}", f.base.desc)?;

        if i != suitable.len() - 1 {
            writeln!(buf)?;
        }
    }

    buf.flush()?;

    Ok(true)
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
