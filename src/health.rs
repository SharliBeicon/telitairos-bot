use axum::{routing::get, Router};
use tokio::net::TcpListener;

pub async fn serve(listener: TcpListener) -> std::io::Result<()> {
    let app = Router::new().route("/health", get(|| async { "ok\n" }));
    axum::serve(listener, app).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn fixture() -> (reqwest::Client, String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { serve(listener).await.unwrap() });
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        (client, url, server)
    }

    #[tokio::test]
    async fn health_returns_ok_without_credentials_or_external_requests() {
        let (client, url, server) = fixture().await;
        let response = client.get(format!("{url}/health")).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.text().await.unwrap(), "ok\n");
        server.abort();
    }

    #[tokio::test]
    async fn health_supports_head_probes() {
        let (client, url, server) = fixture().await;
        let response = client.head(format!("{url}/health")).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert!(response.bytes().await.unwrap().is_empty());
        server.abort();
    }

    #[tokio::test]
    async fn unknown_paths_return_not_found() {
        let (client, url, server) = fixture().await;
        let response = client.get(format!("{url}/")).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
        server.abort();
    }
}
