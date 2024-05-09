use std::path::Path;

use chrono::{NaiveDateTime, Utc};
use jammdb::Tx;

use brewer_core::models;

#[derive(Clone)]
pub struct Store {
    db: jammdb::DB,
}

pub type State = models::State<models::formula::Store, models::cask::Store>;

impl Store {
    const META_BUCKET: &'static str = "meta";
    const STATE_BUCKET: &'static str = "state";

    const LAST_UPDATE_KEY: &'static str = "last-update";
    const STATE_KEY: &'static str = "state";

    pub fn open(path: &Path) -> anyhow::Result<Store> {
        Ok(Store {
            db: jammdb::DB::open(path)?
        })
    }

    pub fn last_update(&self) -> anyhow::Result<Option<NaiveDateTime>> {
        let tx = self.db.tx(false)?;

        match tx.get_bucket(Self::META_BUCKET) {
            Ok(bucket) => {
                let Some(data) = bucket.get(Self::LAST_UPDATE_KEY) else {
                    return Ok(None);
                };

                let datetime: NaiveDateTime = rmp_serde::from_slice(data.kv().value())?;


                Ok(Some(datetime))
            }
            Err(jammdb::Error::BucketMissing) => Ok(None),
            Err(e) => Err(anyhow::anyhow!(e))
        }
    }

    fn commit_update(tx: Tx) -> anyhow::Result<()> {
        let bucket = tx.get_or_create_bucket(Self::META_BUCKET)?;

        let now = Utc::now().naive_utc();
        let now_bytes = rmp_serde::to_vec(&now)?;

        bucket.put(Self::LAST_UPDATE_KEY, now_bytes)?;

        tx.commit()?;

        Ok(())
    }

    pub fn get_state(&self) -> anyhow::Result<Option<State>> {
        let tx = self.db.tx(false)?;

        match tx.get_bucket(Self::STATE_BUCKET) {
            Ok(bucket) => {
                let Some(data) = bucket.get(Self::STATE_KEY) else {
                    return Ok(None);
                };

                let state: State = rmp_serde::from_slice(data.kv().value())?;

                Ok(Some(state))
            }
            Err(jammdb::Error::BucketMissing) => Ok(None),
            Err(e) => Err(anyhow::anyhow!(e))
        }
    }

    pub fn set_state(&mut self, state: State) -> anyhow::Result<()> {
        let tx = self.db.tx(true)?;

        let bucket = tx.get_or_create_bucket(Self::STATE_BUCKET)?;

        let state_bytes = rmp_serde::to_vec(&state)?;

        bucket.put(Self::STATE_KEY, state_bytes)?;

        Self::commit_update(tx)?;

        Ok(())
    }
}