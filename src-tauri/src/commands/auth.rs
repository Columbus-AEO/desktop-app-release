use crate::{storage, AppState, AuthState, PersistedAuth, User, SUPABASE_ANON_KEY, SUPABASE_URL};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[derive(Deserialize)]
struct LoginResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    user: SupabaseUser,
}

#[derive(Deserialize)]
struct SupabaseUser {
    id: String,
    email: String,
}

#[derive(Serialize)]
pub struct AuthStatusResponse {
    pub authenticated: bool,
    pub user: Option<User>,
}

#[tauri::command]
pub async fn login(
    email: String,
    password: String,
    state: State<'_, Arc<AppState>>,
) -> Result<User, String> {
    let client = reqwest::Client::new();

    let url = format!("{}/auth/v1/token?grant_type=password", SUPABASE_URL);

    let response = client
        .post(&url)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "email": email,
            "password": password
        }))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Login failed: {}", error_text));
    }

    let login_data: LoginResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let user = User {
        id: login_data.user.id,
        email: login_data.user.email,
    };

    let expires_at = chrono::Utc::now().timestamp() + login_data.expires_in;

    // Store auth state
    {
        let mut auth = state.auth.lock();
        auth.access_token = Some(login_data.access_token.clone());
        auth.refresh_token = Some(login_data.refresh_token.clone());
        auth.user = Some(user.clone());
        auth.expires_at = Some(expires_at);
    }

    // Persist auth to disk
    let persisted_auth = PersistedAuth {
        access_token: login_data.access_token,
        refresh_token: login_data.refresh_token,
        user_id: user.id.clone(),
        user_email: user.email.clone(),
        expires_at,
    };
    if let Err(e) = storage::update_auth(Some(persisted_auth)) {
        eprintln!("[Auth] Failed to persist auth: {}", e);
    }

    Ok(user)
}

#[tauri::command]
pub async fn logout(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut auth = state.auth.lock();
    *auth = AuthState::default();

    // Clear persisted auth
    if let Err(e) = storage::clear_auth() {
        eprintln!("[Auth] Failed to clear persisted auth: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub async fn get_auth_status(state: State<'_, Arc<AppState>>) -> Result<AuthStatusResponse, String> {
    // First check if we have auth data
    let (user, expires_at, refresh_token) = {
        let auth = state.auth.lock();
        (
            auth.user.clone(),
            auth.expires_at,
            auth.refresh_token.clone(),
        )
    };

    if let Some(ref user) = user {
        // Check if token is expired or about to expire (within 5 minutes)
        if let Some(expires_at) = expires_at {
            let now = chrono::Utc::now().timestamp();
            let is_expired = now >= expires_at - 300; // 5 minute buffer

            if is_expired {
                // Try to refresh the token
                if let Some(refresh_token) = refresh_token {
                    println!("[Auth] Token expired, attempting refresh...");
                    match refresh_access_token(&refresh_token).await {
                        Ok((new_access, new_refresh, new_expires_in)) => {
                            let new_expires_at = chrono::Utc::now().timestamp() + new_expires_in;

                            // Update state
                            {
                                let mut auth = state.auth.lock();
                                auth.access_token = Some(new_access.clone());
                                auth.refresh_token = Some(new_refresh.clone());
                                auth.expires_at = Some(new_expires_at);
                            }

                            // Persist updated tokens
                            let persisted_auth = PersistedAuth {
                                access_token: new_access,
                                refresh_token: new_refresh,
                                user_id: user.id.clone(),
                                user_email: user.email.clone(),
                                expires_at: new_expires_at,
                            };
                            if let Err(e) = storage::update_auth(Some(persisted_auth)) {
                                eprintln!("[Auth] Failed to persist refreshed auth: {}", e);
                            }

                            println!("[Auth] Token refreshed successfully");
                            return Ok(AuthStatusResponse {
                                authenticated: true,
                                user: Some(user.clone()),
                            });
                        }
                        Err(e) => {
                            eprintln!("[Auth] Token refresh failed: {}", e);
                            // Clear invalid auth state
                            {
                                let mut auth = state.auth.lock();
                                *auth = AuthState::default();
                            }
                            let _ = storage::clear_auth();
                            return Ok(AuthStatusResponse {
                                authenticated: false,
                                user: None,
                            });
                        }
                    }
                } else {
                    // No refresh token, can't refresh
                    return Ok(AuthStatusResponse {
                        authenticated: false,
                        user: None,
                    });
                }
            }
        }

        Ok(AuthStatusResponse {
            authenticated: true,
            user: Some(user.clone()),
        })
    } else {
        Ok(AuthStatusResponse {
            authenticated: false,
            user: None,
        })
    }
}

/// Refresh an expired access token using the refresh token
pub async fn refresh_access_token(refresh_token: &str) -> Result<(String, String, i64), String> {
    let client = reqwest::Client::new();

    let url = format!("{}/auth/v1/token?grant_type=refresh_token", SUPABASE_URL);

    let response = client
        .post(&url)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "refresh_token": refresh_token
        }))
        .send()
        .await
        .map_err(|e| format!("Network error during refresh: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Token refresh failed: {}", error_text));
    }

    let refresh_data: LoginResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error during refresh: {}", e))?;

    Ok((
        refresh_data.access_token,
        refresh_data.refresh_token,
        refresh_data.expires_in,
    ))
}

/// Beautiful HTML success page for OAuth completion
fn get_success_page_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Login Successful - Columbus</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }
        .container {
            background: white;
            border-radius: 24px;
            padding: 48px;
            text-align: center;
            box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
            max-width: 420px;
            width: 100%;
        }
        .checkmark-circle {
            width: 80px;
            height: 80px;
            border-radius: 50%;
            background: linear-gradient(135deg, #10b981 0%, #059669 100%);
            margin: 0 auto 24px;
            display: flex;
            align-items: center;
            justify-content: center;
            animation: scaleIn 0.5s ease-out;
        }
        @keyframes scaleIn {
            0% { transform: scale(0); opacity: 0; }
            50% { transform: scale(1.1); }
            100% { transform: scale(1); opacity: 1; }
        }
        .checkmark {
            width: 40px;
            height: 40px;
            stroke: white;
            stroke-width: 3;
            fill: none;
            animation: drawCheck 0.6s ease-out 0.3s forwards;
            stroke-dasharray: 50;
            stroke-dashoffset: 50;
        }
        @keyframes drawCheck {
            to { stroke-dashoffset: 0; }
        }
        h1 {
            color: #1f2937;
            font-size: 28px;
            font-weight: 700;
            margin-bottom: 12px;
        }
        p {
            color: #6b7280;
            font-size: 16px;
            line-height: 1.6;
            margin-bottom: 32px;
        }
        .brand {
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 8px;
            color: #9ca3af;
            font-size: 14px;
        }
        .brand-icon {
            width: 20px;
            height: 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            border-radius: 4px;
        }
        .close-hint {
            margin-top: 24px;
            padding-top: 24px;
            border-top: 1px solid #e5e7eb;
            color: #9ca3af;
            font-size: 13px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="checkmark-circle">
            <svg class="checkmark" viewBox="0 0 24 24">
                <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
        </div>
        <h1>Login Successful!</h1>
        <p>You're all set. Return to the Columbus app to start monitoring your brand's AI visibility.</p>
        <div class="brand">
            <div class="brand-icon"></div>
            <span>Columbus AI Visibility Platform</span>
        </div>
    </div>
</body>
</html>"#
}

// Fixed port for OAuth callback - must be added to Supabase's allowed redirect URLs
const OAUTH_CALLBACK_PORT: u16 = 19820;

#[tauri::command]
pub async fn login_with_google(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<User, String> {
    // Start a local server on a fixed port to receive the OAuth callback
    // Using a fixed port allows us to add it to Supabase's allowed redirect URLs
    let listener = TcpListener::bind(format!("127.0.0.1:{}", OAUTH_CALLBACK_PORT)).await
        .map_err(|e| format!("Failed to start callback server on port {}: {}. Is another instance running?", OAUTH_CALLBACK_PORT, e))?;

    let port = OAUTH_CALLBACK_PORT;

    let redirect_uri = format!("http://localhost:{}/callback", port);

    // Build the Supabase OAuth URL
    let auth_url = format!(
        "{}/auth/v1/authorize?provider=google&redirect_to={}",
        SUPABASE_URL,
        urlencoding::encode(&redirect_uri)
    );

    // Open the browser
    open::that(&auth_url).map_err(|e| format!("Failed to open browser: {}", e))?;

    // First request: Supabase redirects here with tokens in fragment
    // We serve a page that extracts the fragment and sends it back as query params
    let (mut stream, _) = listener.accept().await
        .map_err(|e| format!("Failed to accept connection: {}", e))?;

    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut request_line = String::new();
    buf_reader.read_line(&mut request_line).await
        .map_err(|e| format!("Failed to read request: {}", e))?;

    let url_part = request_line
        .split_whitespace()
        .nth(1)
        .ok_or("Invalid request")?
        .to_string();

    // Check if this is the initial callback (no tokens in query) or the token submission
    if url_part.contains("access_token=") {
        // This is the token submission - parse and process
        let success_html = get_success_page_html();
        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{}", success_html);
        writer.write_all(response.as_bytes()).await.ok();
        writer.flush().await.ok();

        let full_url = format!("http://localhost{}", url_part);
        let (access_token, refresh_token, expires_in) = parse_oauth_tokens(&full_url)?;

        return finalize_oauth(app, state, access_token, refresh_token, expires_in).await;
    }

    // Serve the token extractor page
    // This page reads the hash fragment and redirects with tokens as query params
    let extractor_page = format!(r#"HTTP/1.1 200 OK
Content-Type: text/html
Connection: close

<!DOCTYPE html>
<html>
<head><title>Columbus Login</title></head>
<body>
<h2>Completing login...</h2>
<script>
    // Get the hash fragment (contains the tokens)
    const hash = window.location.hash.substring(1);
    if (hash) {{
        // Redirect to same server with tokens as query params
        window.location.href = 'http://localhost:{}/tokens?' + hash;
    }} else {{
        document.body.innerHTML = '<h2>Login failed</h2><p>No authentication data received.</p>';
    }}
</script>
</body>
</html>"#, port);

    writer.write_all(extractor_page.as_bytes()).await.ok();
    writer.flush().await.ok();
    drop(writer);
    drop(buf_reader);

    // Wait for second request with tokens
    let (mut stream2, _) = listener.accept().await
        .map_err(|e| format!("Failed to receive tokens: {}", e))?;

    let (reader2, mut writer2) = stream2.split();
    let mut buf_reader2 = BufReader::new(reader2);
    let mut request_line2 = String::new();
    buf_reader2.read_line(&mut request_line2).await
        .map_err(|e| format!("Failed to read token request: {}", e))?;

    let url_part2 = request_line2
        .split_whitespace()
        .nth(1)
        .ok_or("Invalid token request")?
        .to_string();

    // Send success response
    let success_html = get_success_page_html();
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{}", success_html);
    writer2.write_all(response.as_bytes()).await.ok();
    writer2.flush().await.ok();

    // Parse tokens from query params
    let full_url = format!("http://localhost{}", url_part2);
    let (access_token, refresh_token, expires_in) = parse_oauth_tokens(&full_url)?;

    finalize_oauth(app, state, access_token, refresh_token, expires_in).await
}

async fn finalize_oauth(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    access_token: String,
    refresh_token: String,
    expires_in: i64,
) -> Result<User, String> {
    // Get user info from Supabase
    let client = reqwest::Client::new();
    let user_response = client
        .get(&format!("{}/auth/v1/user", SUPABASE_URL))
        .header("Authorization", format!("Bearer {}", access_token))
        .header("apikey", SUPABASE_ANON_KEY)
        .send()
        .await
        .map_err(|e| format!("Failed to get user info: {}", e))?;

    if !user_response.status().is_success() {
        return Err("Failed to get user info".to_string());
    }

    let supabase_user: SupabaseUser = user_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse user: {}", e))?;

    let user = User {
        id: supabase_user.id,
        email: supabase_user.email,
    };

    let expires_at = chrono::Utc::now().timestamp() + expires_in;

    // Store auth state
    {
        let mut auth = state.auth.lock();
        auth.access_token = Some(access_token.clone());
        auth.refresh_token = Some(refresh_token.clone());
        auth.user = Some(user.clone());
        auth.expires_at = Some(expires_at);
    }

    // Persist auth to disk
    let persisted_auth = PersistedAuth {
        access_token,
        refresh_token,
        user_id: user.id.clone(),
        user_email: user.email.clone(),
        expires_at,
    };
    if let Err(e) = storage::update_auth(Some(persisted_auth)) {
        eprintln!("[Auth] Failed to persist OAuth auth: {}", e);
    }

    // Emit auth success event so frontend can refresh
    let _ = app.emit("auth:success", &user);
    println!("[Auth] OAuth login successful, emitted auth:success event");

    Ok(user)
}

/// Ensure the auth token is valid, refreshing if needed.
/// Returns Ok(access_token) if valid, Err if not authenticated or refresh failed.
/// This should be called before any API operation that requires authentication.
pub async fn ensure_valid_token(state: &std::sync::Arc<AppState>) -> Result<String, String> {
    let (access_token, refresh_token, expires_at, user) = {
        let auth = state.auth.lock();
        (
            auth.access_token.clone(),
            auth.refresh_token.clone(),
            auth.expires_at,
            auth.user.clone(),
        )
    };

    // Check if we have auth data
    let access_token = access_token.ok_or("Not authenticated")?;
    let user = user.ok_or("Not authenticated")?;

    // Check if token is expired or about to expire (within 5 minutes)
    if let Some(expires_at) = expires_at {
        let now = chrono::Utc::now().timestamp();
        let is_expired = now >= expires_at - 300; // 5 minute buffer

        if is_expired {
            // Try to refresh the token
            let refresh_token = refresh_token.ok_or("No refresh token available")?;
            println!("[Auth] Token expired, attempting refresh...");

            let (new_access, new_refresh, new_expires_in) = refresh_access_token(&refresh_token).await?;
            let new_expires_at = chrono::Utc::now().timestamp() + new_expires_in;

            // Update state
            {
                let mut auth = state.auth.lock();
                auth.access_token = Some(new_access.clone());
                auth.refresh_token = Some(new_refresh.clone());
                auth.expires_at = Some(new_expires_at);
            }

            // Persist updated tokens
            let persisted_auth = crate::PersistedAuth {
                access_token: new_access.clone(),
                refresh_token: new_refresh,
                user_id: user.id.clone(),
                user_email: user.email.clone(),
                expires_at: new_expires_at,
            };
            if let Err(e) = crate::storage::update_auth(Some(persisted_auth)) {
                eprintln!("[Auth] Failed to persist refreshed auth: {}", e);
            }

            println!("[Auth] Token refreshed successfully");
            return Ok(new_access);
        }
    }

    Ok(access_token)
}

fn parse_oauth_tokens(url: &str) -> Result<(String, String, i64), String> {
    // Check for tokens in fragment (#) or query (?)
    let parse_params = |params: &str| -> Option<(String, String, i64)> {
        let mut access_token = None;
        let mut refresh_token = None;
        let mut expires_in = 3600i64;

        for pair in params.split('&') {
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() == 2 {
                match parts[0] {
                    "access_token" => access_token = Some(parts[1].to_string()),
                    "refresh_token" => refresh_token = Some(parts[1].to_string()),
                    "expires_in" => expires_in = parts[1].parse().unwrap_or(3600),
                    _ => {}
                }
            }
        }

        if let (Some(at), Some(rt)) = (access_token, refresh_token) {
            Some((at, rt, expires_in))
        } else {
            None
        }
    };

    // Try fragment first
    if let Some(fragment_pos) = url.find('#') {
        let fragment = &url[fragment_pos + 1..];
        if let Some(tokens) = parse_params(fragment) {
            return Ok(tokens);
        }
    }

    // Try query params
    if let Some(query_pos) = url.find('?') {
        let query = &url[query_pos + 1..];
        // Handle case where fragment comes after query
        let query = query.split('#').next().unwrap_or(query);
        if let Some(tokens) = parse_params(query) {
            return Ok(tokens);
        }
    }

    Err("No tokens found in OAuth callback".to_string())
}
