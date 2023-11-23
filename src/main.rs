use std::path::{Path as FsPath, PathBuf};

use anyhow::anyhow;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::{http, routing::get, Router};

fn fs_path_to_url_path(p: &FsPath) -> String {
    let s = p.to_str().unwrap();
    return if let Some(p) = s.strip_prefix('.') {
        p.to_string()
    } else {
        s.to_string()
    };
}

#[test]
fn test_path_to_href() {
    let path = FsPath::new("./");
    assert_eq!(fs_path_to_url_path(path), "/");

    let path = FsPath::new(".");
    assert_eq!(fs_path_to_url_path(path), "");
}

async fn list_pwd() -> Result<Html<String>, AppErr> {
    return list_dir(FsPath::new(".")).await;
}

async fn get_file_or_list_dir(Path(url_path): Path<String>) -> Response {
    let mut fs_path = PathBuf::from(".");
    fs_path.push(url_path);
    if fs_path.is_dir() {
        return match list_dir(&fs_path).await {
            Ok(html) => html.into_response(),
            Err(e) => e.into_response(),
        };
    }
    if fs_path.is_file() {
        // TODO: stream the content
        return match tokio::fs::read(&fs_path).await {
            Ok(content) => {
                let mime_type = mime_guess::from_path(&fs_path)
                    .first()
                    .map(|m| m.to_string())
                    .unwrap_or("application/octet-stream".to_string());
                (
                    StatusCode::OK,
                    [(http::header::CONTENT_TYPE, mime_type)],
                    content,
                )
                    .into_response()
            }
            Err(e) => AppErr::from(e).into_response(),
        };
    }
    AppErr(anyhow!("unhandled type")).into_response()
}

async fn list_dir(p: &FsPath) -> Result<Html<String>, AppErr> {
    let mut dir = tokio::fs::read_dir(p).await?;
    let mut html_content =
        String::from("<html><head><title>Directory Listing</title></head><body><ul>");

    while let Some(entry) = dir.next_entry().await? {
        let file_name = entry
            .file_name()
            .into_string()
            .unwrap_or_else(|_| String::from("[Invalid UTF-8]"));
        let metadata = entry.metadata().await?;
        let suffix = if metadata.is_dir() {
            "/"
        } else if metadata.is_symlink() {
            "@"
        } else {
            ""
        };
        let mut dir = p.to_path_buf();
        dir.push(FsPath::new(&file_name));
        let href = fs_path_to_url_path(&dir);
        html_content.push_str(&format!(
            "<li><a href=\"{}\">{}{}</a></li>",
            href, file_name, suffix
        ));
    }

    html_content.push_str("</ul></body></html>");
    return Ok(Html(html_content));
}

struct AppErr(anyhow::Error);

impl IntoResponse for AppErr {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR).into_response()
    }
}

impl<E> From<E> for AppErr
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[tokio::main]
async fn main() {
    // TODO: add logging and tracing
    // build our application with a single route
    let app = Router::new()
        .route("/", get(list_pwd))
        .route("/*fs_path", get(get_file_or_list_dir));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
