use super::{model::LOCAL_PROVIDER_ID, types::*};

pub const MAX_PROVIDERS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub local: bool,
    pub online: bool,
    pub capabilities: u32,
    pub privacy_floor: PrivacyPolicy,
    pub latency_class: u8,
    pub power_class: u8,
    pub quality_class: u8,
}

pub struct ProviderRouter {
    providers: [Option<ProviderDescriptor>; MAX_PROVIDERS],
}

impl ProviderRouter {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a provider-neutral bounded router with no implicit remote provider.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            providers: [None; MAX_PROVIDERS],
        }
    }

    // ------------------------=
    // FUNC: register
    // DESC: Registers a provider descriptor behind the Infinity AI abstraction.
    // ------------------=
    pub fn register(&mut self, provider: ProviderDescriptor) -> Result<(), AiError> {
        if let Some(existing) = self
            .providers
            .iter_mut()
            .flatten()
            .find(|p| p.id == provider.id)
        {
            *existing = provider;
            return Ok(());
        }
        let slot = self
            .providers
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(AiError::RegistryFull)?;
        *slot = Some(provider);
        Ok(())
    }

    // ------------------------=
    // FUNC: select
    // DESC: Selects an eligible provider while enforcing locality and privacy policy.
    // ------------------=
    pub fn select(
        &self,
        request: &ModelExecutionRequest<'_>,
    ) -> Result<ProviderDescriptor, AiError> {
        let mut selected: Option<ProviderDescriptor> = None;
        let mut remote_requires_approval = false;
        for provider in self.providers.iter().flatten().copied() {
            if !provider.online
                || provider.capabilities & request.capability_class != request.capability_class
            {
                continue;
            }
            if privacy_rank(request.privacy_policy) > privacy_rank(provider.privacy_floor) {
                continue;
            }
            if matches!(
                request.provider_policy,
                ProviderPolicy::LocalOnly | ProviderPolicy::PrivateDataLocalOnly
            ) && !provider.local
            {
                continue;
            }
            if matches!(
                request.privacy_policy,
                PrivacyPolicy::Personal | PrivacyPolicy::Secret
            ) && !provider.local
            {
                continue;
            }
            if !provider.local && request.provider_policy == ProviderPolicy::AskBeforeRemote {
                remote_requires_approval = true;
                continue;
            }
            selected = choose(selected, provider, request.provider_policy);
        }
        selected.ok_or(if remote_requires_approval {
            AiError::RemoteApprovalRequired
        } else {
            AiError::ProviderUnavailable
        })
    }

    // ------------------------=
    // FUNC: inspect
    // DESC: Returns provider metadata for diagnostics without exposing credentials.
    // ------------------=
    pub fn inspect(&self, id: ProviderId) -> Option<&ProviderDescriptor> {
        self.providers.iter().flatten().find(|p| p.id == id)
    }
}

// ------------------------=
// FUNC: privacy_rank
// DESC: Orders privacy classes for provider eligibility without relying on enum layout.
// ------------------=
const fn privacy_rank(policy: PrivacyPolicy) -> u8 {
    match policy {
        PrivacyPolicy::Public => 0,
        PrivacyPolicy::SystemMetadata => 1,
        PrivacyPolicy::Personal => 2,
        PrivacyPolicy::Secret => 3,
    }
}

// ------------------------=
// FUNC: choose
// DESC: Compares eligible providers according to an application-independent routing policy.
// ------------------=
fn choose(
    current: Option<ProviderDescriptor>,
    candidate: ProviderDescriptor,
    policy: ProviderPolicy,
) -> Option<ProviderDescriptor> {
    let Some(existing) = current else {
        return Some(candidate);
    };
    let candidate_wins = match policy {
        ProviderPolicy::PreferLocal
        | ProviderPolicy::LocalOnly
        | ProviderPolicy::PrivateDataLocalOnly => candidate.local && !existing.local,
        ProviderPolicy::LowestLatency => candidate.latency_class < existing.latency_class,
        ProviderPolicy::LowestPower => candidate.power_class < existing.power_class,
        ProviderPolicy::HighestQuality => candidate.quality_class > existing.quality_class,
        ProviderPolicy::RemoteAllowed | ProviderPolicy::AskBeforeRemote => false,
    };
    Some(if candidate_wins { candidate } else { existing })
}

// ------------------------=
// FUNC: local_provider
// DESC: Describes the tested offline CPU provider.
// ------------------=
pub const fn local_provider() -> ProviderDescriptor {
    ProviderDescriptor {
        id: LOCAL_PROVIDER_ID,
        local: true,
        online: true,
        capabilities: CAP_INTENT_RESOLUTION | CAP_CLASSIFICATION,
        privacy_floor: PrivacyPolicy::Secret,
        latency_class: 1,
        power_class: 2,
        quality_class: 1,
    }
}
