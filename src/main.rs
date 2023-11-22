use anyhow::anyhow;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::{routing::get, Router};

async fn list_pwd() -> Result<Html<String>, AppErr> {
    let mut dir = tokio::fs::read_dir(".")
        .await
        .map_err(|e| AppErr(anyhow::Error::from(e)))?;
    let mut html_content =
        String::from("<html><head><title>Directory Listing</title></head><body><ul>");

    while let Some(entry) = dir
        .next_entry()
        .await
        .map_err(|e| AppErr(anyhow::Error::from(e)))?
    {
        let file_name = entry
            .file_name()
            .into_string()
            .map_err(|_e| AppErr(anyhow!("[Invalid UTF-8]")))?;
        let metadata = entry
            .metadata()
            .await
            .map_err(|e| AppErr(anyhow::Error::from(e)))?;
        let suffix = if metadata.is_dir() {
            "/"
        } else if metadata.is_symlink() {
            "@"
        } else {
            ""
        };
        html_content.push_str(&format!(
            "<li><a href=\"{}\">{}{}</a></li>",
            file_name, file_name, suffix
        ));
    }

    html_content.push_str("</ul></body></html>");
    Ok(Html::from(html_content))
}

struct AppErr(anyhow::Error);

impl IntoResponse for AppErr {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().route("/", get(list_pwd));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
