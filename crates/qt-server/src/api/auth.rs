//! 认证 API

use axum::{
    extract::Json,
    Router,
    routing::post,
};
use serde::{Deserialize, Serialize};

/// 认证路由
pub fn router() -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
}

/// 注册请求
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: String,
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub refresh_token: String,
}

/// 注册
async fn register(Json(_req): Json<RegisterRequest>) -> Json<()> {
    // TODO: 实现注册逻辑
    Json(())
}

/// 登录
async fn login(Json(_req): Json<LoginRequest>) -> Json<LoginResponse> {
    // TODO: 实现登录逻辑
    Json(LoginResponse {
        token: "dummy_token".to_string(),
        refresh_token: "dummy_refresh_token".to_string(),
    })
}

/// 刷新 token
async fn refresh_token() -> Json<LoginResponse> {
    // TODO: 实现刷新逻辑
    Json(LoginResponse {
        token: "dummy_token".to_string(),
        refresh_token: "dummy_refresh_token".to_string(),
    })
}

/// 登出
async fn logout() -> Json<()> {
    Json(())
}