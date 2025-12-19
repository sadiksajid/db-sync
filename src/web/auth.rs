use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};

/// Auth middleware to check for valid session cookie
pub async fn require_auth(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract session cookie
    let session_id = req
        .headers()
        .get(header::COOKIE)
        .and_then(|cookie| cookie.to_str().ok())
        .and_then(|cookie_str| {
            // Parse cookies
            cookie_str
                .split(';')
                .find_map(|cookie| {
                    let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                    if parts.len() == 2 && parts[0] == "session_id" {
                        Some(parts[1].to_string())
                    } else {
                        None
                    }
                })
        });

    if session_id.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Continue to the handler
    Ok(next.run(req).await)
}

/// Extract session ID from request
pub fn extract_session_id(req: &Request) -> Option<String> {
    req.headers()
        .get(header::COOKIE)
        .and_then(|cookie| cookie.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .find_map(|cookie| {
                    let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                    if parts.len() == 2 && parts[0] == "session_id" {
                        Some(parts[1].to_string())
                    } else {
                        None
                    }
                })
        })
}

