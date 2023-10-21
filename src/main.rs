use db::Db;
use rocket::{launch, routes};

pub mod authentication;
pub mod cors;
pub mod db;
pub mod leaderboard;

#[launch]
async fn rocket() -> _ {
    let db = Db::new().await;

    rocket::build().attach(cors::Cors).manage(db).mount(
        "/",
        routes![
            cors::all_options,
            leaderboard::leaderboard,
            authentication::register,
            authentication::login
        ],
    )
}
