//! Saga orchestration primitives and a coordinator implementation.
//!
//! Defines `Saga` and `SagaStep` traits, execution tracking types, and a
//! `SagaCoordinator` which can run composed sagas with retry/compensation logic.
use async_trait::async_trait;
use std::sync::Arc;

use crate::error::SagaError;

// ---------------------------------------------------------------------------
// SagaStep – one atomic unit of work within a saga
// ---------------------------------------------------------------------------

#[async_trait]
pub trait SagaStep: Send + Sync {
    type Input: Clone + Send + Sync;
    type Output: Send;

    async fn execute(&self, input: Self::Input) -> Result<Self::Output, SagaError>;
    async fn compensate(&self, input: Self::Input) -> Result<(), SagaError>;
    fn step_name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Saga – named composition of steps
// ---------------------------------------------------------------------------

#[async_trait]
pub trait Saga: Send + Sync {
    type Input: Clone + Send + Sync;
    type Output: Send;

    fn saga_id(&self) -> &str;

    fn steps(&self) -> Vec<Arc<dyn SagaStep<Input = Self::Input, Output = ()>>>;

    /// Override for direct execution without SagaCoordinator.
    async fn execute(&self, _input: Self::Input) -> Result<Self::Output, SagaError> {
        Err(SagaError::Aborted(
            "direct execution not implemented; use SagaCoordinator".into(),
        ))
    }

    /// Override for direct compensation without SagaCoordinator.
    async fn compensate(&self, _input: Self::Input) -> Result<(), SagaError> {
        Err(SagaError::Aborted(
            "direct compensation not implemented; use SagaCoordinator".into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Saga execution tracking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum SagaExecutionStatus {
    Running,
    Completed,
    Failed(String),
    Compensating,
    Compensated,
}

#[derive(Debug, Clone)]
pub struct SagaExecution {
    pub saga_id: String,
    pub status: SagaExecutionStatus,
    pub completed_steps: Vec<String>,
    pub started_at: u64,
    pub updated_at: u64,
}

// ---------------------------------------------------------------------------
// SagaCoordinator – distributed transaction engine
// ---------------------------------------------------------------------------

#[cfg(feature = "messaging-bridge")]
pub use nova_messaging::MessageBroker;

#[cfg(feature = "messaging-bridge")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(feature = "messaging-bridge")]
use tokio::sync::RwLock;

#[cfg(feature = "messaging-bridge")]
use tracing::{info, warn};

#[cfg(feature = "messaging-bridge")]
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(feature = "messaging-bridge")]
pub struct SagaCoordinator<M: MessageBroker> {
    messaging: Arc<M>,
    max_retries: u32,
    timeout_ms: u64,
    executions: Arc<RwLock<std::collections::HashMap<String, SagaExecution>>>,
}

#[cfg(feature = "messaging-bridge")]
impl<M: MessageBroker> SagaCoordinator<M> {
    pub fn new(messaging: Arc<M>, max_retries: u32, timeout_ms: u64) -> Self {
        Self {
            messaging,
            max_retries,
            timeout_ms,
            executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn executions(&self) -> &Arc<RwLock<std::collections::HashMap<String, SagaExecution>>> {
        &self.executions
    }

    /// Run a saga: execute steps sequentially. On any step failure, run
    /// compensations in reverse order for previously-completed steps.
    pub async fn run<S: Saga>(&self, saga: &S, input: S::Input) -> Result<(), SagaError>
    where
        S::Input: Clone,
    {
        let saga_id = saga.saga_id().to_string();
        let steps = saga.steps();
        let mut completed: Vec<usize> = Vec::new();

        // Record execution start
        {
            let mut execs = self.executions.write().await;
            execs.insert(
                saga_id.clone(),
                SagaExecution {
                    saga_id: saga_id.clone(),
                    status: SagaExecutionStatus::Running,
                    completed_steps: Vec::new(),
                    started_at: now_ms(),
                    updated_at: now_ms(),
                },
            );
        }

        info!(saga_id = %saga_id, step_count = %steps.len(), "saga started");

        for (i, step) in steps.iter().enumerate() {
            let step_name = step.step_name().to_string();

            self.publish_event(&saga_id, &step_name, "started").await;

            let result = self
                .execute_step_with_retry(step.as_ref(), input.clone())
                .await;

            match result {
                Ok(()) => {
                    completed.push(i);
                    if let Some(exec) = self.executions.write().await.get_mut(&saga_id) {
                        exec.completed_steps.push(step_name.clone());
                        exec.updated_at = now_ms();
                    }
                    self.publish_event(&saga_id, &step_name, "succeeded").await;
                    info!(saga_id = %saga_id, step = %step_name, "step completed");
                }
                Err(e) => {
                    warn!(
                        saga_id = %saga_id,
                        step = %step_name,
                        error = %e,
                        "step failed, starting compensation"
                    );
                    self.publish_event(&saga_id, &step_name, "failed").await;

                    // Compensate in reverse order
                    if let Some(status) = self.executions.write().await.get_mut(&saga_id) {
                        status.status = SagaExecutionStatus::Compensating;
                        status.updated_at = now_ms();
                    }

                    for idx in completed.iter().rev() {
                        let comp_step = &steps[*idx];
                        let comp_name = comp_step.step_name().to_string();
                        self.publish_event(&saga_id, &comp_name, "compensating")
                            .await;

                        match comp_step.compensate(input.clone()).await {
                            Ok(()) => {
                                self.publish_event(&saga_id, &comp_name, "compensated")
                                    .await;
                                info!(
                                    saga_id = %saga_id,
                                    step = %comp_name,
                                    "compensation succeeded"
                                );
                            }
                            Err(ce) => {
                                warn!(
                                    saga_id = %saga_id,
                                    step = %comp_name,
                                    error = %ce,
                                    "compensation failed"
                                );
                                self.publish_event(&saga_id, &comp_name, "compensation_failed")
                                    .await;
                            }
                        }
                    }

                    if let Some(status) = self.executions.write().await.get_mut(&saga_id) {
                        status.status = SagaExecutionStatus::Compensated;
                        status.updated_at = now_ms();
                    }

                    self.publish_event(&saga_id, "saga", "compensated").await;
                    return Err(e);
                }
            }
        }

        // All steps completed successfully
        if let Some(status) = self.executions.write().await.get_mut(&saga_id) {
            status.status = SagaExecutionStatus::Completed;
            status.updated_at = now_ms();
        }

        self.publish_event(&saga_id, "saga", "completed").await;
        info!(saga_id = %saga_id, "saga completed successfully");
        Ok(())
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Execute a single step, retrying up to `max_retries`, with a timeout.
    async fn execute_step_with_retry<I: Clone + Send + Sync>(
        &self,
        step: &dyn SagaStep<Input = I, Output = ()>,
        input: I,
    ) -> Result<(), SagaError> {
        let mut attempt = 0u32;
        loop {
            let timeout = Duration::from_millis(self.timeout_ms);
            let deadline = tokio::time::Instant::now() + timeout;

            let result = tokio::select! {
                biased;
                _ = tokio::time::sleep_until(deadline) => {
                    return Err(SagaError::Timeout {
                        step: step.step_name().to_string(),
                    });
                }
                res = step.execute(input.clone()) => {
                    res.map(|_output| ())
                }
            };

            match result {
                Ok(()) => return Ok(()),
                Err(e) => {
                    attempt += 1;
                    if attempt > self.max_retries {
                        return Err(e);
                    }
                    warn!(
                        step = %step.step_name(),
                        attempt = %attempt,
                        max_retries = %self.max_retries,
                        error = %e,
                        "retrying step"
                    );
                    tokio::time::sleep(Duration::from_millis(100 * attempt as u64)).await;
                }
            }
        }
    }

    /// Publish a saga lifecycle event to topic `saga.{saga_id}.{step}.{status}`.
    async fn publish_event(&self, saga_id: &str, step: &str, status: &str) {
        use nova_messaging::EventEnvelope;
        let topic = format!("saga.{saga_id}.{step}.{status}");
        let payload = serde_json::json!({
            "saga_id": saga_id,
            "step": step,
            "status": status,
            "timestamp_ms": now_ms(),
        });
        let envelope = EventEnvelope::new(
            format!("{saga_id}-{step}-{status}"),
            &topic,
            "saga.event",
            payload,
        );
        if let Err(e) = self.messaging.publish(envelope).await {
            warn!(
                saga_id = %saga_id,
                step = %step,
                status = %status,
                error = %e,
                "failed to publish saga event"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tracing::warn;

    struct PassStep {
        name: String,
    }

    #[async_trait]
    impl SagaStep for PassStep {
        type Input = String;
        type Output = ();
        async fn execute(&self, _input: String) -> Result<(), SagaError> {
            Ok(())
        }
        async fn compensate(&self, _input: String) -> Result<(), SagaError> {
            Ok(())
        }
        fn step_name(&self) -> &str {
            &self.name
        }
    }

    struct FailStep {
        name: String,
    }

    #[async_trait]
    impl SagaStep for FailStep {
        type Input = String;
        type Output = ();
        async fn execute(&self, _input: String) -> Result<(), SagaError> {
            Err(SagaError::StepFailed {
                step: self.name.clone(),
                reason: "intentional failure".into(),
            })
        }
        async fn compensate(&self, _input: String) -> Result<(), SagaError> {
            Ok(())
        }
        fn step_name(&self) -> &str {
            &self.name
        }
    }

    struct BadCompensateStep {
        name: String,
    }

    #[async_trait]
    impl SagaStep for BadCompensateStep {
        type Input = String;
        type Output = ();
        async fn execute(&self, _input: String) -> Result<(), SagaError> {
            Ok(())
        }
        async fn compensate(&self, _input: String) -> Result<(), SagaError> {
            Err(SagaError::CompensationFailed {
                step: self.name.clone(),
                reason: "compensation failure".into(),
            })
        }
        fn step_name(&self) -> &str {
            &self.name
        }
    }

    // Saga wrapper that carries Arc'd steps
    fn make_test_saga(
        steps: Vec<Arc<dyn SagaStep<Input = String, Output = ()>>>,
    ) -> impl Saga<Input = String, Output = ()> {
        struct TestSaga {
            steps: Vec<Arc<dyn SagaStep<Input = String, Output = ()>>>,
        }

        #[async_trait]
        impl Saga for TestSaga {
            type Input = String;
            type Output = ();
            fn saga_id(&self) -> &str {
                "test-saga"
            }
            fn steps(&self) -> Vec<Arc<dyn SagaStep<Input = String, Output = ()>>> {
                self.steps.clone()
            }
        }

        TestSaga { steps }
    }

    struct SimpleCoordinator {
        timeout_ms: u64,
    }

    impl SimpleCoordinator {
        fn new(_max_retries: u32, timeout_ms: u64) -> Self {
            Self { timeout_ms }
        }

        async fn run<S: Saga>(&self, saga: &S, input: S::Input) -> Result<(), SagaError>
        where
            S::Input: Clone,
        {
            let steps = saga.steps();
            let mut completed: Vec<usize> = Vec::new();

            for (i, step) in steps.iter().enumerate() {
                let timeout = Duration::from_millis(self.timeout_ms);
                let deadline = tokio::time::Instant::now() + timeout;

                let result = tokio::select! {
                    biased;
                    _ = tokio::time::sleep_until(deadline) => {
                        return Err(SagaError::Timeout {
                            step: step.step_name().to_string(),
                        });
                    }
                    res = step.execute(input.clone()) => res.map(|_| ())
                };

                match result {
                    Ok(()) => {
                        completed.push(i);
                    }
                    Err(e) => {
                        for idx in completed.iter().rev() {
                            let comp = &steps[*idx];
                            if let Err(ce) = comp.compensate(input.clone()).await {
                                warn!("compensation failed for step {}: {}", comp.step_name(), ce);
                            }
                        }
                        return Err(e);
                    }
                }
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn successful_saga_executes_all_steps() {
        let saga = make_test_saga(vec![
            Arc::new(PassStep {
                name: "step1".into(),
            }),
            Arc::new(PassStep {
                name: "step2".into(),
            }),
            Arc::new(PassStep {
                name: "step3".into(),
            }),
        ]);
        let coord = SimpleCoordinator::new(0, 5000);
        let result = coord.run(&saga, "ctx".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn saga_compensates_on_step_failure() {
        let saga = make_test_saga(vec![
            Arc::new(PassStep {
                name: "step1".into(),
            }),
            Arc::new(FailStep {
                name: "step2".into(),
            }),
            Arc::new(PassStep {
                name: "step3".into(),
            }),
        ]);
        let coord = SimpleCoordinator::new(0, 5000);
        let result = coord.run(&saga, "ctx".to_string()).await;
        assert!(result.is_err());
        match result {
            Err(SagaError::StepFailed { step, .. }) => {
                assert_eq!(step, "step2");
            }
            _ => panic!("expected StepFailed for step2"),
        }
    }

    #[tokio::test]
    async fn saga_timeout_returns_timeout_error() {
        struct SlowStep;

        #[async_trait]
        impl SagaStep for SlowStep {
            type Input = String;
            type Output = ();
            async fn execute(&self, _input: String) -> Result<(), SagaError> {
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok(())
            }
            async fn compensate(&self, _input: String) -> Result<(), SagaError> {
                Ok(())
            }
            fn step_name(&self) -> &str {
                "slow"
            }
        }

        let saga = make_test_saga(vec![Arc::new(SlowStep)]);
        let coord = SimpleCoordinator::new(0, 50);
        let result = coord.run(&saga, "ctx".to_string()).await;
        assert!(result.is_err());
        match result {
            Err(SagaError::Timeout { step }) => {
                assert_eq!(step, "slow");
            }
            _ => panic!("expected Timeout error"),
        }
    }

    #[tokio::test]
    async fn compensation_failure_does_not_panic() {
        let saga = make_test_saga(vec![
            Arc::new(BadCompensateStep {
                name: "bad-comp".into(),
            }),
            Arc::new(FailStep {
                name: "trigger".into(),
            }),
        ]);
        let coord = SimpleCoordinator::new(0, 5000);
        let result = coord.run(&saga, "ctx".to_string()).await;
        assert!(result.is_err());
        match result {
            Err(SagaError::StepFailed { step, .. }) => {
                assert_eq!(step, "trigger");
            }
            _ => panic!("expected StepFailed"),
        }
    }

    // Full SagaCoordinator tests with InMemoryBroker
    #[cfg(feature = "messaging-bridge")]
    mod messaging_tests {
        use super::*;
        use nova_messaging::{InMemoryBroker, MessageBroker};

        #[tokio::test]
        async fn coordinator_publishes_events_for_each_step() {
            let broker = Arc::new(InMemoryBroker::default());
            let coord = SagaCoordinator::new(broker.clone(), 0, 5000);

            let saga = make_test_saga(vec![
                Arc::new(PassStep { name: "a".into() }),
                Arc::new(PassStep { name: "b".into() }),
            ]);

            coord
                .run(&saga, "ctx".to_string())
                .await
                .expect("saga should succeed");

            let events = broker.poll("saga.test-saga.a.started", 10).await.unwrap();
            assert_eq!(events.len(), 1);

            let events = broker.poll("saga.test-saga.a.succeeded", 10).await.unwrap();
            assert_eq!(events.len(), 1);

            let events = broker.poll("saga.test-saga.b.succeeded", 10).await.unwrap();
            assert_eq!(events.len(), 1);

            let events = broker
                .poll("saga.test-saga.saga.completed", 10)
                .await
                .unwrap();
            assert_eq!(events.len(), 1);
        }

        #[tokio::test]
        async fn coordinator_compensates_and_publishes_compensation_events() {
            let broker = Arc::new(InMemoryBroker::default());
            let coord = SagaCoordinator::new(broker.clone(), 0, 5000);

            let saga = make_test_saga(vec![
                Arc::new(PassStep {
                    name: "reserve".into(),
                }),
                Arc::new(FailStep {
                    name: "charge".into(),
                }),
            ]);

            let result = coord.run(&saga, "ctx".to_string()).await;
            assert!(result.is_err());

            let compensated = broker
                .poll("saga.test-saga.reserve.compensated", 10)
                .await
                .unwrap();
            assert_eq!(compensated.len(), 1);

            let saga_comp = broker
                .poll("saga.test-saga.saga.compensated", 10)
                .await
                .unwrap();
            assert_eq!(saga_comp.len(), 1);
        }

        #[tokio::test]
        async fn coordinator_retries_and_then_fails() {
            struct FlakyStep {
                attempts: Arc<std::sync::atomic::AtomicU32>,
            }

            #[async_trait]
            impl SagaStep for FlakyStep {
                type Input = String;
                type Output = ();
                async fn execute(&self, _input: String) -> Result<(), SagaError> {
                    let prev = self
                        .attempts
                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if prev == 0 {
                        Err(SagaError::StepFailed {
                            step: "flaky".into(),
                            reason: "transient".into(),
                        })
                    } else {
                        Ok(())
                    }
                }
                async fn compensate(&self, _input: String) -> Result<(), SagaError> {
                    Ok(())
                }
                fn step_name(&self) -> &str {
                    "flaky"
                }
            }

            let attempts = Arc::new(std::sync::atomic::AtomicU32::new(0));
            let broker = Arc::new(InMemoryBroker::default());
            let coord = SagaCoordinator::new(broker.clone(), 3, 5000);

            let saga = make_test_saga(vec![Arc::new(FlakyStep {
                attempts: attempts.clone(),
            })]);

            let result = coord.run(&saga, "ctx".to_string()).await;
            assert!(result.is_ok());
            assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 2);
        }
    }
}
