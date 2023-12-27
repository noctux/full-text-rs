use log::*;
use axum::{
    Router,
    extract::{Query, State, Form},
    routing::get,

    response::{IntoResponse, Response, Html, Redirect},
    http::{StatusCode, header},

    debug_handler,
};
use serde::Deserialize;
use article_scraper::ArticleScraper;
use std::sync::Arc;
use std::convert::TryFrom;

use pathetic::Uri;

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

async fn show_form() -> Html<&'static str> {
    Html(
        r#"
        <!doctype html>
        <html>
            <head></head>
            <body>
                <form action="/" method="post">
                    <label for="url">
                        Feed url:
                        <input type="url" name="url" size="150">
                    </label><br/>
                    <label for="max_items">
                        Maximum number of items (optional):
                        <input type="number" name="max_items">
                    </label>
                    <br/>

                    <group>
                        Handling of failed items:
                        <input type="radio" id="failed_default" name="keep_failed" value="Default" checked="checked">
                        <label for="failed_default">use instance default</label>
                        <input type="radio" id="failed_true" name="keep_failed" value="True">
                        <label for="failed_true">keep in feed</label>
                        <input type="radio" id="failed_false" name="keep_failed" value="False">
                        <label for="failed_false">discard</label>
                    </group>
                    <br/>

                    <group>
                        Keep original content:
                        <input type="radio" id="keep_original_default" name="keep_original_content" value="Default" checked="checked">
                        <label for="keep_original_default">use instance default</label>
                        <input type="radio" id="keep_original_true" name="keep_original_content" value="True">
                        <label for="keep_original_true">keep in feed</label>
                        <input type="radio" id="keep_original_false" name="keep_original_content" value="False">
                        <label for="keep_original_false">discard</label>
                    </group>
                    <br/>

                    <input type="submit" value="Get fulltext feed!">
                </form>
            </body>
        </html>
        "#,
    )
}

#[derive(Deserialize, Debug)]
enum TriState {
    Default,
    True,
    False,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Input {
    url: String,
    max_items: Option<usize>,
    keep_failed: TriState,
    keep_original_content: TriState,
}

async fn accept_form(Form(input): Form<Input>) -> Redirect {
    trace!("Form submission: {:?}", input);
    let mut uri =
        Uri::default()
            .with_path("/makefulltextfeed")
            .with_query_pairs_mut(|q| q.append_pair("url", &input.url));
    if let Some(max_items) = input.max_items {
        uri.query_pairs_mut()
            .append_pair("max_items", &max_items.to_string());
    }
    match input.keep_failed {
        TriState::True => {
            uri.query_pairs_mut()
                .append_pair("keep_failed", "true");
        },
        TriState::False => {
            uri.query_pairs_mut()
                .append_pair("keep_failed", "false");
        },
        TriState::Default => (),
    };
    match input.keep_original_content {
        TriState::True => {
            uri.query_pairs_mut()
                .append_pair("keep_original_content", "true");
        },
        TriState::False => {
            uri.query_pairs_mut()
                .append_pair("keep_original_content", "false");
        },
        TriState::Default => (),
    }
    Redirect::to(uri.as_str())
}

pub async fn serve(extraction_defaults: super::config::ExtractionOpts, extraction_limits: ExtractionLimits) {
    // build our application with a single route
    let app = Router::new()
        .route("/", get(show_form).post(accept_form))
        .route("/makefulltextfeed", get(makefulltextfeed))
        .with_state(Arc::new(AppState {
            defaults: extraction_defaults,
            limits: extraction_limits
        }));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
