use desktoplab_backend_services::{
    BackendServices, ServiceDescriptor, ServiceHealth, ServiceKind, ServiceReadiness,
};
use xtask::check_logical_line_limit;

#[test]
fn services_initialize_in_dependency_order() {
    let services = BackendServices::new(vec![
        ServiceDescriptor::required(ServiceKind::Policy),
        ServiceDescriptor::required(ServiceKind::Storage),
        ServiceDescriptor::required(ServiceKind::Registry),
        ServiceDescriptor::required(ServiceKind::Workspace),
        ServiceDescriptor::required(ServiceKind::Runtime),
        ServiceDescriptor::required(ServiceKind::Model),
        ServiceDescriptor::required(ServiceKind::Session),
        ServiceDescriptor::required(ServiceKind::Approval),
        ServiceDescriptor::required(ServiceKind::JobsAndEvents),
    ]);

    assert_eq!(
        services.startup_order(),
        &[
            ServiceKind::Storage,
            ServiceKind::Policy,
            ServiceKind::Registry,
            ServiceKind::Workspace,
            ServiceKind::Runtime,
            ServiceKind::Model,
            ServiceKind::Session,
            ServiceKind::Approval,
            ServiceKind::JobsAndEvents,
        ]
    );
}

#[test]
fn failed_required_service_blocks_readiness() {
    let services = BackendServices::new(vec![
        ServiceDescriptor::required(ServiceKind::Storage)
            .with_health(ServiceHealth::Failed("migration failed".to_string())),
        ServiceDescriptor::required(ServiceKind::Policy),
    ]);

    let readiness = services.readiness();

    assert_eq!(readiness, ServiceReadiness::Blocked);
    assert!(
        services
            .readiness_reasons()
            .contains(&"required service storage failed: migration failed".to_string())
    );
}

#[test]
fn degraded_optional_service_is_reported_explicitly() {
    let services = BackendServices::new(vec![
        ServiceDescriptor::required(ServiceKind::Storage),
        ServiceDescriptor::optional(ServiceKind::Registry).with_health(ServiceHealth::Degraded(
            "offline, using last-known-good".to_string(),
        )),
    ]);

    assert_eq!(services.readiness(), ServiceReadiness::ReadyDegraded);
    assert!(services.readiness_reasons().contains(
        &"optional service registry degraded: offline, using last-known-good".to_string()
    ));
}

#[test]
fn shutdown_drains_jobs_and_events_before_exit() {
    let mut services = BackendServices::new(vec![
        ServiceDescriptor::required(ServiceKind::Storage),
        ServiceDescriptor::required(ServiceKind::JobsAndEvents),
    ]);

    services.request_shutdown();

    assert_eq!(
        services.shutdown_order(),
        &[ServiceKind::JobsAndEvents, ServiceKind::Storage]
    );
    assert!(services.drains_jobs_and_events());
}

#[test]
fn service_composition_source_stays_below_initial_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-backend-services/src/lib.rs",
            include_str!("../src/lib.rs"),
            250,
        ),
        (
            "crates/desktoplab-backend-services/src/composition.rs",
            include_str!("../src/composition.rs"),
            250,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("backend service composition source should stay below the line-count guard");
    }
}
