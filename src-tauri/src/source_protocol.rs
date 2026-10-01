use std::fs;

use tauri::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use tauri::http::{HeaderValue, Request, Response, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext};

use crate::files::probe_source;
use crate::models::SourceFormat;
use crate::state::AppState;

pub const SOURCE_PROTOCOL: &str = "vfsource";

pub fn preview_url(file_id: &str) -> String {
    #[cfg(any(target_os = "windows", target_os = "android"))]
    {
        format!("http://{SOURCE_PROTOCOL}.localhost/{file_id}")
    }

    #[cfg(not(any(target_os = "windows", target_os = "android")))]
    {
        format!("{SOURCE_PROTOCOL}://localhost/{file_id}")
    }
}

pub fn handle<R: Runtime>(
    context: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let Some(file_id) = source_id_from_path(request.uri().path()) else {
        return response(StatusCode::BAD_REQUEST, "text/plain; charset=utf-8", b"invalid source".to_vec());
    };

    let snapshot = {
        let state = context.app_handle().state::<AppState>();
        let registry = match state.registry.lock() {
            Ok(registry) => registry,
            Err(_) => {
                return response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "text/plain; charset=utf-8",
                    b"source unavailable".to_vec(),
                );
            }
        };

        match registry.resolve_source(file_id) {
            Ok(snapshot) => snapshot,
            Err(_) => {
                return response(
                    StatusCode::NOT_FOUND,
                    "text/plain; charset=utf-8",
                    b"source not found".to_vec(),
                );
            }
        }
    };

    let before = match probe_source(&snapshot.path) {
        Ok(probe) if probe.fingerprint == snapshot.fingerprint => probe,
        Ok(_) => {
            return response(
                StatusCode::CONFLICT,
                "text/plain; charset=utf-8",
                b"source changed".to_vec(),
            );
        }
        Err(_) => {
            return response(
                StatusCode::NOT_FOUND,
                "text/plain; charset=utf-8",
                b"source unavailable".to_vec(),
            );
        }
    };

    let bytes = match fs::read(&snapshot.path) {
        Ok(bytes) => bytes,
        Err(_) => {
            return response(
                StatusCode::NOT_FOUND,
                "text/plain; charset=utf-8",
                b"source unavailable".to_vec(),
            );
        }
    };

    match probe_source(&snapshot.path) {
        Ok(after) if after.fingerprint == snapshot.fingerprint => {
            response(StatusCode::OK, mime_type(before.format), bytes)
        }
        _ => response(
            StatusCode::CONFLICT,
            "text/plain; charset=utf-8",
            b"source changed".to_vec(),
        ),
    }
}

fn source_id_from_path(path: &str) -> Option<&str> {
    let value = path.strip_prefix('/')?;
    if value.is_empty() || value.contains('/') {
        None
    } else {
        Some(value)
    }
}

fn mime_type(format: SourceFormat) -> &'static str {
    match format {
        SourceFormat::Png => "image/png",
        SourceFormat::Jpeg => "image/jpeg",
        SourceFormat::Webp => "image/webp",
        SourceFormat::Bmp => "image/bmp",
    }
}

fn response(status: StatusCode, content_type: &'static str, body: Vec<u8>) -> Response<Vec<u8>> {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use crate::models::SourceFormat;

    use super::{mime_type, preview_url, source_id_from_path};

    #[test]
    fn preview_url_contains_only_opaque_source_id() {
        let id = "00000000-0000-4000-8000-000000000001";
        let url = preview_url(id);
        assert!(url.ends_with(id));
        assert!(!url.contains("Users"));
        assert!(!url.contains("fixture.png"));
    }

    #[test]
    fn protocol_path_accepts_one_opaque_segment_only() {
        assert_eq!(
            source_id_from_path("/00000000-0000-4000-8000-000000000001"),
            Some("00000000-0000-4000-8000-000000000001")
        );
        assert_eq!(source_id_from_path("/one/two"), None);
        assert_eq!(source_id_from_path("/"), None);
    }

    #[test]
    fn source_mime_comes_from_probed_format() {
        assert_eq!(mime_type(SourceFormat::Png), "image/png");
        assert_eq!(mime_type(SourceFormat::Jpeg), "image/jpeg");
        assert_eq!(mime_type(SourceFormat::Webp), "image/webp");
        assert_eq!(mime_type(SourceFormat::Bmp), "image/bmp");
    }
}
