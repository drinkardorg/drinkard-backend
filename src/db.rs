use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    #[sqlx(rename = "ID")]
    pub id: i32,

    #[sqlx(rename = "Username")]
    pub username: String,

    #[sqlx(rename = "Password")]
    pub password: String,

    #[sqlx(rename = "EloPoints")]
    pub elo_points: i32,

    #[sqlx(rename = "CountryID")]
    pub country_id: String,

    #[sqlx(rename = "ProfilePictureURL")]
    pub profile_picture_url: String,
}

pub struct Db {
    pub pool: MySqlPool,
}

impl Db {
    pub async fn new() -> Self {
        dotenv::dotenv().ok();

        let pool = MySqlPool::connect(&dotenv::var("DATABASE_URL").unwrap())
            .await
            .unwrap();

        Self::run_migrations(&pool).await;

        Self { pool }
    }

    pub async fn run_migrations(pool: &MySqlPool) {
        sqlx::migrate!("db/migrations").run(pool).await.unwrap()
    }

    pub async fn insert_user(&self, username: &str, password: &str) -> anyhow::Result<()> {
        let mut transaction = self.pool.begin().await?;
        let result: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM User WHERE Username = ?)")
                .bind(username)
                .fetch_one(&mut *transaction)
                .await?;

        if result.0 {
            return Err(anyhow::Error::msg("Username already exists!"));
        }

        const QUERY: &str = "
            INSERT INTO User(Username, Password) VALUES(?, ?)
        ";

        sqlx::query(QUERY)
            .bind(username)
            .bind(password)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;

        Ok(())
    }

    pub async fn get_user_by_name_password(
        &self,
        username: &str,
        password: &str,
    ) -> anyhow::Result<User> {
        let mut transaction = self.pool.begin().await?;
        let result: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM User WHERE Username = ?)")
                .bind(username)
                .fetch_one(&mut *transaction)
                .await
                .unwrap();

        if !result.0 {
            return Err(anyhow::Error::msg("Invalid username or password"));
        }

        const QUERY: &str = "
            SELECT * FROM User WHERE Username = ? AND Password = ?
        ";

        let user = sqlx::query_as::<_, User>(QUERY)
            .bind(username)
            .bind(password)
            .fetch_one(&mut *transaction)
            .await?;
        Ok(user)
    }
}
