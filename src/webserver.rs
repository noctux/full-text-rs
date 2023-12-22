use log::*;
use axum::{
    Router,
    extract::Query,
    routing::get,

    response::{IntoResponse, Response},
    http::{StatusCode, header},

    debug_handler,
};
use serde::Deserialize;
use article_scraper::ArticleScraper;

use super::feeds;

#[derive(Deserialize, Debug)]
struct ExtractionTarget {
    url: String,
}

#[debug_handler]
async fn makefulltextfeed(extraction: Query<ExtractionTarget>) -> Response {
    trace!("makefulltextfeed: {:?}", extraction);
    let scraper = ArticleScraper::new(None).await;

    let extract_conf = feeds::ExtractionOpts {
        max_items: Some(5),
        keep_failed: true,
        keep_original_content: true
    };
    let feed_res = feeds::get_fulltext_feed(&scraper, &extraction.url, &extract_conf).await;
    let response = match feed_res {
        Ok(feed) => {
            (StatusCode::OK, [(header::CONTENT_TYPE, [feed.mime_type(), "charset=UTF-8"].join("; "))], feed.to_string()).into_response()
        }
        Err(e) => {
            info!("Failed to extract feed {}: {:?}", extraction.url, e);
            (StatusCode::BAD_REQUEST, format!("{:?}", e)).into_response()
        }
    };
    return response;
}

pub async fn serve() {
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route_service("/makefulltextfeed", get(makefulltextfeed));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
