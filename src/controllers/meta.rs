use std::collections::BTreeSet;

use axum::{
    body::Body,
    extract::State,
    http::{Response, header},
};

use crate::{
    GlobalState, Project,
    services::meta::{canonical_url, xml_escape},
};

pub async fn robots_txt(State(state): State<GlobalState>) -> Response<Body> {
    let body = format!(
        "User-agent: *\n\
         Allow: /\n\n\
         Disallow: /~*/branches\n\
         Disallow: /~*/tags\n\
         Disallow: /~*/log/\n\
         Disallow: /~*/tree/\n\
         Disallow: /~*/blob/\n\n\
         Sitemap: {}/sitemap.xml\n",
        state.config.base_url.trim_end_matches('/')
    );

    Response::builder()
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

pub async fn sitemap_xml(State(state): State<GlobalState>) -> Response<Body> {
    let mut urls = BTreeSet::new();
    urls.insert(canonical_url(&state.config.base_url, "/"));
    urls.insert(canonical_url(&state.config.base_url, "/projects"));

    for page in &state.config.content_pages {
        urls.insert(canonical_url(&state.config.base_url, &page.url_path()));
    }

    for info in state.auth.get_public_projects() {
        let Some(project) = Project::from_project_info(&info) else {
            continue;
        };
        urls.insert(canonical_url(
            &state.config.base_url,
            &format!("/~{}", project.owner),
        ));
        urls.insert(canonical_url(&state.config.base_url, &project.uri()));
    }

    let mut body = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#,
    );

    for url in urls {
        body.push_str("  <url><loc>");
        body.push_str(&xml_escape(&url));
        body.push_str("</loc></url>\n");
    }

    body.push_str("</urlset>\n");

    Response::builder()
        .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}
