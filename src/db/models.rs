use crate::broker::evaluator::SMAConfig;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RecordId>,
    pub config: SMAConfig,
    pub symbol: String,
    pub gain: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
