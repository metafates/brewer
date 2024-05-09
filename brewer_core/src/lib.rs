use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

use derive_builder::Builder;
use serde::Deserialize;

use crate::models::*;

pub mod models;

const DEFAULT_BREW_PATH: &str = "brew";

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const DEFAULT_BREW_PREFIX: &str = "/opt/homebrew";

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const DEFAULT_BREW_PREFIX: &str = "/usr/local";

#[cfg(target_os = "linux")]
const DEFAULT_BREW_PREFIX: &str = "/home/linuxbrew/.linuxbrew";

const DEFAULT_BREW_BIN_REGISTRY_URL: &str = "https://raw.githubusercontent.com/Homebrew/homebrew-command-not-found/master/executables.txt";

#[derive(Builder, Clone)]
pub struct Brew {
    path: PathBuf,
    prefix: PathBuf,
}

impl Default for Brew {
    fn default() -> Self {
        Brew {
            path: DEFAULT_BREW_PATH.into(),
            prefix: DEFAULT_BREW_PREFIX.into(),
        }
    }
}

impl Brew {
    const JSON_FLAG: &'static str = "--json=v2";

    fn brew(&self) -> Command {
        Command::new(self.path.clone())
    }

    pub fn executables(&self) -> anyhow::Result<formula::Executables> {
        let body = reqwest::blocking::get(DEFAULT_BREW_BIN_REGISTRY_URL)?.text()?;
        let mut store = formula::Executables::new();

        for line in body.lines().filter(|l| !l.is_empty()) {
            let Some((lhs, rhs)) = line.split_once(':') else {
                continue;
            };

            let Some(index) = lhs.find('(') else {
                continue;
            };

            let name = &lhs[..index];
            let executables: HashSet<String> = rhs.split_whitespace().map(|s| s.to_string()).collect();

            store.insert(name.to_string(), executables);
        }

        Ok(store)
    }

    pub fn state(&self) -> anyhow::Result<State<formula::State, cask::State>> {
        let all = self.eval_all()?;
        let executables = self.executables()?;

        let all: State<formula::Store, cask::Store> = State {
            formulae: all.formulae.into_iter().map(|(k, base)| {
                let executables = if let Some(e) = executables.get(&k) {
                    e.clone()
                } else {
                    HashSet::new()
                };

                (k, formula::Formula {
                    base,
                    executables,
                })
            }).collect(),
            casks: all.casks.into_iter().map(|(k, base)| {
                (k, cask::Cask { base })
            }).collect(),
        };

        let installed = self.eval_installed(&all)?;

        Ok(State {
            formulae: formula::State {
                all: all.formulae,
                installed: installed.formulae,
            },
            casks: cask::State {
                all: all.casks,
                installed: installed.casks,
            },
        })
    }

    pub fn eval_installed(&self, all: &State<formula::Store, cask::Store>) -> anyhow::Result<State<formula::installed::Store, cask::installed::Store>> {
        let formulae = self.eval_installed_formulae(&all.formulae)?;
        let casks = self.eval_installed_casks(&all.casks)?;

        Ok(State { formulae, casks })
    }

    fn eval_installed_casks(&self, store: &cask::Store) -> anyhow::Result<cask::installed::Store> {
        let mut installed = cask::installed::Store::new();

        for (name, versions) in self.eval_installed_casks_versions()? {
            let Some(cask) = store.get(&name) else {
                continue;
            };

            installed.insert(name, cask::installed::Cask {
                upstream: cask.clone(),
                versions,
            });
        }

        Ok(installed)
    }

    fn eval_installed_casks_versions(&self) -> anyhow::Result<cask::installed::VersionsStore> {
        let caskroom = self.prefix.join("Caskroom").read_dir()?;

        let mut store = cask::installed::VersionsStore::new();

        for entry in caskroom {
            let entry = entry?;
            let path = entry.path();

            let Some(name) = path.file_name() else {
                continue;
            };

            let name = name.to_string_lossy().to_string();
            let mut versions: HashSet<String> = HashSet::new();

            for entry in path.canonicalize()?.read_dir()? {
                let entry = entry?;
                let path = entry.path();

                let Some(name) = path.file_name() else {
                    continue;
                };

                let name = name.to_string_lossy().to_string();

                if Self::is_dotfile(&name) {
                    continue;
                }

                versions.insert(name);
            }

            store.insert(name, versions);
        }

        Ok(store)
    }

    fn eval_installed_formulae(&self, store: &formula::Store) -> anyhow::Result<formula::installed::Store> {
        let mut installed = formula::installed::Store::new();

        for (name, receipt) in self.eval_installed_formulae_receipts()? {
            let Some(formula) = store.get(&name) else {
                continue;
            };

            installed.insert(name, formula::installed::Formula {
                upstream: formula.clone(),
                receipt,
            });
        }

        Ok(installed)
    }

    fn eval_installed_formulae_receipts(&self) -> anyhow::Result<formula::receipt::Store> {
        let opt = self.prefix.join("opt").read_dir()?;

        let mut store = formula::receipt::Store::new();

        for entry in opt {
            let entry = entry?;
            let path = entry.path();

            let Some(name) = path.file_name() else {
                continue;
            };

            let name = name.to_string_lossy().to_string();

            if Self::is_dotfile(&name) {
                continue;
            }

            let receipt_path = path
                .canonicalize()?
                .join("INSTALL_RECEIPT.json");

            let mut file = File::open(receipt_path)?;
            let mut data = Vec::new();

            file.read_to_end(&mut data)?;

            let receipt: formula::receipt::Receipt = serde_json::from_slice(data.as_slice())?;

            store.insert(name.clone(), receipt);
        }

        Ok(store)
    }

    fn is_dotfile(name: &str) -> bool {
        name.starts_with('.')
    }

    pub fn eval_all(&self) -> anyhow::Result<State<formula::base::Store, cask::base::Store>> {
        let mut command = self.brew();

        let output = command
            .arg("info")
            .arg("--eval-all")
            .arg(Self::JSON_FLAG)
            .output()?;

        #[derive(Deserialize)]
        struct Result {
            formulae: Vec<formula::base::Formula>,
            casks: Vec<cask::base::Cask>,
        }

        let result: Result = serde_json::from_slice(output.stdout.as_slice())?;

        let formulae: formula::base::Store = result
            .formulae
            .into_iter()
            .map(|f| (f.name.clone(), f))
            .collect();

        let casks: cask::base::Store = result
            .casks
            .into_iter()
            .map(|c| (c.token.clone(), c))
            .collect();

        Ok(State {
            formulae,
            casks,
        })
    }
}