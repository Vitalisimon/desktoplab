use desktoplab_domain::{DeepLinkAction, DeepLinkError, DesktopLabDeepLink};

#[test]
fn accepts_supported_desktoplab_deep_links() {
    assert_eq!(
        DesktopLabDeepLink::parse("desktoplab://setup")
            .expect("setup link should parse")
            .action(),
        &DeepLinkAction::Setup
    );

    assert_eq!(
        DesktopLabDeepLink::parse("desktoplab://open/repository?path=/Users/example/project")
            .expect("repository link should parse")
            .action(),
        &DeepLinkAction::OpenRepository {
            path: Some("/Users/example/project".to_string())
        }
    );

    assert_eq!(
        DesktopLabDeepLink::parse("desktoplab://thread/session-123")
            .expect("thread link should parse")
            .action(),
        &DeepLinkAction::Thread {
            thread_id: "session-123".to_string()
        }
    );

    assert_eq!(
        DesktopLabDeepLink::parse(
            "desktoplab://provider/callback?provider=openai&state=nonce&code=oauth-code"
        )
        .expect("provider callback should parse")
        .action(),
        &DeepLinkAction::ProviderCallback {
            provider: "openai".to_string(),
            state: "nonce".to_string(),
            code: Some("oauth-code".to_string())
        }
    );
}

#[test]
fn rejects_unknown_or_unsafe_deep_links() {
    assert_eq!(
        DesktopLabDeepLink::parse("https://desktoplab.ai/setup"),
        Err(DeepLinkError::UnsupportedScheme)
    );
    assert_eq!(
        DesktopLabDeepLink::parse("desktoplab://unknown/action"),
        Err(DeepLinkError::UnknownAction)
    );
    assert_eq!(
        DesktopLabDeepLink::parse("desktoplab://open/repository?path=/tmp/../secret"),
        Err(DeepLinkError::UnsafeRepositoryPath)
    );
    assert_eq!(
        DesktopLabDeepLink::parse("desktoplab://provider/callback?provider=openai"),
        Err(DeepLinkError::MissingRequiredQuery)
    );
}

#[test]
fn deep_link_contract_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-domain/tests/deep_link_contract.rs",
        include_str!("deep_link_contract.rs"),
        120,
    )
    .expect("deep link contract test should stay focused");
}
