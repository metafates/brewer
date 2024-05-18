use std::io::{BufWriter, Write};
use std::sync::Arc;

use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use skim::{Skim, SkimItem, SkimItemReceiver, SkimItemSender};
use skim::prelude::{SkimOptionsBuilder, unbounded};
use terminal_size::{terminal_size, Width};

use brewer_core::models;
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
    Which(which::Which),

    /// Update the local cache
    Update(Update),

    /// List installed formulae and casks
    #[clap(alias = "ls")]
    List(List),

    /// Show information about formula or cask
    Info(Info),

    /// Search for formulae and casks
    #[clap(alias = "s")]
    Search(search::Search),

    /// Show paths that brewer uses
    Paths(paths::Paths),

    /// Indicate if the given formula or cask exists by exit code.
    Exists(Exists),

    /// Install the given formula or cask.
    #[clap(alias = "i")]
    Install(install::Install),

    /// Uninstall the given formula or cask.
    #[clap(aliases = & ["r", "remove"])]
    Uninstall(uninstall::Uninstall),
}

pub mod which {
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::io::{BufWriter, IsTerminal, Write};

    use clap::Args;
    use colored::Colorize;
    use skim::{ItemPreview, PreviewContext, SkimItem};

    use brewer_core::models;
    use brewer_engine::State;

    use crate::cli::{info_formula, select_skim};

    #[derive(Args)]
    pub struct Which {
        pub name: Option<String>,

        /// Show all matched formulae instead of the most popular one.
        #[clap(long, short, action)]
        pub all: bool,
    }

    impl Which {
        pub fn run(&self, state: State) -> anyhow::Result<bool> {
            let name = if let Some(name) = &self.name {
                name.to_string()
            } else {
                self.run_skim(&state)?
            };

            let mut formulae: Vec<_> = state
                .formulae
                .all
                .into_iter()
                .filter_map(|(_, f)| {
                    if f.executables.contains(&name) {
                        Some(f)
                    } else {
                        None
                    }
                })
                .collect();

            if formulae.is_empty() {
                return Ok(false);
            }

            formulae.sort_unstable_by_key(|f| f.analytics.as_ref().map(|a| a.number).unwrap_or_default());

            let mut buf = BufWriter::new(std::io::stdout());

            if std::io::stdout().is_terminal() {
                if self.all {
                    for (i, f) in formulae.iter().enumerate() {
                        info_formula(&mut buf, f, None)?;

                        if i != formulae.len() - 1 {
                            writeln!(buf)?;
                        }
                    }
                } else {
                    // we return early if formulae is empty, so we have at least 1 element
                    let first = formulae.first().unwrap();

                    info_formula(&mut buf, first, None)?;

                    let rest: Vec<_> = formulae.into_iter().skip(1).collect();

                    if !rest.is_empty() {
                        write!(buf, "Command {} is also provided by", name.purple().bold())?;

                        for f in rest {
                            write!(buf, " {}", f.base.name.cyan().bold())?;
                        }

                        writeln!(buf)?;
                    }
                }
            } else {
                let formulae = if self.all {
                    formulae
                } else {
                    formulae.into_iter().take(1).collect()
                };

                for f in formulae {
                    writeln!(buf, "{}", f.base.name)?;
                }
            }

            buf.flush()?;

            Ok(true)
        }


        fn run_skim(&self, state: &State) -> anyhow::Result<String> {
            let mut executables: HashMap<String, models::formula::Store> = HashMap::new();

            for f in state.formulae.all.values() {
                for e in f.executables.iter() {
                    match executables.get_mut(e) {
                        Some(store) => {
                            store.insert(f.base.name.clone(), f.clone());
                        }
                        None => {
                            let mut store = HashMap::new();

                            store.insert(f.base.name.clone(), f.clone());

                            executables.insert(e.clone(), store);
                        }
                    }
                }
            }

            let executables = executables
                .into_iter()
                .map(|(name, provided_by)| Executable {
                    name,
                    provided_by,
                });

            let selected = select_skim(executables, "Executables", false)?;
            let selected = selected.into_iter().map(|e| e.name).take(1).collect();

            Ok(selected)
        }
    }

    #[derive(Clone)]
    struct Executable {
        pub name: String,
        pub provided_by: models::formula::Store,
    }

    impl SkimItem for Executable {
        fn text(&self) -> Cow<str> {
            Cow::Borrowed(&self.name)
        }

        fn preview(&self, _context: PreviewContext) -> ItemPreview {
            let mut w = Vec::new();

            writeln!(w, "Provided by").unwrap();
            writeln!(w).unwrap();

            for (i, f) in self.provided_by.values().enumerate() {
                info_formula(&mut w, f, None).unwrap();

                if i != self.provided_by.len() - 1 {
                    writeln!(w).unwrap();
                }
            }

            let preview = String::from_utf8(w).unwrap();
            let preview = textwrap::wrap(&preview, _context.width).join("\n");

            ItemPreview::AnsiText(preview)
        }
    }
}


#[derive(Args)]
pub struct Update {}

impl Update {
    pub fn run(&self, mut engine: Engine) -> anyhow::Result<()> {
        println!("Updating the database, this will take some time");

        let state = engine.latest()?;

        engine.update_cache(&state)?;

        println!("Database updated, found {} formulae and {} casks", state.formulae.all.len(), state.casks.all.len());

        Ok(())
    }
}

#[derive(Args)]
pub struct List {
    /// List formulae
    #[clap(short, long, action, group = "type")]
    pub casks: bool,

    /// List casks
    #[clap(short, long, action, group = "type")]
    pub formulae: bool,

    /// List the formulae installed on request.
    #[clap(short = 'r', long, action, group = "installed")]
    pub installed_on_request: bool,

    /// List the formulae installed as dependencies.
    #[clap(short = 'd', long, action, group = "installed")]
    pub installed_as_dependency: bool,
}

impl List {
    pub fn run(&self, state: State) -> anyhow::Result<()> {
        let mut buf = BufWriter::new(std::io::stdout());

        let max_width = terminal_size().map(|(Width(w), _)| w).unwrap_or(80);

        if self.formulae {
            self.list_formulae(&mut buf, max_width, state.formulae.installed)?;
            return Ok(());
        }

        if !self.casks {
            self.list_formulae(&mut buf, max_width, state.formulae.installed)?;
        }

        if !self.formulae {
            self.list_casks(&mut buf, max_width, state.casks.installed)?;
        }

        buf.flush()?;

        Ok(())
    }

    fn list_formulae(&self, w: &mut impl Write, max_width: u16, formulae: models::formula::installed::Store) -> anyhow::Result<()> {
        writeln!(w, "{}", pretty::header("Formulae"))?;
        let mut installed: Vec<_> = formulae
            .into_values()
            .filter_map(|f| {
                let name = f.upstream.base.name;

                if self.installed_as_dependency {
                    return if f.receipt.installed_as_dependency {
                        Some(name)
                    } else {
                        None
                    };
                }

                if self.installed_on_request {
                    return if f.receipt.installed_on_request {
                        Some(name)
                    } else {
                        None
                    };
                }

                Some(name)
            })
            .collect();

        installed.sort_unstable();

        let table = pretty::table(&installed, max_width);

        table.print(w)?;

        Ok(())
    }

    fn list_casks(&self, w: &mut impl Write, max_width: u16, casks: models::cask::installed::Store) -> anyhow::Result<()> {
        writeln!(w, "{}", pretty::header("Casks"))?;

        let mut installed: Vec<_> = casks
            .into_values()
            .map(|v| v.upstream.base.token)
            .collect();

        installed.sort_unstable();

        let table = pretty::table(&installed, max_width);

        table.print(w)?;

        Ok(())
    }
}

#[derive(Args)]
pub struct Info {
    pub name: String,

    /// Treat the given name as cask
    #[clap(long, short, action, group = "type")]
    pub cask: bool,

    /// Treat the given name as formula
    #[clap(long, short, action, group = "type")]
    pub formula: bool,

    /// Open the homepage using default browser
    #[clap(long, short, action)]
    pub open_homepage: bool,
}

impl Info {
    pub fn run(&self, state: State) -> anyhow::Result<bool> {
        if self.cask {
            let Some(cask) = state.casks.all.get(&self.name) else {
                return Ok(false);
            };

            self.handle_cask(cask, state.casks.installed.get(&self.name))?;

            return Ok(true);
        }

        if self.formula {
            let Some(formula) = state.formulae.all.get(&self.name) else {
                return Ok(false);
            };

            self.handle_formula(formula, state.formulae.installed.get(&self.name))?;

            return Ok(true);
        }

        match state.formulae.all.get(&self.name) {
            Some(formula) => self.handle_formula(formula, state.formulae.installed.get(&self.name))?,
            None => {
                match state.casks.all.get(&self.name) {
                    Some(cask) => self.handle_cask(cask, state.casks.installed.get(&self.name))?,
                    None => return Ok(false)
                }
            }
        };

        Ok(true)
    }

    pub fn handle_formula(&self, formula: &models::formula::Formula, installed: Option<&models::formula::installed::Formula>) -> anyhow::Result<()> {
        if self.open_homepage {
            open::that_detached(&formula.base.homepage)?;
            return Ok(());
        }

        let mut buf = BufWriter::new(std::io::stdout());

        info_formula(&mut buf, formula, installed)?;

        buf.flush()?;

        Ok(())
    }

    pub fn handle_cask(&self, cask: &models::cask::Cask, installed: Option<&models::cask::installed::Cask>) -> anyhow::Result<()> {
        if self.open_homepage {
            open::that_detached(&cask.base.homepage)?;
            return Ok(());
        }

        let mut buf = BufWriter::new(std::io::stdout());

        info_cask(&mut buf, cask, installed)?;

        buf.flush()?;

        Ok(())
    }
}

fn info_formula(mut buf: impl Write, formula: &models::formula::Formula, installed: Option<&models::formula::installed::Formula>) -> anyhow::Result<()> {
    writeln!(buf, "{} {} (Cask)", pretty::header(&formula.base.name), formula.base.versions.stable)?;
    writeln!(buf, "From {}", formula.base.tap.yellow())?;

    if let Some(installed) = installed {
        writeln!(buf)?;
        writeln!(buf, "Installed {} {}", installed.receipt.source.version(), pretty::bool(true))?;
    }


    writeln!(buf)?;
    writeln!(buf, "{}", formula.base.homepage.underline().blue())?;
    writeln!(buf)?;
    writeln!(buf, "{}", formula.base.desc.italic())?;

    if !formula.executables.is_empty() {
        writeln!(buf)?;
        write!(buf, "Provides")?;

        const LIMIT: usize = 5;

        if formula.executables.len() > LIMIT {
            for e in formula.executables.iter().take(LIMIT) {
                write!(buf, " {}", e.bold().purple())?;
            }

            write!(buf, " and {} more", formula.executables.len() - LIMIT)?;
        } else {
            for e in formula.executables.iter() {
                write!(buf, " {}", e.bold().purple())?;
            }
        }


        writeln!(buf)?;
    }

    Ok(())
}

fn info_cask(buf: &mut impl Write, cask: &models::cask::Cask, installed: Option<&models::cask::installed::Cask>) -> anyhow::Result<()> {
    writeln!(buf, "{} {} (Formula)", pretty::header(&cask.base.token), cask.base.version)?;
    writeln!(buf, "From {}", cask.base.tap.yellow())?;
    writeln!(buf)?;

    if let Some(installed) = installed {
        let versions: Vec<_> = installed.versions.iter().cloned().collect();
        let versions = versions.join(", ");

        writeln!(buf, "Installed {versions} {}", pretty::bool(true))?;
        writeln!(buf)?;
    }

    writeln!(buf, "{}", cask.base.homepage.underline().blue())?;
    writeln!(buf)?;


    let desc = if let Some(desc) = &cask.base.desc {
        desc
    } else {
        "No description"
    };

    writeln!(buf, "{}", desc.italic())?;

    Ok(())
}

pub mod search {
    use std::borrow::Cow;
    use std::io::{BufWriter, IsTerminal, Write};

    use clap::Args;
    use nucleo_matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization};
    use skim::{ItemPreview, PreviewContext, SkimItem};
    use terminal_size::{terminal_size, Width};

    use brewer_core::models;
    use brewer_engine::State;

    use crate::cli::{info_cask, info_formula, select_skim};
    use crate::pretty;

    #[derive(Args)]
    pub struct Search {
        pub name: Option<String>,
    }

    impl Search {
        pub fn run(&self, state: State) -> anyhow::Result<bool> {
            let kegs = match &self.name {
                Some(name) => {
                    let mut matcher = nucleo_matcher::Matcher::new(nucleo_matcher::Config::DEFAULT);

                    let atom = Atom::new(name, CaseMatching::Ignore, Normalization::Smart, AtomKind::Substring, false);

                    let formulae = atom.match_list(state.formulae.all.into_values(), &mut matcher);
                    let mut formulae: Vec<_> = formulae.into_iter().map(|(formula, _)| {
                        let installed = state.formulae.installed.get(&formula.base.name);

                        Keg::Formula(formula, Box::new(installed.cloned()))
                    }).collect();

                    let casks = atom.match_list(state.casks.all.into_values(), &mut matcher);
                    let mut casks: Vec<_> = casks.into_iter().map(|(cask, _)| {
                        let installed = state.casks.installed.get(&cask.base.token);

                        Keg::Cask(cask, installed.cloned())
                    }).collect();

                    formulae.append(&mut casks);

                    formulae
                }
                None => self.run_skim(state)?
            };

            if kegs.is_empty() {
                return Ok(false);
            }

            if !std::io::stdout().is_terminal() {
                for keg in kegs {
                    match keg {
                        Keg::Formula(formula, _) => println!("{}", formula.base.name),
                        Keg::Cask(cask, _) => println!("{}", cask.base.token),
                    };
                }

                return Ok(true);
            }

            let width = terminal_size().map(|(Width(w), _)| w).unwrap_or(80);

            let mut formulae = Vec::new();
            let mut casks = Vec::new();

            for keg in kegs {
                match keg {
                    Keg::Formula(formula, installed) => {
                        let name = if installed.is_some() {
                            format!("{} {}", formula.base.name, pretty::bool(true))
                        } else {
                            formula.base.name
                        };

                        formulae.push(name)
                    }
                    Keg::Cask(cask, installed) => {
                        let name = if installed.is_some() {
                            format!("{} {}", cask.base.token, pretty::bool(true))
                        } else {
                            cask.base.token
                        };

                        casks.push(name)
                    }
                }
            }

            let formulae = pretty::table(&formulae, width);
            let casks = pretty::table(&casks, width);

            let mut buf = BufWriter::new(std::io::stdout());

            writeln!(buf, "{}", pretty::header("Formulae"))?;
            formulae.print(&mut buf)?;

            writeln!(buf)?;

            writeln!(buf, "{}", pretty::header("Casks"))?;
            casks.print(&mut buf)?;

            Ok(true)
        }

        fn run_skim(&self, state: State) -> anyhow::Result<Vec<Keg>> {
            let mut kegs: Vec<Keg> = Vec::new();

            for formula in state.formulae.all.into_values() {
                let name = formula.base.name.clone();
                let keg = Keg::Formula(formula, Box::new(state.formulae.installed.get(&name).cloned()));

                kegs.push(keg);
            }

            for cask in state.casks.all.into_values() {
                let token = cask.base.token.clone();
                let keg = Keg::Cask(cask, state.casks.installed.get(&token).cloned());

                kegs.push(keg);
            }

            let selected = select_skim(kegs, "Search", true)?;

            Ok(selected)
        }
    }

    #[derive(Clone)]
    enum Keg {
        Formula(models::formula::Formula, Box<Option<models::formula::installed::Formula>>),
        Cask(models::cask::Cask, Option<models::cask::installed::Cask>),
    }

    impl SkimItem for Keg {
        fn text(&self) -> Cow<str> {
            match self {
                Keg::Formula(formula, _) => Cow::Borrowed(&formula.base.name),
                Keg::Cask(cask, _) => Cow::Borrowed(&cask.base.token)
            }
        }

        fn preview(&self, _context: PreviewContext) -> ItemPreview {
            let mut w = Vec::new();

            match self {
                Keg::Formula(formula, installed) => info_formula(&mut w, formula, installed.as_ref().as_ref()).unwrap(),
                Keg::Cask(cask, installed) => info_cask(&mut w, cask, installed.as_ref()).unwrap(),
            };

            let preview = String::from_utf8(w).unwrap();
            let preview = textwrap::wrap(&preview, _context.width).join("\n");

            ItemPreview::AnsiText(preview)
        }
    }
}

pub mod paths {
    use clap::{Parser, Subcommand};

    use crate::settings;

    #[derive(Parser)]
    pub struct Paths {
        #[command(subcommand)]
        pub command: Commands,
    }

    #[derive(Subcommand)]
    pub enum Commands {
        /// Show config path
        Config
    }

    impl Paths {
        pub fn run(&self) {
            match self.command {
                Commands::Config => println!("{}.toml", settings::Settings::config_file().to_string_lossy()),
            }
        }
    }
}

#[derive(Args)]
pub struct Exists {
    pub name: String,

    /// Treat given name as formula
    #[clap(short, long, action)]
    pub formula: bool,

    /// Treat given name as cask
    #[clap(short, long, action)]
    pub cask: bool,
}

impl Exists {
    pub fn run(&self, state: State) -> bool {
        let formulae = state.formulae.all;
        let casks = state.casks.all;

        if self.cask {
            return casks.contains_key(&self.name);
        }

        if self.formula {
            return formulae.contains_key(&self.name);
        }

        formulae.contains_key(&self.name) || casks.contains_key(&self.name)
    }
}

pub mod install {
    use std::borrow::Cow;
    use std::io::{BufWriter, Write};
    use std::ops::Deref;

    use anyhow::bail;
    use clap::Args;
    use colored::Colorize;
    use inquire::{Confirm, InquireError};
    use skim::{ItemPreview, PreviewContext, SkimItem};

    use brewer_core::models;
    use brewer_engine::{Engine, State};

    use crate::cli::{info_cask, info_formula, select_skim};
    use crate::pretty;

    #[derive(Args)]
    pub struct Install {
        pub names: Vec<String>,

        #[clap(short, long, action, group = "type")]
        pub formula: bool,

        #[clap(short, long, action, group = "type")]
        pub cask: bool,

        /// Confirm
        #[clap(short, long, action)]
        pub yes: bool,
    }

    impl Install {
        pub fn run(&self, mut engine: Engine) -> anyhow::Result<()> {
            let state = engine.cache_or_latest()?;

            let kegs = self.get_kegs(state)?;

            if kegs.is_empty() {
                Ok(())
            } else {
                if self.yes || plan(&kegs)? {
                    engine.install(kegs)?;
                }

                Ok(())
            }
        }

        fn get_kegs(&self, state: State) -> anyhow::Result<Vec<models::Keg>> {
            if self.names.is_empty() {
                self.get_kegs_from_skim(state)
            } else {
                self.get_kegs_from_args(state)
            }
        }

        fn get_kegs_from_args(&self, mut state: State) -> anyhow::Result<Vec<models::Keg>> {
            let mut kegs = Vec::new();

            for name in &self.names {
                let keg = if self.formula {
                    if state.formulae.installed.contains_key(name) {
                        println!("formula {name} is already installed, skipping");
                        continue;
                    }

                    state.formulae.all.remove(name).map(models::Keg::Formula)
                } else if self.cask {
                    if state.casks.installed.contains_key(name) {
                        println!("cask {name} is already installed, skipping");
                        continue;
                    }

                    state.casks.all.remove(name).map(models::Keg::Cask)
                } else {
                    if state.formulae.installed.contains_key(name) {
                        println!("formula {name} is already installed, skipping");
                        continue;
                    }

                    if state.casks.installed.contains_key(name) {
                        println!("cask {name} is already installed, skipping");
                        continue;
                    }

                    state
                        .formulae
                        .all
                        .remove(name)
                        .map(models::Keg::Formula)
                        .or_else(|| state.casks.all.remove(name).map(models::Keg::Cask))
                };

                let Some(keg) = keg else {
                    bail!("keg {} not found", name);
                };

                kegs.push(keg);
            }

            Ok(kegs)
        }

        fn get_kegs_from_skim(&self, state: State) -> anyhow::Result<Vec<models::Keg>> {
            let mut non_installed: Vec<Keg> = Vec::with_capacity(state.formulae.all.len() + state.casks.all.len());

            for formula in state.formulae.all.into_values() {
                if !state.formulae.installed.contains_key(&formula.base.name) {
                    non_installed.push(formula.into());
                }
            }

            for cask in state.casks.all.into_values() {
                if !state.casks.installed.contains_key(&cask.base.token) {
                    non_installed.push(cask.into());
                }
            }

            let selected = select_skim(non_installed, "Install", true)?
                .into_iter()
                .map(|k| k.0)
                .collect();

            Ok(selected)
        }
    }

    fn plan(kegs: &Vec<models::Keg>) -> anyhow::Result<bool> {
        let mut w = BufWriter::new(std::io::stderr());

        writeln!(w, "{}", pretty::header("The following kegs will be installed"))?;

        for keg in kegs {
            match &keg {
                models::Keg::Formula(f) => writeln!(w, "{} {} (Formula)", f.base.name.cyan(), f.base.versions.stable)?,
                models::Keg::Cask(c) => writeln!(w, "{} {} (Cask)", c.base.token.cyan(), c.base.version)?,
            }
        }

        writeln!(w)?;

        let mut executables: Vec<String> = Vec::new();

        for k in kegs {
            if let models::Keg::Formula(f) = &k {
                for e in &f.executables {
                    executables.push(e.purple().to_string());
                }
            }
        }

        if !executables.is_empty() {
            writeln!(w, "{}", pretty::header("The following executables will be provided"))?;
            writeln!(w, "{}", executables.join(" "))?;
            writeln!(w)?;
        }

        w.flush()?;

        let result = Confirm::new("Proceed?").with_default(false).prompt();


        match result {
            Ok(value) => Ok(value),
            Err(e) => match e {
                InquireError::OperationCanceled => Ok(false),
                e => Err(e.into())
            }
        }
    }

    #[derive(Clone)]
    struct Keg(models::Keg);

    impl From<models::formula::Formula> for Keg {
        fn from(value: models::formula::Formula) -> Self {
            Keg(value.into())
        }
    }

    impl From<models::cask::Cask> for Keg {
        fn from(value: models::cask::Cask) -> Self {
            Keg(value.into())
        }
    }

    impl Deref for Keg {
        type Target = models::Keg;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl SkimItem for Keg {
        fn text(&self) -> Cow<str> {
            match &self.0 {
                models::Keg::Formula(formula) => Cow::Borrowed(&formula.base.name),
                models::Keg::Cask(cask) => Cow::Borrowed(&cask.base.token),
            }
        }

        fn preview(&self, _context: PreviewContext) -> ItemPreview {
            let mut buf = Vec::new();

            match &self.0 {
                models::Keg::Formula(formula) => info_formula(&mut buf, formula, None).unwrap(),
                models::Keg::Cask(cask) => info_cask(&mut buf, cask, None).unwrap()
            };

            let preview = String::from_utf8(buf).unwrap();

            ItemPreview::AnsiText(preview)
        }
    }
}

pub mod uninstall {
    use std::borrow::Cow;
    use std::io::{BufWriter, Write};

    use anyhow::bail;
    use clap::Args;
    use colored::Colorize;
    use inquire::{Confirm, InquireError};
    use skim::{ItemPreview, PreviewContext, SkimItem};

    use brewer_core::models;
    use brewer_engine::{Engine, State};

    use crate::cli::{info_cask, info_formula, select_skim};
    use crate::pretty;

    #[derive(Args)]
    pub struct Uninstall {
        pub names: Vec<String>,

        #[clap(short, long, action, group = "type")]
        pub formula: bool,

        #[clap(short, long, action, group = "type")]
        pub cask: bool,

        /// Confirm
        #[clap(short, long, action)]
        pub yes: bool,
    }

    impl Uninstall {
        pub fn run(&self, mut engine: Engine) -> anyhow::Result<()> {
            let state = engine.cache_or_latest()?;

            let kegs = self.get_kegs(state)?;

            if kegs.is_empty() {
                Ok(())
            } else {
                let kegs = kegs
                    .into_iter()
                    .map(|k| match k {
                        Keg::Formula(formula) => formula.upstream.into(),
                        Keg::Cask(cask) => cask.upstream.into()
                    })
                    .collect();

                if self.yes || plan(&kegs)? {
                    engine.uninstall(kegs)?;
                }

                Ok(())
            }
        }

        fn get_kegs(&self, state: State) -> anyhow::Result<Vec<Keg>> {
            if self.names.is_empty() {
                self.get_kegs_from_skim(state)
            } else {
                self.get_kegs_from_args(state)
            }
        }

        fn get_kegs_from_args(&self, mut state: State) -> anyhow::Result<Vec<Keg>> {
            let mut kegs = Vec::new();

            for name in &self.names {
                let keg = if self.formula {
                    state.formulae.installed.remove(name).map(Keg::Formula)
                } else if self.cask {
                    state.casks.installed.remove(name).map(Keg::Cask)
                } else {
                    state
                        .formulae
                        .installed
                        .remove(name)
                        .map(Keg::Formula)
                        .or_else(|| state.casks.installed.remove(name).map(Keg::Cask))
                };

                let Some(keg) = keg else {
                    bail!("keg {} is not installed", name);
                };

                kegs.push(keg);
            }

            Ok(kegs)
        }

        fn get_kegs_from_skim(&self, state: State) -> anyhow::Result<Vec<Keg>> {
            let mut installed: Vec<Keg> = Vec::with_capacity(state.formulae.installed.len() + state.casks.installed.len());

            for formula in state.formulae.installed.into_values().filter(|f| f.receipt.installed_on_request) {
                installed.push(formula.into());
            }

            for cask in state.casks.installed.into_values() {
                installed.push(cask.into());
            }

            let selected = select_skim(installed, "Uninstall", true)?
                .into_iter()
                .collect();

            Ok(selected)
        }
    }


    fn plan(kegs: &Vec<models::Keg>) -> anyhow::Result<bool> {
        let mut w = BufWriter::new(std::io::stderr());

        writeln!(w, "{}", pretty::header("The following kegs will be uninstalled"))?;

        for keg in kegs {
            match &keg {
                models::Keg::Formula(f) => writeln!(w, "{} {} (Formula)", f.base.name.cyan(), f.base.versions.stable)?,
                models::Keg::Cask(c) => writeln!(w, "{} {} (Cask)", c.base.token.cyan(), c.base.version)?,
            }
        }

        writeln!(w)?;

        let mut executables: Vec<String> = Vec::new();

        for k in kegs {
            if let models::Keg::Formula(f) = &k {
                for e in &f.executables {
                    executables.push(e.purple().to_string());
                }
            }
        }

        if !executables.is_empty() {
            writeln!(w, "{}", pretty::header("The following executables will be removed"))?;
            writeln!(w, "{}", executables.join(" "))?;
            writeln!(w)?;
        }

        w.flush()?;

        let result = Confirm::new("Proceed?").with_default(false).prompt();


        match result {
            Ok(value) => Ok(value),
            Err(e) => match e {
                InquireError::OperationCanceled => Ok(false),
                e => Err(e.into())
            }
        }
    }

    #[derive(Clone)]
    pub enum Keg {
        Formula(models::formula::installed::Formula),
        Cask(models::cask::installed::Cask),
    }

    impl From<models::formula::installed::Formula> for Keg {
        fn from(value: models::formula::installed::Formula) -> Self {
            Keg::Formula(value)
        }
    }

    impl From<models::cask::installed::Cask> for Keg {
        fn from(value: models::cask::installed::Cask) -> Self {
            Keg::Cask(value)
        }
    }

    impl SkimItem for Keg {
        fn text(&self) -> Cow<str> {
            match &self {
                Keg::Formula(formula) => Cow::Borrowed(&formula.upstream.base.name),
                Keg::Cask(cask) => Cow::Borrowed(&cask.upstream.base.token),
            }
        }

        fn preview(&self, _context: PreviewContext) -> ItemPreview {
            let mut buf = Vec::new();

            match &self {
                Keg::Formula(formula) => info_formula(&mut buf, &formula.upstream, Some(formula)).unwrap(),
                Keg::Cask(cask) => info_cask(&mut buf, &cask.upstream, Some(cask)).unwrap()
            };

            let preview = String::from_utf8(buf).unwrap();

            ItemPreview::AnsiText(preview)
        }
    }
}

fn select_skim<T, I>(items: I, header: &str, multi: bool) -> anyhow::Result<Vec<T>>
    where
        T: SkimItem + Clone,
        I: IntoIterator<Item=T>
{
    let options = SkimOptionsBuilder::default()
        .multi(multi)
        .preview(Some("")) // preview should be specified to enable preview window
        .preview_window(Some("60%"))
        .header(Some(header))
        .build()?;

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();


    for item in items.into_iter() {
        tx.send(Arc::new(item))?;
    }

    drop(tx);

    match Skim::run_with(&options, Some(rx)) {
        Some(output) => {
            if output.is_abort {
                return Ok(Vec::new());
            }

            let mut selected = Vec::new();

            for item in output.selected_items {
                let item: T = (*item).as_any().downcast_ref::<T>().unwrap().to_owned();

                selected.push(item);
            }

            Ok(selected)
        }
        None => Ok(Vec::new())
    }
}