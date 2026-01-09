use std::convert::Infallible;

use axum::{
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::Response,
};
use futures::stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tower_cookies::Cookies;

use crate::{
    AccessType, GlobalState, Project,
    extractors::{PathUser, PathUserProject},
    models::RepoEvent,
    services::session,
};

/// GET /.events - SSE stream for all events on the instance
pub async fn global_events(State(state): State<GlobalState>, cookies: Cookies) -> Response<Body> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    let rx = state.event_tx.subscribe();

    let stream = BroadcastStream::new(rx)
        .filter_map(move |result| {
            let state = state.clone();
            let logged_in_user = logged_in_user.clone();

            async move {
                let event = result.ok()?;

                // Check read access for the event's project
                let info = event.info();
                if let Ok(project) = Project::load(&state, &info.owner, &info.project).await {
                    let access = project
                        .access_level(logged_in_user.as_ref().map(|u| u.slug.clone()))
                        .await;
                    if access != AccessType::None {
                        return Some(event);
                    }
                }
                None
            }
        })
        .map(format_sse_event);

    sse_response(stream)
}

/// GET /{username}/.events - SSE stream for all projects owned by user
pub async fn user_events(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUser(owner): PathUser,
) -> Response<Body> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();
    let user_slug = owner.slug.clone();

    let rx = state.event_tx.subscribe();

    let stream = BroadcastStream::new(rx)
        .filter_map(move |result| {
            let state = state.clone();
            let logged_in_user = logged_in_user.clone();
            let user_slug = user_slug.clone();

            async move {
                let event = result.ok()?;
                let info = event.info();

                // Filter by owner
                if info.owner != user_slug {
                    return None;
                }

                // Check read access for the event's project
                if let Ok(project) = Project::load(&state, &info.owner, &info.project).await {
                    let access = project
                        .access_level(logged_in_user.as_ref().map(|u| u.slug.clone()))
                        .await;
                    if access != AccessType::None {
                        return Some(event);
                    }
                }
                None
            }
        })
        .map(format_sse_event);

    sse_response(stream)
}

/// GET /{username}/{slug}/.events - SSE stream for a specific project
pub async fn project_events(
    State(state): State<GlobalState>,
    cookies: Cookies,
    PathUserProject(owner, project): PathUserProject,
) -> Result<Response<Body>, StatusCode> {
    let logged_in_user = session::current_user(&state, &cookies).await.ok();

    // Check read access upfront
    let access_level = project
        .access_level(logged_in_user.as_ref().map(|u| u.slug.clone()))
        .await;

    if access_level == AccessType::None {
        return Err(StatusCode::FORBIDDEN);
    }

    let owner_slug = owner.slug.clone();
    let project_slug = project.slug.clone();

    let rx = state.event_tx.subscribe();

    let stream = BroadcastStream::new(rx)
        .filter_map(move |result| {
            let owner_slug = owner_slug.clone();
            let project_slug = project_slug.clone();

            async move {
                let event = result.ok()?;
                let info = event.info();

                // Filter by owner and project
                if info.owner == owner_slug && info.project == project_slug {
                    Some(event)
                } else {
                    None
                }
            }
        })
        .map(format_sse_event);

    Ok(sse_response(stream))
}

fn format_sse_event(event: RepoEvent) -> Result<String, Infallible> {
    let json = serde_json::to_string(&event).unwrap_or_default();
    Ok(format!("data: {}\n\n", json))
}

fn sse_response<S>(stream: S) -> Response<Body>
where
    S: futures::Stream<Item = Result<String, Infallible>> + Send + 'static,
{
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(Body::from_stream(stream))
        .unwrap()
}
