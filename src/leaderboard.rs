use crate::db::Db;
use std::sync::Arc;

pub async fn leaderboard(db: Arc<Db>) -> String {
    let leaderboards = db.get_leaderboard().await;
    serde_json::to_string(&leaderboards).unwrap()
}
