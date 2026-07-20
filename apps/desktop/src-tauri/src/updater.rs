#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacosUpdateChannel {
    Dev,
    Beta,
    Stable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacosUpdateSignature {
    Unsigned,
    Signed,
    Notarized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MacosUpdateCandidate {
    channel: MacosUpdateChannel,
    signature: MacosUpdateSignature,
    local_fixture: bool,
}

impl MacosUpdateCandidate {
    #[must_use]
    pub const fn new(
        channel: MacosUpdateChannel,
        signature: MacosUpdateSignature,
        local_fixture: bool,
    ) -> Self {
        Self {
            channel,
            signature,
            local_fixture,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacosUpdateDecision {
    Allowed,
    Blocked(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacosUpdateAttempt {
    Updated,
    ExistingAppStillUsable,
}

#[must_use]
pub const fn evaluate_macos_update(candidate: MacosUpdateCandidate) -> MacosUpdateDecision {
    match (candidate.channel, candidate.signature, candidate.local_fixture) {
        (MacosUpdateChannel::Dev, MacosUpdateSignature::Unsigned, true) => {
            MacosUpdateDecision::Allowed
        }
        (MacosUpdateChannel::Dev, MacosUpdateSignature::Unsigned, false) => {
            MacosUpdateDecision::Blocked("unsigned dev update must be a local fixture")
        }
        (MacosUpdateChannel::Dev, _, _) => MacosUpdateDecision::Allowed,
        (MacosUpdateChannel::Beta, MacosUpdateSignature::Unsigned, _) => {
            MacosUpdateDecision::Blocked("beta macOS updates require signing")
        }
        (MacosUpdateChannel::Beta, _, _) => MacosUpdateDecision::Allowed,
        (MacosUpdateChannel::Stable, MacosUpdateSignature::Notarized, _) => {
            MacosUpdateDecision::Allowed
        }
        (MacosUpdateChannel::Stable, MacosUpdateSignature::Unsigned, _) => {
            MacosUpdateDecision::Blocked("stable macOS updates require notarization")
        }
        (MacosUpdateChannel::Stable, MacosUpdateSignature::Signed, _) => {
            MacosUpdateDecision::Blocked("stable macOS updates require notarization")
        }
    }
}

#[must_use]
pub const fn finalize_macos_update_attempt(install_succeeded: bool) -> MacosUpdateAttempt {
    if install_succeeded {
        MacosUpdateAttempt::Updated
    } else {
        MacosUpdateAttempt::ExistingAppStillUsable
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateChannel {
    Dev,
    Beta,
    Stable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateManifestTrust {
    LocalFixture,
    UnsignedRemote,
    Signed,
    Notarized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateUiCheckState {
    NotChecked,
    CheckFailed,
    VerifiedNoUpdate,
    VerifiedUpdateAvailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateUiInteraction {
    ReadOnly,
    Actionable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateInstallResult {
    Installed,
    RecoverableFailure(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateDeliveryState {
    Disabled,
    EnabledWithVerifiedKey,
}

#[must_use]
pub const fn evaluate_update_manifest(
    delivery: UpdateDeliveryState,
    channel: UpdateChannel,
    trust: UpdateManifestTrust,
) -> Result<(), &'static str> {
    if matches!(delivery, UpdateDeliveryState::Disabled) {
        return Err("update delivery is disabled for this build");
    }
    match (channel, trust) {
        (UpdateChannel::Dev, UpdateManifestTrust::LocalFixture) => Ok(()),
        (_, UpdateManifestTrust::UnsignedRemote) => {
            if matches!(channel, UpdateChannel::Dev) {
                Err("unsigned remote update manifest is not trusted")
            } else {
                Err("beta and stable update manifests require signing")
            }
        }
        (UpdateChannel::Beta | UpdateChannel::Stable, UpdateManifestTrust::LocalFixture) => {
            Err("beta and stable update manifests require signing")
        }
        (_, UpdateManifestTrust::Signed | UpdateManifestTrust::Notarized) => Ok(()),
    }
}

#[must_use]
pub fn verify_update_artifact(expected_sha256: &str, actual_sha256: &str) -> Result<(), &'static str> {
    if expected_sha256 == actual_sha256 {
        Ok(())
    } else {
        Err("update artifact hash mismatch")
    }
}

#[must_use]
pub const fn finalize_update_install(install_succeeded: bool) -> UpdateInstallResult {
    if install_succeeded {
        UpdateInstallResult::Installed
    } else {
        UpdateInstallResult::RecoverableFailure("existing app remains usable")
    }
}

#[must_use]
pub const fn update_ui_state(state: UpdateUiCheckState) -> UpdateUiInteraction {
    match state {
        UpdateUiCheckState::VerifiedUpdateAvailable => UpdateUiInteraction::Actionable,
        UpdateUiCheckState::NotChecked
        | UpdateUiCheckState::CheckFailed
        | UpdateUiCheckState::VerifiedNoUpdate => UpdateUiInteraction::ReadOnly,
    }
}
