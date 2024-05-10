use std::time::Duration;

use chrono::Utc;
use derive_builder::Builder;

use brewer_core::{Brew, models};

use crate::store::Store;

pub mod store;

pub type State = models::State<models::formula::State, models::cask::State>;

#[derive(Builder)]
pub struct Engine {
    store: Store,

    #[builder(default)]
    brew: Brew,

    #[builder(default = "Duration::from_secs(60 * 24 * 12)")]
    cache_duration: Duration,
}

impl Engine {
    pub fn new(store: Store, brew: Brew) -> Engine {
        Engine {
            store,
            brew,
            cache_duration: Duration::from_secs(60 * 24 * 12),
        }
    }

    pub fn cache_or_latest(&mut self) -> anyhow::Result<State> {
        let cache = self.cache()?;

        if self.cache_expired()? || cache.is_none() {
            // TODO: replace this logger
            println!("Updating the cache, this will take some time...");

            let latest = self.latest()?;

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

        let installed = self.brew.eval_installed(&all)?;

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
        let last_update = self.store.last_update()?;

        match last_update {
            Some(last_update) => {
                let now = Utc::now().naive_utc();

                Ok(last_update + self.cache_duration <= now)
            }
            None => Ok(true)
        }
    }

    pub fn update_cache(&mut self, state: &State) -> anyhow::Result<()> {
        self.store.set_state(store::State {
            formulae: state.formulae.all.clone(),
            casks: state.casks.all.clone(),
        })?;

        Ok(())
    }

    pub fn latest(&self) -> anyhow::Result<State> {
        let state = self.brew.state()?;

        Ok(state)
    }
}