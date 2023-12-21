use cfg_if::cfg_if;

cfg_if! { if #[cfg(feature = "ssr")] {
    use {
        axum::{
            body::{boxed, Body, BoxBody},
            extract::State,
            response::IntoResponse,
            http::{Request, Response, StatusCode, Uri},
            response::Response as AxumResponse,
        },
        crate::components::layout::App,
        tower::ServiceExt,
        tower_http::services::ServeDir,
        leptos::*
    };

    pub async fn file_and_error_handler(uri: Uri, State(options): State<LeptosOptions>, req: Request<Body>) -> AxumResponse {
        let root = options.site_root.clone();
        let res = get_static_file(uri.clone(), &root).await.unwrap();

        if res.status() == StatusCode::OK {
            res.into_response()
        } else {
            let handler = leptos_axum::render_app_to_stream(options.to_owned(), move || view!{<App/>});
            handler(req).await.into_response()
        }
    }

    async fn get_static_file(uri: Uri, root: &str) -> Result<Response<BoxBody>, (StatusCode, String)> {
        let req = Request::builder().uri(uri.clone()).body(Body::empty()).unwrap();

        match ServeDir::new(root).oneshot(req).await {
            Ok(res) => Ok(res.map(boxed)),
            Err(err) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {err}"),
            )),
        }
    }
}}
