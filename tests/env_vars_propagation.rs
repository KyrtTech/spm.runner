use std::collections::HashMap;

use spm_runner::{
    commands::install::RunArguments,
    config::{AppConfig, RegistryConfig},
};

use mockito::{Matcher, Server};
use tokio;

#[tokio::test]
async fn test_no_strategy_found() {
    let mut server = Server::new_async().await;

    let _m = server
        .mock(
            "GET",
            Matcher::Regex(r"^/find-install-strategy".to_string()),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create_async()
        .await;

    let registry_config = RegistryConfig::new(server.url());

    let mut buffer = Vec::new();

    let empty_user_params = HashMap::new();

    let args = RunArguments {
        application: "Suri",
        infra: "AWS",
        user_params: &empty_user_params,
        registry_config: &registry_config,
    };

    let config = AppConfig::new("dev", "test", false, false);

    let _ = spm_runner::commands::install::run(args, config, &mut buffer).await;

    let output = String::from_utf8_lossy(&buffer);

    _m.assert_async().await;

    assert_ne!(output.find("No valid install strategies found"), None);
}

#[tokio::test]
async fn one_step_strategy_no_scripts() {
    let mut server = Server::new_async().await;

    let _m = server
        .mock(
            "GET",
            Matcher::Regex(r"^/find-install-strategy".to_string()),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
         "title": "Some title",
         "uri": "http://something.com",
         "description": "Some description",
         "optionalParams": [],
         "scripts": []
        }]"#,
        )
        .create_async()
        .await;

    let registry_config = RegistryConfig::new(server.url());

    let mut buffer = Vec::new();

    let empty_user_params = HashMap::new();

    let args = RunArguments {
        application: "Suri",
        infra: "AWS",
        user_params: &empty_user_params,
        registry_config: &registry_config,
    };

    let config = AppConfig::new("dev", "test", false, false);

    let _ = spm_runner::commands::install::run(args, config, &mut buffer).await;

    let output = String::from_utf8_lossy(&buffer);

    _m.assert_async().await;

    assert_ne!(output.find("Using strategy 'Some title'"), None);
}

#[tokio::test]
async fn one_step_strategy_with_scripts() {
    let mut server = Server::new_async().await;

    let strategy_url = server.url() + "/sample-strategy.sh";

    let get_body = format!(
        r#"[{{
        "title": "Some title",
        "uri": "http://something.com",
        "description": "Some description",
        "optionalParams": [],
        "scripts": ["{}"]
       }}]"#,
        strategy_url
    );

    let _m = server
        .mock(
            "GET",
            Matcher::Regex(r"^/find-install-strategy".to_string()),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(get_body)
        .create_async()
        .await;

    let _s = server
        .mock("GET", "/sample-strategy.sh")
        .with_status(200)
        .with_body(
            r#"#!/bin/sh
          output_params="{\"output_params\": {\"url\": \"http://localhost:8080\"}}"
          echo $output_params > "${SMP_OUTPUT_PATH}"
          "#,
        )
        .create_async()
        .await;

    let registry_config = RegistryConfig::new(server.url());

    let empty_user_params = HashMap::new();

    let args = RunArguments {
        application: "Suri",
        infra: "AWS",
        user_params: &empty_user_params,
        registry_config: &registry_config,
    };

    let mut buffer = Vec::new();

    let config = AppConfig::new("dev", "test", false, false);

    let env_vars = spm_runner::commands::install::run(args, config, &mut buffer)
        .await
        .unwrap_or_default();

    let output = String::from_utf8_lossy(&buffer);

    _m.assert_async().await;
    _s.assert_async().await;

    assert_ne!(output.find("Using strategy 'Some title'"), None);
    assert_eq!(
        env_vars.get("URL"),
        Some(&"http://localhost:8080".to_string())
    );
}

#[tokio::test]
async fn two_one_step_strategy_with_scripts() {
    let mut server = Server::new_async().await;

    let strategy_url_1 = server.url() + "/sample-strategy.sh";
    let strategy_url_2 = server.url() + "/sample-strategy2.sh";

    let get_body = format!(
        r#"[{{
        "title": "Some title 1",
        "uri": "http://something.com",
        "description": "Some description 1",
        "optionalParams": [],
        "scripts": ["{}"]
       }}, {{
        "title": "Some title 2",
        "uri": "http://something2.com",
        "description": "Some description 2",
        "optionalParams": [],
        "scripts": ["{}"]
       }}]"#,
        strategy_url_1, strategy_url_2
    );

    let _m = server
        .mock(
            "GET",
            Matcher::Regex(r"^/find-install-strategy".to_string()),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(get_body)
        .create_async()
        .await;

    let _s = server
        .mock("GET", "/sample-strategy.sh")
        .with_status(200)
        .with_body(
            r#"#!/bin/sh
          output_params="{\"output_params\": {\"url\": \"http://localhost:8080\"}}"
          echo $output_params > "${SMP_OUTPUT_PATH}"
          "#,
        )
        .create_async()
        .await;

    let _s2 = server
        .mock("GET", "/sample-strategy2.sh")
        .with_status(200)
        .with_body(
            r#"#!/bin/sh
          output_params="{\"output_params\": {\"ip\": \"127.0.0.1\"}}"
          echo $output_params > "${SMP_OUTPUT_PATH}"
          "#,
        )
        .create_async()
        .await;

    let registry_config = RegistryConfig::new(server.url());

    let empty_user_params = HashMap::new();

    let args = RunArguments {
        application: "Suri",
        infra: "AWS",
        user_params: &empty_user_params,
        registry_config: &registry_config,
    };

    let mut buffer = Vec::new();

    let config = AppConfig::new("dev", "test", false, false);

    let env_vars = spm_runner::commands::install::run(args, config, &mut buffer)
        .await
        .unwrap_or_default();

    let output = String::from_utf8_lossy(&buffer);

    _m.assert_async().await;
    _s.assert_async().await;
    _s2.assert_async().await;

    assert_ne!(output.find("Using strategy 'Some title 1'"), None);
    assert_ne!(output.find("Using strategy 'Some title 2'"), None);
    assert_eq!(
        env_vars.get("URL"),
        Some(&"http://localhost:8080".to_string())
    );
    assert_eq!(env_vars.get("IP"), Some(&"127.0.0.1".to_string()));
}
