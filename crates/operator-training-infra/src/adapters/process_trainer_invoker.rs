//! `TrainerInvoker` backed by `std::process::Command`. Spawns the
//! configured `TrainerTarget` synchronously, waits for the child to
//! exit, and surfaces the exit status as a typed
//! `TrainerInvocationOutcome`. stdout / stderr are inherited from the
//! parent: this adapter does not parse trainer logs.
//!
//! **stdin is explicitly redirected to `/dev/null`** so the trainer
//! cannot block waiting for input the parent never sends — production
//! callers can always launch this adapter from a script context
//! without prearranging an input pipe.
//!
//! Command line:
//!
//! ```text
//! <command> --base-model <base_model> --output-dir <output_directory>
//! ```
//!
//! Extra arguments / environment variables are deferred to a follow-up
//! PR introducing a richer `TrainerTarget`.
//!
//! Note: the integration tests in `tests/process_trainer_invoker.rs`
//! serialise their write-chmod-exec triples behind a global mutex
//! (`EXEC_LOCK`) to dodge a parallel-execution `ETXTBSY` race on tmpfs
//! / CI filesystems. The race is in the test harness, not in this
//! adapter; do not remove the mutex without re-validating CI under
//! parallel test execution.

use std::process::{Command, Stdio};

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
            .stdin(Stdio::null())
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
