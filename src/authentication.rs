use std::sync::Arc;

use crate::db::Db;
use serde::{Deserialize, Serialize};
use warp::reply::Json;

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthError {
    pub err: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthenticationData {
    pub username: String,
    pub password: String,
}

pub async fn register(authentication: AuthenticationData, db: Arc<Db>) -> String {
    if authentication.username.len() < 3 || authentication.username.len() > 15 {
        let error = AuthError {
            err: String::from("Username must be between 3 to 15 characters"),
        };
        return serde_json::to_string(&error).unwrap();
    }

    if authentication.password.len() < 3 || authentication.password.len() > 250 {
        let error = AuthError {
            err: String::from("Username must be between 3 to 250 characters"),
        };
        return serde_json::to_string(&error).unwrap();
    }

    match db
        .insert_user(&authentication.username, &authentication.password)
        .await
    {
        Ok(()) => serde_json::to_string(&AuthError { err: String::new() }).unwrap(),
        Err(error) => serde_json::to_string(&AuthError {
            err: error.to_string(),
        })
        .unwrap(),
    }
}

pub async fn login(authentication: AuthenticationData, db: Arc<Db>) -> Json {
    let user = match db
        .get_user_by_name_password(&authentication.username, &authentication.password)
        .await
    {
        Ok(user) => user,
        Err(error) => {
            return warp::reply::json(&AuthError {
                err: error.to_string(),
            })
        }
    };

    warp::reply::json(&user)
}
