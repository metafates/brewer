use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::exit;

use clap::Parser;
use terminal_size::{terminal_size, Width};

use brewer_engine::{Engine, State};

use crate::cli::{Cli, Commands, Which};

mod cli;
mod pretty;

fn run() -> anyhow::Result<bool> {
    let c = Cli::parse();

    match c.command {
        Commands::Which(which) => {
            let mut engine = get_engine()?;
            let state = engine.cache_or_latest()?;

            Ok(cmd_which(which, state)?)
        }
        Commands::Update => {
            let engine = get_engine()?;

            cmd_update(engine)?;

            Ok(true)
        }
        Commands::List => {
            let mut engine = get_engine()?;
            let state = engine.cache_or_latest()?;

            cmd_list(state)?;

            Ok(true)
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

fn cmd_list(state: State) -> anyhow::Result<()> {
    let mut buf = BufWriter::new(std::io::stdout());

    let max_width = terminal_size().map(|(Width(w), _)| w).unwrap_or(80);

    {
        writeln!(buf, "{}", pretty::header("Formulae"))?;

        let mut installed: Vec<_> = state
            .formulae
            .installed
            .into_values()
            .filter_map(|f| {
                if f.receipt.installed_on_request {
                    Some(f.upstream.base.name)
                } else {
                    None
                }
            })
            .collect();

        installed.sort_unstable();

        let table = pretty::table(&installed, max_width);

        table.print(&mut buf)?;

        if !installed.is_empty() {
            writeln!(buf)?;
        }
    }
    {
        writeln!(buf, "{}", pretty::header("Casks"))?;

        let mut installed: Vec<_> = state
            .casks
            .installed
            .into_values()
            .map(|v| v.upstream.base.token)
            .collect();

        installed.sort_unstable();

        let table = pretty::table(&installed, max_width);

        table.print(&mut buf)?;
    }

    buf.flush()?;

    Ok(())
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
