use axum::{
    body::Body,
    extract::{Query, State},
    http::{StatusCode, header},
    response::Response,
};
use futures::stream;
use serde::Deserialize;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tower_cookies::Cookies;

use crate::{AccessType, GlobalState, extractors::PathUserProject, services::session};

#[derive(Debug, Deserialize)]
pub struct GitServiceQuery {
    service: String,
}

/// GET /{username}/{slug}/info/refs?service=git-upload-pack
/// Service discovery endpoint for git clone/fetch operations
pub async fn git_info_refs(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
    Query(query): Query<GitServiceQuery>,
) -> Result<Response, StatusCode> {
    // Check access control
    let logged_in_user = session::current_user(&state, &cookies).await.ok();
    let access_level = project
        .access_level(logged_in_user.as_ref().map(|user| user.slug.clone()))
        .await;

    if access_level == AccessType::None {
        return Err(StatusCode::NOT_FOUND);
    }

    // Only support git-upload-pack (read-only)
    if query.service != "git-upload-pack" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Build repository path
    let repo_path = state.config.git_root.join(&owner.slug).join(&project.slug);

    if !repo_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Execute git upload-pack --http-backend-info-refs
    let mut child = Command::new("git")
        .arg("upload-pack")
        .arg("--http-backend-info-refs")
        .arg(&repo_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Read all output
    let mut output = Vec::new();
    stdout
        .read_to_end(&mut output)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Wait for child to complete
    let status = child
        .wait()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !status.success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Prepend the service advertisement packet-line
    let service_line = format!("# service={}\n", query.service);
    let service_packet = format!("{:04x}{}", service_line.len() + 4, service_line);
    let flush_packet = "0000";

    let mut response_body = Vec::new();
    response_body.extend_from_slice(service_packet.as_bytes());
    response_body.extend_from_slice(flush_packet.as_bytes());
    response_body.extend_from_slice(&output);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/x-git-upload-pack-advertisement",
        )
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(response_body))
        .unwrap())
}

/// POST /{username}/{slug}/git-upload-pack
/// Data transfer endpoint for git clone/fetch
pub async fn git_upload_pack(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
    body: axum::body::Bytes,
) -> Result<Response, StatusCode> {
    // Check access control
    let logged_in_user = session::current_user(&state, &cookies).await.ok();
    let access_level = project
        .access_level(logged_in_user.as_ref().map(|user| user.slug.clone()))
        .await;

    if access_level == AccessType::None {
        return Err(StatusCode::NOT_FOUND);
    }

    // Build repository path
    let repo_path = state.config.git_root.join(&owner.slug).join(&project.slug);

    if !repo_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Execute git upload-pack --stateless-rpc
    let mut child = Command::new("git")
        .arg("upload-pack")
        .arg("--stateless-rpc")
        .arg(&repo_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Write request body to git stdin
    let mut stdin = child
        .stdin
        .take()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    tokio::io::AsyncWriteExt::write_all(&mut stdin, &body)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    drop(stdin); // Close stdin to signal EOF

    // Stream response from git stdout
    let stdout = child
        .stdout
        .take()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let stream = stream::unfold(
        (stdout, Some(child), vec![0u8; 8192]),
        |(mut stdout, mut child, mut buf)| async move {
            match stdout.read(&mut buf).await {
                Ok(0) => {
                    if let Some(mut child) = child {
                        let _ = child.wait().await;
                    }
                    None // EOF
                }
                Ok(n) => {
                    let bytes = axum::body::Bytes::copy_from_slice(&buf[..n]);
                    Some((Ok::<_, std::io::Error>(bytes), (stdout, child, buf)))
                }
                Err(e) => {
                    if let Some(mut child) = child.take() {
                        let _ = child.wait().await;
                    }
                    Some((Err(e), (stdout, None, buf)))
                }
            }
        },
    );

    let body = Body::from_stream(stream);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-git-upload-pack-result")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(body)
        .unwrap())
}
