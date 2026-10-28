//! Ejecutor secuencial SRV que conserva resultados parciales y exige permiso
//! vigente para cada propietario antes de llamar al resolver.

use crate::{
    DnsAuthorizationPort, DnsCancellation, DnsResolutionError, DnsResolutionResult,
    DnsResolverPort, DnsServiceQueryPlan, ResolveDns,
};
use chef_core::Clock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsServiceQueryResult {
    pub owner_name: String,
    pub source_evidence_id: Option<String>,
    pub observed_at: String,
    pub resolution: DnsResolutionResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsServicePlanTermination {
    Completed,
    Stopped(DnsResolutionError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsServicePlanExecution {
    pub results: Vec<DnsServiceQueryResult>,
    pub termination: DnsServicePlanTermination,
}

pub struct ExecuteDnsServicePlan<'a, R, A> {
    resolver: &'a R,
    authorization: &'a A,
}

impl<'a, R, A> ExecuteDnsServicePlan<'a, R, A>
where
    R: DnsResolverPort,
    A: DnsAuthorizationPort,
{
    pub fn new(resolver: &'a R, authorization: &'a A) -> Self {
        Self {
            resolver,
            authorization,
        }
    }

    /// Ejecuta en orden. Un rechazo, vencimiento o cancelación detiene las
    /// consultas siguientes y conserva las respuestas ya obtenidas.
    pub async fn execute(
        &self,
        plan: &DnsServiceQueryPlan,
        cancellation: &DnsCancellation,
        clock: &impl Clock,
    ) -> DnsServicePlanExecution {
        let mut results = Vec::with_capacity(plan.candidates().len());
        let resolver = ResolveDns::new(self.resolver, self.authorization);

        for candidate in plan.candidates() {
            let request = match candidate.request() {
                Ok(request) => request,
                Err(error) => {
                    return DnsServicePlanExecution {
                        results,
                        termination: DnsServicePlanTermination::Stopped(error),
                    };
                }
            };
            match resolver.execute(request, cancellation).await {
                Ok(resolution) => results.push(DnsServiceQueryResult {
                    owner_name: candidate.owner_name().to_owned(),
                    source_evidence_id: candidate.source_evidence_id().map(str::to_owned),
                    observed_at: clock.now(),
                    resolution,
                }),
                Err(error) => {
                    return DnsServicePlanExecution {
                        results,
                        termination: DnsServicePlanTermination::Stopped(error),
                    };
                }
            }
        }

        DnsServicePlanExecution {
            results,
            termination: DnsServicePlanTermination::Completed,
        }
    }
}
