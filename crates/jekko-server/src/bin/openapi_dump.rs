//! Dump the OpenAPI document to stdout.
//!
//! Used by `xtask openapi-check` to diff the canonical OpenAPI schema against
//! `docs/openapi-snapshot.json`. Runs without any AppState dependencies — it
//! just asks `utoipa` to serialise the `ApiDoc` derive.

use jekko_server::routes::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = ApiDoc::openapi();
    let json = serde_json::to_string_pretty(&doc)?;
    println!("{json}");
    Ok(())
}
