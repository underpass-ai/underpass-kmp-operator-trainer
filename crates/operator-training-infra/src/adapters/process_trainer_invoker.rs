//! `TrainerInvoker` backed by `std::process::Command`. Spawns the
//! configured `TrainerTarget` synchronously, waits for the child to
//! exit, and surfaces the exit status as a typed
//! `TrainerInvocationOutcome`. stdout / stderr are inherited from the
//! parent: this adapter does not parse trainer logs.
//!
//! Command line:
//!
//! ```text
//! <command> --base-model <base_model> --output-dir <output_directory>
//! ```
//!
//! Extra arguments / environment variables are deferred to a follow-up
//! PR introducing a richer `TrainerTarget`.

use std::process::Command;

use operator_training_application::errors::trainer_invoker_error::TrainerInvokerError;
use operator_training_application::ports::trainer_invocation_outcome::TrainerInvocationOutcome;
use operator_training_application::ports::trainer_invoker::TrainerInvoker;
use operator_training_domain::trainer::trainer_target::TrainerTarget;

const ADAPTER: &str = "process_trainer_invoker";

#[derive(Debug, Default, Clone)]
pub struct ProcessTrainerInvoker;

impl ProcessTrainerInvoker {
    pub fn new() -> Self {
        Self
    }
}

impl TrainerInvoker for ProcessTrainerInvoker {
    fn invoke(
        &self,
        target: &TrainerTarget,
    ) -> Result<TrainerInvocationOutcome, TrainerInvokerError> {
        let mut command = Command::new(target.command().as_str());
        command
            .arg("--base-model")
            .arg(target.base_model().as_str())
            .arg("--output-dir")
            .arg(target.output_directory().as_str());
        let status = command
            .status()
            .map_err(|err| TrainerInvokerError::SpawnFailure {
                adapter: ADAPTER,
                command: target.command().as_str().to_string(),
                message: err.to_string(),
            })?;
        Ok(match status.code() {
            Some(0) => TrainerInvocationOutcome::Success { exit_code: 0 },
            Some(code) => TrainerInvocationOutcome::Failed {
                exit_code: Some(code),
            },
            None => TrainerInvocationOutcome::Failed { exit_code: None },
        })
    }
}
