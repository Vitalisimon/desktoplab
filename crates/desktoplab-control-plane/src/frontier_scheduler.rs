use std::collections::BTreeMap;

use desktoplab_policy::{
    FrontierAccessPolicy, FrontierAccessRequest, FrontierDeploymentMode, FrontierResourceAction,
    WorkspaceAccess,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrontierPartitionKind {
    WholeHost,
    MigSlice {
        gpu_uuid: String,
        profile: String,
    },
    EquivalentPartition {
        provider: String,
        partition_type: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierResourcePartition {
    id: String,
    kind: FrontierPartitionKind,
    memory_gb: u32,
    supported_runtimes: Vec<String>,
}

impl FrontierResourcePartition {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        kind: FrontierPartitionKind,
        memory_gb: u32,
        supported_runtimes: &[&str],
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            memory_gb,
            supported_runtimes: supported_runtimes.iter().map(ToString::to_string).collect(),
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn kind(&self) -> &FrontierPartitionKind {
        &self.kind
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierScheduleRequest {
    pub actor_user_id: String,
    pub session_id: String,
    pub session_owner_user_id: String,
    pub workspace_id: String,
    pub workspace_owner_user_id: String,
    pub workspace_access: WorkspaceAccess,
    pub model_id: String,
    pub runtime_id: String,
    pub required_memory_gb: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierResourceLease {
    partition_id: String,
    user_id: String,
    session_id: String,
    workspace_id: String,
    model_id: String,
}

impl FrontierResourceLease {
    #[must_use]
    pub fn partition_id(&self) -> &str {
        &self.partition_id
    }

    #[must_use]
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrontierScheduleDecision {
    Assigned(FrontierResourceLease),
    Queued { session_id: String },
    Rejected { reason: &'static str },
}

#[derive(Clone, Debug)]
pub struct FrontierResourceScheduler {
    policy: FrontierAccessPolicy,
    partitions: Vec<FrontierResourcePartition>,
    leases: BTreeMap<String, FrontierResourceLease>,
    queued_sessions: Vec<String>,
}

impl FrontierResourceScheduler {
    #[must_use]
    pub fn new(
        mode: FrontierDeploymentMode,
        mut partitions: Vec<FrontierResourcePartition>,
    ) -> Self {
        partitions.sort_by_key(|partition| partition.memory_gb);
        Self {
            policy: FrontierAccessPolicy::new(mode),
            partitions,
            leases: BTreeMap::new(),
            queued_sessions: Vec::new(),
        }
    }

    pub fn schedule(&mut self, request: FrontierScheduleRequest) -> FrontierScheduleDecision {
        let access = self.policy.evaluate(&FrontierAccessRequest::new(
            &request.actor_user_id,
            &request.session_owner_user_id,
            &request.workspace_owner_user_id,
            request.workspace_access,
            FrontierResourceAction::UseWorkspace,
        ));
        if !access.allowed() {
            return FrontierScheduleDecision::Rejected {
                reason: "workspace_or_session_isolation_denied",
            };
        }
        if self
            .leases
            .values()
            .any(|lease| lease.session_id == request.session_id)
        {
            return FrontierScheduleDecision::Rejected {
                reason: "session_already_scheduled",
            };
        }
        let compatible = |partition: &FrontierResourcePartition| {
            partition.memory_gb >= request.required_memory_gb
                && partition.supported_runtimes.contains(&request.runtime_id)
        };
        if !self.partitions.iter().any(&compatible) {
            return FrontierScheduleDecision::Rejected {
                reason: "no_compatible_partition",
            };
        }
        let available = self
            .partitions
            .iter()
            .filter(|partition| compatible(partition))
            .find(|candidate| !self.leases.contains_key(&candidate.id));
        let Some(partition) = available else {
            if !self.queued_sessions.contains(&request.session_id) {
                self.queued_sessions.push(request.session_id.clone());
            }
            return FrontierScheduleDecision::Queued {
                session_id: request.session_id,
            };
        };
        let lease = FrontierResourceLease {
            partition_id: partition.id.clone(),
            user_id: request.actor_user_id,
            session_id: request.session_id,
            workspace_id: request.workspace_id,
            model_id: request.model_id,
        };
        self.leases.insert(partition.id.clone(), lease.clone());
        FrontierScheduleDecision::Assigned(lease)
    }

    pub fn release(&mut self, actor_user_id: &str, session_id: &str) -> bool {
        let partition_id = self.leases.iter().find_map(|(partition_id, lease)| {
            (lease.session_id == session_id && lease.user_id == actor_user_id)
                .then(|| partition_id.clone())
        });
        partition_id
            .and_then(|partition_id| self.leases.remove(&partition_id))
            .is_some()
    }

    #[must_use]
    pub fn can_reuse_model_cache(
        &self,
        actor_user_id: &str,
        session_owner_user_id: &str,
        cache_owner_user_id: &str,
        checksum_verified: bool,
        contains_user_material: bool,
    ) -> bool {
        self.policy
            .evaluate(&FrontierAccessRequest::new(
                actor_user_id,
                session_owner_user_id,
                cache_owner_user_id,
                WorkspaceAccess::Owner,
                FrontierResourceAction::ReadModelCache {
                    checksum_verified,
                    contains_user_material,
                },
            ))
            .allowed()
    }

    #[must_use]
    pub fn can_resolve_approval(&self, actor_user_id: &str, session_owner_user_id: &str) -> bool {
        self.policy
            .evaluate(&FrontierAccessRequest::new(
                actor_user_id,
                session_owner_user_id,
                session_owner_user_id,
                WorkspaceAccess::Owner,
                FrontierResourceAction::ResolveApproval,
            ))
            .allowed()
    }

    #[must_use]
    pub fn queued_sessions(&self) -> &[String] {
        &self.queued_sessions
    }
}
