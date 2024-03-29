use spm_runner::config::RegistryConfig;

use mockito::{Matcher, Server};
use tokio;
use url::Url;

#[tokio::test]
async fn test_no_strategy_found() {
    let mut server = Server::new_async().await;

    let _m = server
        .mock(
            "GET",
            Matcher::Regex(r"/find-install-strategy".to_string()),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create_async()
        .await;

    let url = Url::parse(&server.url()).unwrap();

    let registry_config = RegistryConfig::new(server.url(), url.port().unwrap_or(80));

    let mut buffer = Vec::new();

    let _ = spm_runner::commands::install::run("Suri", "AWS", &registry_config, &mut buffer).await;

    let output = String::from_utf8_lossy(&buffer);

    _m.assert_async().await;

    assert_ne!(output.find("No valid install strategies found"), None);
}
