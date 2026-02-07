use crate::db::models::RunResult;
use crate::settings::Settings;
use surrealdb::engine::any::{self, Any};
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::RecordId;
use surrealdb::Surreal;

pub struct Db {
    client: Surreal<Any>,
}

impl Db {
    pub async fn new() -> Result<Self, surrealdb::Error> {
        let settings = Settings::new().unwrap();

        // Open a connection
        let db = any::connect(settings.surreal_db_url.clone()).await?;
        db.use_ns("trader").use_db("results").await?;
        db.signin(Root {
            username: &settings.surreal_db_user,
            password: &settings.surreal_db_pass,
        })
        .await?;

        //client.use_ns("trader").use_db("results").await?;

        Ok(Self { client: db })
    }

    pub async fn init_schema(&self) -> Result<(), surrealdb::Error> {
        self.client
            .query(
                "DEFINE TABLE run_results SCHEMAFULL;
                 DEFINE FIELD config ON TABLE run_results TYPE object;
                 DEFINE FIELD config.long_range ON TABLE run_results TYPE int;
                 DEFINE FIELD config.short_range ON TABLE run_results TYPE int;
                 DEFINE FIELD symbol ON TABLE run_results TYPE string;
                 DEFINE FIELD gain ON TABLE run_results TYPE float;
                 DEFINE FIELD timestamp ON TABLE run_results TYPE string;",
            )
            .await?;
        Ok(())
    }

    pub async fn delete_schema(&self) -> Result<(), surrealdb::Error> {
        self.client.query("REMOVE TABLE run_results").await?;
        Ok(())
    }

    pub async fn add_result(
        &self,
        result: RunResult,
    ) -> Result<Option<RunResult>, surrealdb::Error> {
        let created: Option<RunResult> = self.client.create("run_results").content(result).await?;
        Ok(created)
    }

    pub async fn list_results(&self) -> Result<Vec<RunResult>, surrealdb::Error> {
        let results: Vec<RunResult> = self.client.select("run_results").await?;
        Ok(results)
    }

    pub async fn delete_result(&self, id: RecordId) -> Result<(), surrealdb::Error> {
        let _: Option<RunResult> = self.client.delete(&id).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::evaluator::SMAConfig;

    #[tokio::test]
    //#[ignore] // Requires network connection to SurrealDB
    async fn test_init_schema() {
        let db = Db::new().await.expect("Failed to connect to DB");

        // Clean up first in case schema exists
        let _ = db.delete_schema().await;

        // Initialize schema
        db.init_schema().await.expect("Failed to init schema");

        // Verify we can create a record (schema exists and is valid)
        let config = SMAConfig {
            long_range: 10,
            short_range: 3,
        };
        let res = RunResult {
            id: None,
            config,
            symbol: "TEST".to_string(),
            gain: 50.0,
            timestamp: chrono::Utc::now(),
        };

        let created = db
            .add_result(res)
            .await
            .expect("Failed to add result with new schema");
        assert!(created.is_some());

        // Clean up
        db.delete_schema().await.expect("Failed to delete schema");
    }

    #[tokio::test]
    #[ignore] // Requires network connection to SurrealDB
    async fn test_delete_schema() {
        let db = Db::new().await.expect("Failed to connect to DB");

        // Initialize schema first
        db.init_schema().await.expect("Failed to init schema");

        // Delete schema
        db.delete_schema().await.expect("Failed to delete schema");

        // Verify schema is deleted by trying to add a result (should fail or return empty)
        let config = SMAConfig {
            long_range: 10,
            short_range: 3,
        };
        let res = RunResult {
            id: None,
            config,
            symbol: "TEST".to_string(),
            gain: 50.0,
            timestamp: chrono::Utc::now(),
        };

        // This should fail because the table doesn't exist
        let result = db.add_result(res).await;
        // We expect this to either fail or return None since table doesn't exist
        assert!(result.is_err() || result.unwrap().is_none());
    }

    #[tokio::test]
    #[ignore] // Requires network connection to SurrealDB
    async fn test_crud_cycle() {
        let db = Db::new().await.expect("Failed to connect to DB");

        // Clean up and initialize
        let _ = db.delete_schema().await;
        db.init_schema().await.expect("Failed to init schema");

        let config = SMAConfig {
            long_range: 20,
            short_range: 5,
        };
        let res = RunResult {
            id: None,
            config,
            symbol: "TEST".to_string(),
            gain: 123.45,
            timestamp: chrono::Utc::now(),
        };

        let created = db.add_result(res).await.expect("Failed to add result");
        assert!(created.is_some());
        let created = created.unwrap();
        assert!(created.id.is_some());
        let id = created.id.unwrap();

        let list = db.list_results().await.expect("Failed to list results");
        assert!(list.iter().any(|r| r.id == Some(id.clone())));

        db.delete_result(id.clone())
            .await
            .expect("Failed to delete result");

        let list_after = db
            .list_results()
            .await
            .expect("Failed to list results after delete");
        assert!(!list_after.iter().any(|r| r.id == Some(id.clone())));

        // Clean up
        db.delete_schema().await.expect("Failed to delete schema");
    }
}
