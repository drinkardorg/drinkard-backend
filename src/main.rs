use std::sync::Arc;

use db::Db;
use game::State;
use tokio::sync::Mutex;
use warp::Filter;

pub mod authentication;
pub mod db;
pub mod game;
pub mod leaderboard;

#[tokio::main]
async fn main() {
    let db = Arc::new(Db::new().await);
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(["POST", "PATCH", "PUT", "DELETE", "HEAD", "OPTIONS", "GET"])
        .allow_credentials(true)
        .allow_headers([
            "Host",
            "Accept",
            "User-Agent",
            "Content-Type",
            "Content-Length",
            "Access-Control-Request-Method",
            "Access-Control-Request-Headers",
        ])
        .build();

    let db_cloned: Arc<Db> = db.clone();
    let register_route = warp::path("register")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .and(warp::any().map(move || db_cloned.clone()))
        .then(authentication::register);

    let db_cloned: Arc<Db> = db.clone();
    let login_route = warp::path("login")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .and(warp::any().map(move || db_cloned.clone()))
        .then(authentication::login);

    let db_cloned = db.clone();
    let leaderboard_route = warp::path("leaderboard")
        .and(warp::get())
        .and(warp::any().map(move || db_cloned.clone()))
        .then(leaderboard::leaderboard);

    let state = Arc::new(Mutex::new(State::new()));
    let db_cloned = db.clone();
    let state_cloned = state.clone();
    let game_route = warp::path("game")
        .and(warp::any().map(move || db_cloned.clone()))
        .and(warp::any().map(move || state_cloned.clone()))
        .and(warp::ws())
        .map(game::game);

    std::env::set_var("RUST_LOG", "backend=info");
    std::env::set_var("RUST_APP_LOG", "info");
    pretty_env_logger::init_custom_env("RUST_APP_LOG");
    log::info!("Drinkard Backend");

    let routes = register_route
        .or(login_route)
        .or(leaderboard_route)
        .or(game_route)
        .with(cors)
        .with(warp::log("backend"));

    warp::serve(routes).run(([0, 0, 0, 0], 8000)).await;
}
