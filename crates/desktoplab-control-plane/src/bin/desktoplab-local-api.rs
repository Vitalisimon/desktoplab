use desktoplab_control_plane::{
    ControlPlane, ControlPlaneHttpServer, HttpServerConfig, LocalApiAuth, LocalApiRouter,
    LocalAuthToken, VersionInfo, bind_default_local_api_server,
};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    let unsafe_no_auth = std::env::args().any(|arg| arg == "--unsafe-no-auth");
    let explicit_token = arg_value("--auth-token");
    let test_router = test_router_from_env();
    let server = if let Some(router) = test_router {
        bind_configured_server(unsafe_no_auth, explicit_token.clone(), router)
            .expect("failed to bind configured DesktopLab local API on 127.0.0.1:1421")
    } else if unsafe_no_auth {
        desktoplab_control_plane::bind_unsafe_dev_local_api_server(1421)
            .expect("failed to bind unsafe DesktopLab local API on 127.0.0.1:1421")
    } else if let Some(token) = explicit_token {
        desktoplab_control_plane::bind_authenticated_local_api_server(
            1421,
            LocalAuthToken::explicit_for_test(token),
        )
        .expect("failed to bind authenticated DesktopLab local API on 127.0.0.1:1421")
    } else {
        bind_default_local_api_server(1421)
            .expect("failed to bind DesktopLab local API on 127.0.0.1:1421")
    };
    let address = server.local_addr();
    let _handle = server.spawn();

    if unsafe_no_auth {
        println!("DesktopLab local API listening on http://{address} with UNSAFE auth disabled");
    } else {
        println!("DesktopLab local API listening on http://{address} with desktop auth required");
    }
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

fn bind_configured_server(
    unsafe_no_auth: bool,
    explicit_token: Option<String>,
    router: LocalApiRouter,
) -> Result<ControlPlaneHttpServer, desktoplab_control_plane::HttpServerError> {
    let control_plane = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
        "0.1.0", "v1",
    ))));
    control_plane
        .lock()
        .expect("control plane lock should not be poisoned")
        .mark_ready();
    let config = if unsafe_no_auth {
        HttpServerConfig::loopback(1421)?
    } else if let Some(token) = explicit_token {
        HttpServerConfig::loopback(1421)?.with_auth(LocalApiAuth::required(
            LocalAuthToken::explicit_for_test(token),
        ))
    } else {
        HttpServerConfig::loopback(1421)?
            .with_auth(LocalApiAuth::required(LocalAuthToken::for_desktop_session()))
    };
    ControlPlaneHttpServer::bind_with_router(config, control_plane, router)
}

#[cfg(debug_assertions)]
fn test_router_from_env() -> Option<LocalApiRouter> {
    test_router_from_values(
        std::env::var("DESKTOPLAB_AGENT_BACKEND_MODE").ok(),
        std::env::var("DESKTOPLAB_TEST_CONTROLS").ok().as_deref() == Some("1"),
        std::env::var("DESKTOPLAB_AGENT_BACKEND_OUTPUT").ok(),
    )
}

#[cfg(not(debug_assertions))]
fn test_router_from_env() -> Option<LocalApiRouter> {
    None
}

#[cfg(debug_assertions)]
fn test_router_from_values(
    mode: Option<String>,
    test_controls_enabled: bool,
    output: Option<String>,
) -> Option<LocalApiRouter> {
    if !test_controls_enabled {
        return None;
    }
    let mode = mode?;
    let mut router = LocalApiRouter::default();
    match mode.as_str() {
        "fail" => router.fail_agent_backend_for_test(),
        "deterministic" => router.complete_agent_backend_for_test(
            output.unwrap_or_else(|| "Deterministic agent response.".to_string()),
        ),
        _ => return None,
    }
    if test_controls_enabled {
        router.enable_test_controls_for_dev_server();
        router.set_runtime_verification_for_test(true, "dev test control runtime");
        router.set_local_model_inventory_for_test(&["gemma4:12b"]);
    }
    Some(router)
}

fn arg_value(name: &str) -> Option<String> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == name {
            return args.next();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::test_router_from_values;
    use serde_json::Value;

    #[test]
    fn deterministic_agent_backend_env_requires_test_controls_opt_in() {
        assert!(test_router_from_values(Some("deterministic".to_string()), false, None).is_none());
        assert!(test_router_from_values(Some("deterministic".to_string()), true, None).is_some());
    }

    #[test]
    fn dev_test_router_setup_does_not_depend_on_host_runtime_inventory() {
        let mut router = test_router_from_values(Some("fail".to_string()), true, None).unwrap();
        let reset = router.route("POST", "/v1/test/reset", "{}").unwrap();
        assert_eq!(reset.status(), "200 OK");
        router.route(
            "POST",
            "/v1/setup/accept",
            r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
        );
        let runtime = router
            .route("POST", "/v1/runtimes/runtime.ollama/verify", "{}")
            .unwrap();
        let model = router
            .route("POST", "/v1/models/model.gemma4-12b-q4/verify", "{}")
            .unwrap();

        assert_eq!(
            serde_json::from_str::<Value>(runtime.body()).unwrap()["verificationState"],
            "verified"
        );
        assert_eq!(
            serde_json::from_str::<Value>(model.body()).unwrap()["verificationState"],
            "verified"
        );
    }
}
