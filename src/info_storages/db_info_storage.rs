use std::time::Duration;

use async_trait::async_trait;
use rbatis::crud::CRUD;
use rbatis::crud_table;
use rbatis::db::DBPoolOptions;
use rbatis::executor::Executor;
use rbatis::rbatis::Rbatis;

use crate::errors::{RustusError, RustusResult};
use crate::info_storages::{FileInfo, InfoStorage};
use crate::RustusConf;

#[crud_table]
struct DbModel {
    pub id: String,
    pub info: String,
}

impl TryFrom<&FileInfo> for DbModel {
    type Error = RustusError;

    fn try_from(value: &FileInfo) -> Result<Self, Self::Error> {
        Ok(DbModel {
            id: value.id.clone(),
            info: serde_json::to_string(value)?,
        })
    }
}

pub struct DBInfoStorage {
    db: Rbatis,
}

impl DBInfoStorage {
    pub async fn new(app_conf: RustusConf) -> RustusResult<Self> {
        let db = Rbatis::new();
        let mut opts = DBPoolOptions::new();
        opts.connect_timeout = Duration::new(2, 0);
        db.link_opt(
            app_conf.info_storage_opts.info_db_dsn.unwrap().as_str(),
            opts,
        )
        .await?;
        Ok(Self { db })
    }
}

#[async_trait]
impl InfoStorage for DBInfoStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        self.db
            .exec(
                "CREATE TABLE IF NOT EXISTS db_model (id VARCHAR(40) PRIMARY KEY, info TEXT);",
                Vec::new(),
            )
            .await?;
        Ok(())
    }

    async fn set_info(&self, file_info: &FileInfo, create: bool) -> RustusResult<()> {
        let model = DbModel::try_from(file_info)?;
        if create {
            self.db.save(&model, &[]).await?;
        } else {
            self.db.update_by_column("id", &model).await?;
        }
        Ok(())
    }

    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo> {
        let model: Option<DbModel> = self.db.fetch_by_column("id", file_id).await?;
        if let Some(info) = model {
            serde_json::from_str(info.info.as_str()).map_err(RustusError::from)
        } else {
            Err(RustusError::FileNotFound)
        }
    }

    async fn remove_info(&self, file_id: &str) -> RustusResult<()> {
        self.db
            .remove_by_column::<DbModel, &str>("id", file_id)
            .await?;
        Ok(())
    }
}
