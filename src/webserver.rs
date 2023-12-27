use log::*;
use axum::{
    Router,
    extract::{Query, State},
    routing::get,

    response::{IntoResponse, Response},
    http::{StatusCode, header},

    debug_handler,
};
use serde::Deserialize;
use article_scraper::ArticleScraper;
use std::sync::Arc;
use std::convert::TryFrom;

use super::config::ExtractionLimits;

use super::feeds;

#[derive(Copy, Clone, Debug)]
struct AppState {
    defaults: super::config::ExtractionOpts,
    limits: ExtractionLimits,
}

#[derive(Deserialize, Debug)]
struct ExtractionQueryOptions {
    url: String,
    max_items: Option<u32>,
    keep_failed: Option<bool>,
    keep_original_content: Option<bool>
}

/// Merge extraction defaults from config with configuration from the current request, safely
/// bounding by limits (again from configuration)
fn determine_effective_extraction_parameters(conf_params: &super::config::ExtractionOpts, req_params: &ExtractionQueryOptions, limits: &ExtractionLimits) -> feeds::ExtractionOpts {
    feeds::ExtractionOpts {
        max_items: req_params.max_items
                    // Default to largest usize type if parameter is too large
                    .map(|n| usize::try_from(n).unwrap_or(usize::MAX))
                    .or(conf_params.max_items),
        keep_failed: req_params.keep_failed.unwrap_or(conf_params.keep_failed),
        keep_original_content: req_params.keep_original_content.unwrap_or(conf_params.keep_original_content),
    }.bound_by_limits(&limits)
}

#[debug_handler]
async fn makefulltextfeed(Query(extraction_params): Query<ExtractionQueryOptions>, State(state): State<Arc<AppState>>) -> Response {
    trace!("makefulltextfeed: extraction_params: {:?} state: {:?}", extraction_params, state);
    let scraper = ArticleScraper::new(None).await;

    let extract_conf = determine_effective_extraction_parameters(&state.defaults, &extraction_params, &state.limits);
    trace!("Effective extraction opts: {:?}", extract_conf);

    let feed_res = feeds::get_fulltext_feed(&scraper, &extraction_params.url, &extract_conf).await;
    let response = match feed_res {
        Ok(feed) => {
            (StatusCode::OK, [(header::CONTENT_TYPE, [feed.mime_type(), "charset=UTF-8"].join("; "))], feed.to_string()).into_response()
        }
        Err(e) => {
            info!("Failed to extract feed {}: {:?}", extraction_params.url, e);
            (StatusCode::BAD_REQUEST, format!("{:?}", e)).into_response()
        }
    };
    return response;
}

pub async fn serve(extraction_defaults: super::config::ExtractionOpts, extraction_limits: ExtractionLimits) {
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/makefulltextfeed", get(makefulltextfeed))
        .with_state(Arc::new(AppState {
            defaults: extraction_defaults,
            limits: extraction_limits
        }));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
