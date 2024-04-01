use bcrypt::{hash, verify, DEFAULT_COST};
use log::debug;
use sqlx::PgPool;

#[derive(sqlx::FromRow, Clone)]
struct LoginDbResult {
    id: i32,
    password: String,
    passcode: Option<String>,
}

#[derive(sqlx::FromRow, Clone)]
struct IdResult(i32);

pub(crate) enum LoginResult {
    Success(i32),
    MissingPasscode,
    InvalidCredentials,
    Blocked,
}

pub(crate) enum RegistrationResult {
    Success,
    UsernameTaken,
    DatabaseError,
}

pub(crate) enum SetGmResult {
    Success,
    DatabaseError,
    UserNotFound,
}

pub(crate) struct LoginProvider {
    pool: PgPool,
}

impl LoginProvider {
    pub(crate) fn new(pool: PgPool) -> Self {
        LoginProvider { pool }
    }

    pub async fn try_login(&self, username: &str, password: &str) -> LoginResult {
        let result: Option<LoginDbResult> = sqlx::query_as!(
            LoginDbResult,
            "SELECT id, password, passcode FROM users WHERE username = $1",
            username
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap();

        match result {
            Some(result) => {
                if !verify(password, &result.password).ok().unwrap_or(false) {
                    return LoginResult::InvalidCredentials;
                }

                if result.passcode.is_some() {
                    LoginResult::MissingPasscode
                } else {
                    LoginResult::Success(result.id)
                }
            },
            None => LoginResult::InvalidCredentials,
        }
    }

    pub async fn try_login_passcode(&self, username: &str, password: &str, passcode: &str) -> LoginResult {
        let result = sqlx::query!(
            "SELECT id, password FROM users WHERE username = $1 and passcode = $2",
            username,
            passcode
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap();

        match result {
            Some(r) => {
                if verify(password, &r.password).ok().unwrap_or(false) {
                    LoginResult::Success(r.id)
                } else {
                    LoginResult::InvalidCredentials
                }
            },
            None => LoginResult::InvalidCredentials,
        }
    }

    pub async fn register(&self, username: &str, password: &str, passcode: Option<&str>) -> RegistrationResult {
        let exists = sqlx::query!("SELECT ID FROM users WHERE username = $1", username)
            .fetch_optional(&self.pool)
            .await
            .expect("should be able to query existing usernames");

        if exists.is_some() {
            return RegistrationResult::UsernameTaken;
        }

        let password_hash = hash(password, DEFAULT_COST).expect("Should be able to hash password");
        let res = match passcode {
            Some(code) => sqlx::query!(
                "INSERT INTO users(username, password, passcode) values($1, $2, $3)",
                username,
                password_hash,
                code
            )
            .execute(&self.pool)
            .await
            .is_ok(),
            None => sqlx::query!(
                "INSERT INTO users(username, password) values($1, $2)",
                username,
                password_hash
            )
            .execute(&self.pool)
            .await
            .is_ok(),
        };

        return if res {
            RegistrationResult::Success
        } else {
            RegistrationResult::DatabaseError
        };
    }

    pub async fn set_gm(&self, character_name: &str, gm: bool) -> SetGmResult {
        let exists = sqlx::query!("SELECT id FROM characters WHERE charname = $1", character_name)
            .fetch_optional(&self.pool)
            .await
            .expect("should be able to query existing usernames");

        if exists.is_none() {
            return SetGmResult::UserNotFound;
        }

        let result = sqlx::query!("UPDATE characters SET gm = $1 WHERE charname = $2", gm, character_name)
            .execute(&self.pool)
            .await;

        match result {
            Ok(_) => SetGmResult::Success,
            Err(_) => SetGmResult::DatabaseError,
        }
    }
}
