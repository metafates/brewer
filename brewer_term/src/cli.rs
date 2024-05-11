use std::io::{BufWriter, Write};

use clap::{Args, Parser, Subcommand};
use colored::Colorize;
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
}

pub mod which {
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::io::{BufWriter, IsTerminal, Write};
    use std::sync::Arc;

    use clap::Args;
    use colored::Colorize;
    use skim::{ItemPreview, PreviewContext, Skim, SkimItem, SkimItemReceiver, SkimItemSender};
    use skim::prelude::{SkimOptionsBuilder, unbounded};

    use brewer_core::models;
    use brewer_engine::State;

    use crate::cli::info_formula;

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

            let options = SkimOptionsBuilder::default()
                .multi(false)
                .preview(Some("")) // preview should be specified to enable preview window
                .preview_window(Some("60%"))
                .header(Some("Executables"))
                .build()?;

            let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();

            for (name, provided_by) in executables {
                tx.send(Arc::new(Executable {
                    name,
                    provided_by,
                }))?;
            }

            drop(tx);

            let item = Skim::run_with(&options, Some(rx))
                .map(|out| out.selected_items)
                .unwrap_or_default()
                .first()
                .map(|item| item.output().to_string())
                .unwrap_or_default();

            Ok(item)
        }
    }

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

#[derive(Args)]
pub struct Info {
    pub name: String,

    /// Treat the given name as cask
    #[clap(long, short, action)]
    pub cask: bool,
}

impl Info {
    pub fn run(&self, state: State) -> anyhow::Result<bool> {
        let buf = BufWriter::new(std::io::stdout());

        if self.cask {
            let Some(cask) = state.casks.all.get(&self.name) else {
                return Ok(false);
            };

            info_cask(buf, cask, state.casks.installed.get(&self.name))?;

            return Ok(true);
        }

        match state.formulae.all.get(&self.name) {
            Some(formula) => info_formula(buf, formula, state.formulae.installed.get(&self.name))?,
            None => {
                match state.casks.all.get(&self.name) {
                    Some(cask) => info_cask(buf, cask, state.casks.installed.get(&self.name))?,
                    None => return Ok(false)
                }
            }
        };

        Ok(true)
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

fn info_cask(mut buf: impl Write, cask: &models::cask::Cask, installed: Option<&models::cask::installed::Cask>) -> anyhow::Result<()> {
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
    use std::sync::Arc;

    use clap::Args;
    use nucleo_matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization};
    use skim::{ItemPreview, PreviewContext, Skim, SkimItem, SkimItemReceiver, SkimItemSender};
    use skim::prelude::{SkimOptionsBuilder, unbounded};
    use terminal_size::{terminal_size, Width};

    use brewer_core::models;
    use brewer_engine::State;

    use crate::cli::{info_cask, info_formula};
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
            let options = SkimOptionsBuilder::default()
                .multi(true)
                .preview(Some("")) // preview should be specified to enable preview window
                .preview_window(Some("60%"))
                .header(Some("Search"))
                .build()?;

            let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();

            for formula in state.formulae.all.into_values() {
                let name = formula.base.name.clone();
                let keg = Keg::Formula(formula, Box::new(state.formulae.installed.get(&name).cloned()));

                tx.send(Arc::new(keg))?;
            }

            for cask in state.casks.all.into_values() {
                let token = cask.base.token.clone();
                let keg = Keg::Cask(cask, state.casks.installed.get(&token).cloned());

                tx.send(Arc::new(keg))?;
            }

            drop(tx);

            let selected_items = Skim::run_with(&options, Some(rx))
                .map(|out| out.selected_items)
                .unwrap_or_default();

            let selected_items: Vec<_> = selected_items
                .iter()
                .map(|selected_item| (**selected_item).as_any().downcast_ref::<Keg>().unwrap().to_owned())
                .collect();

            let mut kegs = Vec::new();

            for keg in selected_items {
                kegs.push(keg.clone());
            }

            Ok(kegs)
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