use std::sync::Arc;

use axum::async_trait;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use serde::{Deserialize, Serialize};

use crate::app_error::UserRepoError;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

pub struct ExampleUserRepo;

#[async_trait]
impl UserRepo for ExampleUserRepo {
    async fn find(&self, _user_id: String) -> Result<User, UserRepoError> {
        unimplemented!()
    }

    async fn create(&self, _params: CreateUser) -> Result<User, UserRepoError> {
        unimplemented!()
    }
}

pub type DynUserRepo = Arc<dyn UserRepo + Send + Sync>;

#[async_trait]
pub trait UserRepo {
    async fn find(&self, user_id: String) -> Result<User, UserRepoError>;
    async fn create(&self, params: CreateUser) -> Result<User, UserRepoError>;
}

#[derive(Debug, Serialize)]
pub struct User {
    id: String,
    username: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateUser {
    username: String,
}
