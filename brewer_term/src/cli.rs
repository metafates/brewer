use std::io::{BufWriter, Write};

use clap::{Parser, Subcommand};
use terminal_size::{terminal_size, Width};

use brewer_engine::{Engine, State};

use crate::pretty;

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Locate the formulae which provides the given executable
    Which(Which),

    /// Update the local cache
    Update(Update),

    /// List installed formulae and casks
    List(List),
}

#[derive(Parser)]
pub struct Which {
    pub name: String,
}

impl Which {
    pub fn run(&self, state: State) -> anyhow::Result<bool> {
        let suitable: Vec<_> = state
            .formulae
            .all
            .into_iter()
            .filter_map(|(_, f)| {
                if f.executables.contains(&self.name) {
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
}

#[derive(Parser)]
pub struct Update {}

impl Update {
    pub fn run(&self, mut engine: Engine) -> anyhow::Result<()> {
        let state = engine.latest()?;

        engine.update_cache(&state)?;

        println!("Updated, found {} formulae and {} casks", state.formulae.all.len(), state.casks.all.len());

        Ok(())
    }
}

#[derive(Parser)]
pub struct List {}

impl List {
    pub fn run(&self, state: State) -> anyhow::Result<()> {
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
}