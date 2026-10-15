//! GPUI image transport. Blocking network/decode work belongs to services and the background executor.
use gpui_kit::{
    BackgroundExecutor,
    http_client::{AsyncBody, HttpClient, Method, Request, Response, Url, http::HeaderValue},
};
use std::{future::Future, pin::Pin};
pub(super) struct ImageHttpClient {
    executor: BackgroundExecutor,
}
impl ImageHttpClient {
    pub fn new(executor: BackgroundExecutor) -> Self {
        Self { executor }
    }
}
impl HttpClient for ImageHttpClient {
    fn user_agent(&self) -> Option<&HeaderValue> {
        None
    }
    fn proxy(&self) -> Option<&Url> {
        None
    }
    fn send(
        &self,
        request: Request<AsyncBody>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Response<AsyncBody>>> + Send + 'static>> {
        let executor = self.executor.clone();
        Box::pin(async move {
            anyhow::ensure!(
                request.method() == Method::GET,
                "The image transport supports GET only"
            );
            let url = Url::parse(&request.uri().to_string())?;
            anyhow::ensure!(
                matches!(url.scheme(), "http" | "https")
                    && url.username().is_empty()
                    && url.password().is_none(),
                "Unsupported image URL"
            );
            let (cancel, guard) = glim_services::work::Cancellation::task();
            let _guard = guard;
            executor
                .spawn(async move {
                    let bytes = glim_services::markup_images::fetch_image_cancellable(
                        url.as_str(),
                        &cancel,
                    )
                    .or_else(|_| glim_services::markup_images::unavailable_image())?;
                    Response::builder()
                        .status(200)
                        .body(AsyncBody::from(bytes))
                        .map_err(Into::into)
                })
                .await
        })
    }
}
