use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::{validate_token, TokenError};

pub async fn auth_middleware(
    request: Request,
    next: Next,
    jwt_secret: &[u8],
) -> Result<Response, StatusCode> {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    validate_token(jwt_secret, token).map_err(|e| match e {
        TokenError::Expired => StatusCode::UNAUTHORIZED,
        TokenError::InvalidSignature => StatusCode::UNAUTHORIZED,
        TokenError::Malformed => StatusCode::BAD_REQUEST,
    })?;

    Ok(next.run(request).await)
}

pub fn require_auth(
    jwt_secret: Vec<u8>,
) -> impl Fn(
    Request,
    Next,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>,
> + Clone
       + Send {
    move |request: Request, next: Next| {
        let secret = jwt_secret.clone();
        Box::pin(async move { auth_middleware(request, next, &secret).await })
    }
}
