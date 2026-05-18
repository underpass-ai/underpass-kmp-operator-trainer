//! Port: invoke the external trainer described by a `TrainerTarget`.
//! Returns a typed `TrainerInvocationOutcome`; the use case never sees
//! stdout, stderr, or the process handle. Adapters that capture those
//! for diagnostics do so behind their own private types.

use operator_training_domain::trainer::trainer_target::TrainerTarget;

use crate::errors::trainer_invoker_error::TrainerInvokerError;
use crate::ports::trainer_invocation_outcome::TrainerInvocationOutcome;

pub trait TrainerInvoker: std::fmt::Debug + Send + Sync {
    fn invoke(
        &self,
        target: &TrainerTarget,
    ) -> Result<TrainerInvocationOutcome, TrainerInvokerError>;
}
