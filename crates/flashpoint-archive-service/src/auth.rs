use std::net::SocketAddr;

use axum::{
    async_trait,
    extract::{ConnectInfo, FromRef, FromRequestParts, Query, Request, State},
    http::{request::Parts, HeaderMap, HeaderValue},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use axum_extra::{headers::Cookie, TypedHeader};
use chrono::{Days, NaiveDateTime, Utc};
use flashpoint_archive::game::{search::SearchParam, TagVec};
use oauth2::{
    reqwest::async_http_client, AccessToken, AuthorizationCode, CsrfToken, Scope, TokenResponse,
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use reqwest::Client;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{error::AppError, AppState};

#[derive(Deserialize)]
struct OauthProfileFpfss {
    #[serde(rename = "UserID")]
    id: i64,
    #[serde(rename = "Username")]
    name: String,
    #[serde(rename = "AvatarURL")]
    avatar_url: String,
    #[serde(rename = "UserRoles")]
    pub roles: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub avatar_url: String,
    pub roles: TagVec,
    pub name: String,
}

pub(crate) struct TokenData {
    user_id: String,
    session_id: String,
    ip_addr: String,
    created_at: NaiveDateTime,
    expires_at: NaiveDateTime,
}

// Start the OAuth flow
pub(crate) async fn start_oauth(State(state): State<AppState>) -> impl IntoResponse {
    let provider = state.config.oauth_provider.as_str();
    let scopes_str = match provider {
        "fpfss" => vec!["identity".to_owned()],
        _ => vec![],
    };
    let scopes = scopes_str
        .iter()
        .map(|s| Scope::new(s.clone()))
        .collect::<Vec<Scope>>();
    let (auth_url, _csrf_state) = state
        .client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(scopes)
        .url();
    Redirect::temporary(auth_url.as_str())
}

#[derive(Deserialize)]
pub struct OauthCallback {
    code: String,
}

// Handle the OAuth callback
pub(crate) async fn handle_oauth_callback(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(query): Query<OauthCallback>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Redirect), AppError> {
    let code = AuthorizationCode::new(query.code);
    let token_response = state
        .client
        .exchange_code(code)
        .request_async(async_http_client)
        .await
        .map_err(|e| AppError::AuthError(format!("Failed to exchange token: {}", e)))?;

    // Extract user info from the token and provider
    let user_info = get_user_info(
        &state.config.oauth_provider,
        &state.config.oauth_profile_url,
        token_response.access_token(),
    )
    .await?;

    // Save user and session info to the database
    {
        save_user_to_db(&state.auth_pool, &user_info).await?;
    }

    let ip_addr = headers
        .get("X-Forwarded-For")
        .and_then(|header| header.to_str().ok())
        .or_else(|| {
            headers
                .get("X-Real-IP")
                .and_then(|header| header.to_str().ok())
        })
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());
    let token_data = TokenData {
        user_id: user_info.id,
        session_id: generate_session_id(),
        ip_addr: ip_addr.to_owned(),
        created_at: Utc::now().naive_utc(),
        expires_at: Utc::now()
            .checked_add_days(Days::new(14))
            .unwrap()
            .naive_utc(),
    };

    {
        save_session_to_db(&state.auth_pool, &token_data).await?;
    }

    let cookie = format!(
        "session_id={}; HttpOnly; Path=/api; Secure; SameSite=Strict",
        &token_data.session_id,
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        HeaderValue::from_str(&cookie).unwrap(),
    );

    Ok((headers, Redirect::temporary("/")))
}

async fn get_user_info(
    provider: &str,
    profile_url: &str,
    token: &AccessToken,
) -> Result<UserInfo, AppError> {
    let client = Client::new();
    let res = client
        .get(profile_url)
        .bearer_auth(token.secret().clone())
        .send()
        .await
        .map_err(|_| AppError::AuthError("Failed to request user info from provider".to_owned()))?;

    if !res.status().is_success() {
        return Err(AppError::AuthError(
            "Failed to get user info form provider".to_owned(),
        ));
    }

    match provider {
        "fpfss" => {
            let fpfss_data = res
                .json::<OauthProfileFpfss>()
                .await
                .map_err(|_| AppError::AuthError("Failed to parse FPFSS auth info".to_owned()))?;
            println!("roles: {:?}", fpfss_data.roles);
            Ok(UserInfo {
                id: fpfss_data.id.to_string(),
                avatar_url: fpfss_data.avatar_url,
                roles: fpfss_data.roles.into(),
                name: fpfss_data.name,
            })
        }
        _ => Err(AppError::AuthError(
            "Invalid auth provider in config".to_owned(),
        )),
    }
}

fn generate_session_id() -> String {
    // Replace this with a proper UUID generator
    uuid::Uuid::new_v4().to_string()
}

async fn save_user_to_db(
    db: &RwLock<Pool<SqliteConnectionManager>>,
    user_info: &UserInfo,
) -> Result<(), AppError> {
    let conn = db.write().await.get().unwrap();
    // Check if the user already exists
    let mut stmt = conn
        .prepare("SELECT id FROM users WHERE id = ?1")
        .map_err(|_| AppError::AuthError("Failed to prep query".to_owned()))?;
    let user_id: Option<String> = stmt
        .query_row(params![&user_info.id], |row| row.get(0))
        .optional()
        .map_err(|e| AppError::AuthError(format!("Failed to check for existing user: {}", e)))?;

    let params = vec![
        SearchParam::String(user_info.id.clone()),
        SearchParam::String(user_info.name.clone()),
        SearchParam::String(user_info.avatar_url.clone()),
        SearchParam::String(user_info.roles.join("; ")),
    ];
    let params_as_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    if let Some(_) = user_id {
        conn.execute(
            "UPDATE users SET name = ?2, avatar_url = ?3, roles = ?4 WHERE id = ?1",
            params_as_refs.as_slice(),
        )
        .map_err(|_| AppError::AuthError("Failed to update existing user".to_owned()))?;
        Ok(())
    } else {
        // Insert a new user
        conn.execute(
            "INSERT INTO users (id, name, avatar_url, roles) VALUES (?1, ?2, ?3, ?4)",
            params_as_refs.as_slice(),
        )
        .map_err(|_| AppError::AuthError("Failed to create new user".to_owned()))?;

        Ok(())
    }
}

async fn save_session_to_db(
    db: &RwLock<Pool<SqliteConnectionManager>>,
    token_data: &TokenData,
) -> Result<(), AppError> {
    let conn = db.write().await.get().unwrap();
    // Insert a new session
    conn.execute(
        "INSERT INTO sessions (user_id, session_id, ip_addr, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            &token_data.user_id,
            &token_data.session_id,
            &token_data.ip_addr,
            &token_data.created_at,
            &token_data.expires_at,
        ],
    ).map_err(|e| AppError::AuthError(format!("Failed to create new session {}", e)))?;

    Ok(())
}

async fn get_user_info_from_session(
    db: &RwLock<Pool<SqliteConnectionManager>>,
    session_id: &str,
) -> Result<UserInfo, AppError> {
    let conn = db.read().await.get().unwrap();

    let mut stmt = conn
        .prepare(
            "SELECT id, name, avatar_url, roles FROM users WHERE id = (
            SELECT user_id FROM sessions WHERE session_id = ?1
        )",
        )
        .map_err(|_| AppError::AuthError("Failed to create query".to_owned()))?;
    let user = stmt
        .query_row(params![session_id], |row| {
            Ok(UserInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                avatar_url: row.get(2)?,
                roles: row.get(3)?,
            })
        })
        .optional()
        .map_err(|e| AppError::AuthError(format!("Failed to search for user: {}", e)))?;

    match user {
        Some(user) => Ok(user),
        None => Err(AppError::Unauthorized),
    }
}

pub async fn get_profile(user: UserInfo) -> Json<UserInfo> {
    Json(user)
}

#[async_trait]
impl<S> FromRequestParts<S> for UserInfo
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cookie = TypedHeader::<Cookie>::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::Unauthorized)?;

        let state = AppState::from_ref(state);

        if let Some(session_id) = cookie.get("session_id") {
            let user = get_user_info_from_session(&state.auth_pool, session_id).await?;
            Ok(user)
        } else {
            Err(AppError::Unauthorized)
        }
    }
}

pub async fn is_admin_middleware(
    user: UserInfo,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    match user.roles.iter().any(|role| role == "Administrator") {
        true => {
            let response = next.run(req).await;
            Ok(response)
        }
        false => Err(AppError::Forbidden),
    }
}
