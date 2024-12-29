use std::time::Duration;

use chrono::Utc;
use derive_builder::Builder;

use brewer_core::{models, Brew};
use log::info;

use crate::store::Store;

pub mod store;

pub type State = models::State<models::formula::State, models::cask::State>;

#[derive(Builder)]
pub struct Engine {
    store: Store,

    #[builder(default)]
    brew: Brew,

    /// How often cache should expire. None means never
    cache_duration: Option<Duration>,
}

impl Engine {
    pub fn new(store: Store, brew: Brew) -> Engine {
        Engine {
            store,
            brew,
            cache_duration: None,
        }
    }

    pub fn install(&self, kegs: Vec<models::Keg>) -> anyhow::Result<()> {
        self.brew.install(kegs)?;

        Ok(())
    }

    pub fn uninstall(&self, kegs: Vec<models::Keg>) -> anyhow::Result<()> {
        self.brew.uninstall(kegs)?;

        Ok(())
    }

    pub fn cache_or_latest(&mut self) -> anyhow::Result<State> {
        let cache = self.cache()?;

        if self.cache_expired()? || cache.is_none() {
            info!("updating the cache, this will take some time");

            let latest = self.fetch_latest()?;

            self.update_cache(&latest)?;

            Ok(latest)
        } else {
            Ok(cache.unwrap())
        }
    }

    pub fn cache(&self) -> anyhow::Result<Option<State>> {
        let Some(all) = self.store.get_state()? else {
            return Ok(None);
        };

        let installed = self.brew.installed(&all)?;

        let state = State {
            formulae: models::formula::State {
                all: all.formulae,
                installed: installed.formulae,
            },
            casks: models::cask::State {
                all: all.casks,
                installed: installed.casks,
            },
        };

        Ok(Some(state))
    }

    pub fn cache_expired(&self) -> anyhow::Result<bool> {
        let Some(cache_duration) = self.cache_duration else {
            return Ok(false);
        };

        let last_update = self.store.last_update()?;

        match last_update {
            Some(last_update) => {
                let now = Utc::now().naive_utc();

                Ok(last_update + cache_duration <= now)
            }
            None => Ok(true),
        }
    }

    pub fn update_cache(&mut self, state: &State) -> anyhow::Result<()> {
        self.store.set_state(store::State {
            formulae: state.formulae.all.clone(),
            casks: state.casks.all.clone(),
        })?;

        Ok(())
    }

    pub fn fetch_latest(&self) -> anyhow::Result<State> {
        let state = self.brew.state()?;

        Ok(state)
    }
}

