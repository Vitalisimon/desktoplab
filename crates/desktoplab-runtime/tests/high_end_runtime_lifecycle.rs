use desktoplab_runtime::{
    DesktopLabOwnedRuntimeProcess, HighEndLaunchSpec, HighEndProcessError, HighEndRuntimeFamily,
    HighEndRuntimeHealthEvidence, HighEndRuntimeLifecycle, HighEndRuntimeLifecycleError,
    RuntimeEndpointSpec, high_end_runtime_contracts,
};
use xtask::check_logical_line_limit;

#[cfg(unix)]
#[test]
fn approved_launch_creates_and_stops_only_a_desktoplab_owned_process() {
    let contract = contract(HighEndRuntimeFamily::Vllm);
    let spec = HighEndLaunchSpec::new("/bin/sleep").arg("30");
    let mut process = DesktopLabOwnedRuntimeProcess::launch_after_approval(&contract, &spec)
        .expect("approved launch should spawn the process directly without a shell");

    assert_eq!(process.runtime_id(), contract.runtime_id());
    assert!(process.try_status().unwrap().is_none());
    let status = process.stop_owned().unwrap();
    assert!(!status.success());
}

#[test]
fn attach_only_contract_rejects_launch_before_process_execution() {
    let contract = contract(HighEndRuntimeFamily::CustomLan);
    let spec = HighEndLaunchSpec::new("this-program-must-never-run");

    let error = DesktopLabOwnedRuntimeProcess::launch_after_approval(&contract, &spec).unwrap_err();
    assert_eq!(
        error,
        HighEndProcessError::AttachOnly(contract.runtime_id().clone())
    );
}

#[test]
fn lifecycle_refuses_to_stop_user_owned_endpoint() {
    let contract = contract(HighEndRuntimeFamily::Vllm);
    let endpoint = RuntimeEndpointSpec::local("http://127.0.0.1:8000", "model.large").unwrap();
    let mut lifecycle = HighEndRuntimeLifecycle::attached(
        contract.clone(),
        endpoint,
        HighEndRuntimeHealthEvidence::failed("not probed"),
    );

    assert_eq!(
        lifecycle.stop_owned(),
        Err(HighEndRuntimeLifecycleError::UserOwned(
            contract.runtime_id().clone()
        ))
    );
}

#[test]
fn high_end_process_source_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/high_end_process.rs",
        include_str!("../src/high_end_process.rs"),
        180,
    )
    .expect("high-end process ownership should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/high_end_health.rs",
        include_str!("../src/high_end_health.rs"),
        380,
    )
    .expect("high-end health source should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/high_end_http.rs",
        include_str!("../src/high_end_http.rs"),
        180,
    )
    .expect("high-end HTTP probe should stay focused");
}

fn contract(family: HighEndRuntimeFamily) -> desktoplab_runtime::HighEndRuntimeContract {
    high_end_runtime_contracts()
        .into_iter()
        .find(|contract| contract.family() == family)
        .unwrap()
}
