use rocket::{
    get,
    serde::json::{self, Value},
    State,
};

use crate::db::Db;

#[get("/leaderboard", format = "json")]
pub async fn leaderboard(db: &State<Db>) -> Value {
    let leaderboards = db.get_leaderboard().await;
    json::to_value(&leaderboards).unwrap()
}
