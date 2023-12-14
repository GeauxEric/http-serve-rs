use std::path::{Path as FsPath, PathBuf};

use anyhow::anyhow;
use axum::body::Body;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::{http, routing::get, Router};
use clap::{arg, command, value_parser, Command};
use futures_util::TryFutureExt;
use futures_util::TryStreamExt;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{BytesCodec, FramedRead};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

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
    list_dir(FsPath::new(".")).await
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
        let mime_type = mime_guess::from_path(&fs_path)
            .first()
            .map(|m| m.to_string())
            .unwrap_or("application/octet-stream".to_string());
        let stream = tokio::fs::File::open(fs_path)
            .map_ok(|file| FramedRead::new(file, BytesCodec::new()).map_ok(BytesMut::freeze))
            .try_flatten_stream();
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_TYPE, mime_type)
            .body(Body::from_stream(stream))
            .unwrap();
        return response;
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
    Ok(Html(html_content))
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

fn get_commands() -> Command {
    command!() // requires `cargo` feature
        .arg(
            arg!(
                -p --port <PORT> "Sets network port"
            )
            .required(false)
            .default_value("3000")
            .value_parser(value_parser!(usize)),
        )
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let matches = get_commands().get_matches();
    let port: usize = *matches.get_one("port").expect("`port` is not set");
    let addr = format!("0.0.0.0:{}", port);
    println!("Serving HTTP on http://{}/ ...", addr);
    // build our application with a single route
    let app = Router::new()
        .route("/", get(list_pwd))
        .route("/*fs_path", get(get_file_or_list_dir))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
