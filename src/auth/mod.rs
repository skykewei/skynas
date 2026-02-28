use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::server::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
}

/// 创建 JWT Token，有效期 24 小时
pub fn create_token(secret: &str) -> anyhow::Result<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: "admin".to_string(),
        iat: now.timestamp(),
        exp: (now + Duration::hours(24)).timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

/// 验证 JWT Token
pub fn verify_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(token_data.claims)
}

/// JWT 认证中间件
pub async fn require_admin_auth(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 获取 JWT secret
    let secret = {
        let db = state.db.lock().await;
        db.get_or_create_admin_config("change-me-in-production")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .jwt_secret
    };

    match verify_token(auth.token(), &secret) {
        Ok(claims) if claims.sub == "admin" => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
