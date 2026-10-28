//! Ejecutor secuencial del plan. Cada solicitud vuelve a pasar por autorización.

use crate::{
    DnsAuthorizationPort, DnsCancellation, DnsQueryPlan, DnsQueryStage, DnsResolutionError,
    DnsResolutionResult, DnsResolverPort, ResolveDns,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsPlanQueryResult {
    pub stage: DnsQueryStage,
    pub resolution: DnsResolutionResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsPlanTermination {
    Completed,
    Stopped(DnsResolutionError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsPlanExecution {
    pub results: Vec<DnsPlanQueryResult>,
    pub termination: DnsPlanTermination,
}

pub struct ExecuteDnsPlan<'a, R, A> {
    resolver: &'a R,
    authorization: &'a A,
}

impl<'a, R, A> ExecuteDnsPlan<'a, R, A>
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

    /// Ejecuta en orden, conserva respuestas parciales y se detiene ante un
    /// error de autorización, cancelación o expiración. Los fallos DNS tipados
    /// siguen como resultados de consulta y no impiden registrar las demás.
    pub async fn execute(
        &self,
        plan: &DnsQueryPlan,
        cancellation: &DnsCancellation,
    ) -> DnsPlanExecution {
        let mut results = Vec::with_capacity(plan.queries().len());
        let resolver = ResolveDns::new(self.resolver, self.authorization);

        for planned in plan.queries() {
            match resolver
                .execute(planned.request().clone(), cancellation)
                .await
            {
                Ok(resolution) => results.push(DnsPlanQueryResult {
                    stage: planned.stage(),
                    resolution,
                }),
                Err(error) => {
                    return DnsPlanExecution {
                        results,
                        termination: DnsPlanTermination::Stopped(error),
                    };
                }
            }
        }

        DnsPlanExecution {
            results,
            termination: DnsPlanTermination::Completed,
        }
    }
}
