use axum::{
    extract::{Extension, Path},
    Json,
};

use crate::{
    app_error::AppError,
    app_model::{CreateUser, DynUserRepo, User},
};

pub async fn users_show(
    Path(user_id): Path<String>,
    Extension(user_repo): Extension<DynUserRepo>,
) -> Result<Json<User>, AppError> {
    let user = user_repo.find(user_id).await?;

    Ok(user.into())
}

pub async fn users_create(
    Json(params): Json<CreateUser>,
    Extension(user_repo): Extension<DynUserRepo>,
) -> Result<Json<User>, AppError> {
    let user = user_repo.create(params).await?;

    Ok(user.into())
}
