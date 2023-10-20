use crate::db::Db;
use rocket::{
    post,
    serde::json::{self, json, Json, Value},
    State,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthenticationData {
    username: String,
    password: String,
}

#[post("/register", format = "json", data = "<authentication>")]
pub async fn register(db: &State<Db>, authentication: Json<AuthenticationData>) -> Value {
    if authentication.username.len() < 3 || authentication.username.len() > 15 {
        return json!({"err":"Username must be between 3 to 15 characters"});
    }

    if authentication.password.len() < 3 || authentication.password.len() > 250 {
        return json!({"err":"Username must be between 3 to 250 characters"});
    }

    match db
        .insert_user(&authentication.username, &authentication.password)
        .await
    {
        Ok(()) => json!({"err":""}),
        Err(error) => json!({"err":error.to_string()}),
    }
}

#[post("/login", format = "json", data = "<authentication>")]
pub async fn login(db: &State<Db>, authentication: Json<AuthenticationData>) -> Value {
    let user = match db
        .get_user_by_name_password(&authentication.username, &authentication.password)
        .await
    {
        Ok(user) => user,
        Err(error) => return json!({"err": error.to_string()}),
    };

    json::to_value(user).unwrap()
}
