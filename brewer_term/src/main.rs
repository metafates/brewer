use std::process::exit;

use clap::Parser;

use brewer_core::Brew;
use brewer_engine::Engine;
use log::LevelFilter;

use crate::cli::{Cli, Commands};
use crate::settings::AutoUpdate;

mod cli;
mod pretty;
mod settings;

fn setup_logger(level: LevelFilter) {
    env_logger::builder().filter_level(level).init();
}

fn run() -> anyhow::Result<bool> {
    let c = Cli::parse();

    setup_logger(c.verbose.log_level_filter());

    match c.command {
        Commands::Which(cmd) => {
            let settings = settings::Settings::new()?;

            let mut engine = get_engine(settings)?;
            let state = engine.cache_or_latest()?;

            Ok(cmd.run(state)?)
        }
        Commands::Update(cmd) => {
            let settings = settings::Settings::new()?;

            let engine = get_engine(settings)?;

            cmd.run(engine)?;

            Ok(true)
        }
        Commands::List(cmd) => {
            let settings = settings::Settings::new()?;

            let mut engine = get_engine(settings)?;
            let state = engine.cache_or_latest()?;

            cmd.run(state)?;

            Ok(true)
        }
        Commands::Info(cmd) => {
            let settings = settings::Settings::new()?;

            let mut engine = get_engine(settings)?;
            let state = engine.cache_or_latest()?;

            Ok(cmd.run(state)?)
        }
        Commands::Search(cmd) => {
            let settings = settings::Settings::new()?;

            let mut engine = get_engine(settings)?;
            let state = engine.cache_or_latest()?;

            Ok(cmd.run(state)?)
        }
        Commands::Paths(cmd) => {
            cmd.run();

            Ok(true)
        }
        Commands::Exists(cmd) => {
            let settings = settings::Settings::new()?;

            let mut engine = get_engine(settings)?;
            let state = engine.cache_or_latest()?;

            Ok(cmd.run(state))
        }
        Commands::Install(cmd) => {
            let settings = settings::Settings::new()?;

            let engine = get_engine(settings)?;

            cmd.run(engine)?;

            Ok(true)
        }
        Commands::Uninstall(cmd) => {
            let settings = settings::Settings::new()?;

            let engine = get_engine(settings)?;

            cmd.run(engine)?;

            Ok(true)
        }
    }
}

fn get_brew(settings: settings::Homebrew) -> anyhow::Result<Brew> {
    let brew = Brew::default();

    let brew = brewer_core::BrewBuilder::default()
        .path(settings.path.unwrap_or(brew.path))
        .prefix(settings.prefix.unwrap_or(brew.prefix))
        .build()?;

    Ok(brew)
}

fn get_engine(settings: settings::Settings) -> anyhow::Result<Engine> {
    let db_path = if let Some(dir) = dirs::cache_dir() {
        dir.join("brewer.db")
    } else {
        "brewer.db".into()
    };

    let store = brewer_engine::store::Store::open(db_path.as_path())?;

    let mut engine_builder = brewer_engine::EngineBuilder::default();

    engine_builder.store(store);

    if let AutoUpdate::Every(duration) = settings.cache.auto_update {
        engine_builder.cache_duration(Some(duration));
    } else {
        engine_builder.cache_duration(None);
    }

    let brew = get_brew(settings.homebrew)?;

    engine_builder.brew(brew);

    let engine = engine_builder.build()?;

    Ok(engine)
}

fn main() {
    match run() {
        Ok(success) => {
            if success {
                exit(0)
            } else {
                exit(1)
            }
        }
        Err(e) => {
            eprintln!("{}", pretty::header::error!("{e}"));
            exit(1)
        }
    }
}
