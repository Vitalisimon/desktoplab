#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ServiceKind {
    Storage,
    Policy,
    Registry,
    Workspace,
    Runtime,
    Model,
    Session,
    Approval,
    JobsAndEvents,
}

impl ServiceKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Storage => "storage",
            Self::Policy => "policy",
            Self::Registry => "registry",
            Self::Workspace => "workspace",
            Self::Runtime => "runtime",
            Self::Model => "model",
            Self::Session => "session",
            Self::Approval => "approval",
            Self::JobsAndEvents => "jobs-and-events",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceRequirement {
    Required,
    Optional,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServiceHealth {
    Healthy,
    Degraded(String),
    Failed(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceDescriptor {
    kind: ServiceKind,
    requirement: ServiceRequirement,
    health: ServiceHealth,
}

impl ServiceDescriptor {
    #[must_use]
    pub fn required(kind: ServiceKind) -> Self {
        Self::new(kind, ServiceRequirement::Required)
    }

    #[must_use]
    pub fn optional(kind: ServiceKind) -> Self {
        Self::new(kind, ServiceRequirement::Optional)
    }

    #[must_use]
    pub fn with_health(mut self, health: ServiceHealth) -> Self {
        self.health = health;
        self
    }

    fn new(kind: ServiceKind, requirement: ServiceRequirement) -> Self {
        Self {
            kind,
            requirement,
            health: ServiceHealth::Healthy,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceReadiness {
    Ready,
    ReadyDegraded,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendServices {
    services: Vec<ServiceDescriptor>,
    startup_order: Vec<ServiceKind>,
    shutdown_requested: bool,
}

impl BackendServices {
    #[must_use]
    pub fn new(services: Vec<ServiceDescriptor>) -> Self {
        let startup_order = ordered_kinds(&services);
        Self {
            services,
            startup_order,
            shutdown_requested: false,
        }
    }

    #[must_use]
    pub fn startup_order(&self) -> &[ServiceKind] {
        &self.startup_order
    }

    #[must_use]
    pub fn readiness(&self) -> ServiceReadiness {
        if self.services.iter().any(|service| {
            service.requirement == ServiceRequirement::Required
                && matches!(service.health, ServiceHealth::Failed(_))
        }) {
            return ServiceReadiness::Blocked;
        }

        if self
            .services
            .iter()
            .any(|service| matches!(service.health, ServiceHealth::Degraded(_)))
        {
            return ServiceReadiness::ReadyDegraded;
        }

        ServiceReadiness::Ready
    }

    #[must_use]
    pub fn readiness_reasons(&self) -> Vec<String> {
        self.services
            .iter()
            .filter_map(|service| match (&service.requirement, &service.health) {
                (ServiceRequirement::Required, ServiceHealth::Failed(reason)) => Some(format!(
                    "required service {} failed: {reason}",
                    service.kind.as_str()
                )),
                (ServiceRequirement::Optional, ServiceHealth::Degraded(reason)) => Some(format!(
                    "optional service {} degraded: {reason}",
                    service.kind.as_str()
                )),
                _ => None,
            })
            .collect()
    }

    pub fn request_shutdown(&mut self) {
        self.shutdown_requested = true;
    }

    #[must_use]
    pub fn shutdown_order(&self) -> Vec<ServiceKind> {
        let mut order = self.startup_order.clone();
        order.reverse();
        order
    }

    #[must_use]
    pub fn drains_jobs_and_events(&self) -> bool {
        self.shutdown_requested && self.startup_order.contains(&ServiceKind::JobsAndEvents)
    }
}

fn ordered_kinds(services: &[ServiceDescriptor]) -> Vec<ServiceKind> {
    let preferred = [
        ServiceKind::Storage,
        ServiceKind::Policy,
        ServiceKind::Registry,
        ServiceKind::Workspace,
        ServiceKind::Runtime,
        ServiceKind::Model,
        ServiceKind::Session,
        ServiceKind::Approval,
        ServiceKind::JobsAndEvents,
    ];

    preferred
        .into_iter()
        .filter(|kind| services.iter().any(|service| service.kind == *kind))
        .collect()
}
