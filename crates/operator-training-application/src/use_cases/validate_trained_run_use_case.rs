//! Validate a trained `LoRA` adapter against a holdout. Orchestrates:
//!
//! 1. `Predictor::predict` runs the external predictor over the
//!    holdout dataset, producing `predictions.jsonl` and
//!    `summary.json` at the configured output directory.
//! 2. A `PredictionsReader` adapter parses the predictions file into
//!    `Vec<StepKeyedPrediction>`.
//! 3. The use case joins step-keyed predictions to the in-memory
//!    ground-truth `TrainingTrajectory` set by `step_id`, building
//!    `EvaluationPair` values (refusing pairs that fail the trajectory
//!    id check inside `EvaluationPair::new`).
//! 4. `PolicyEvaluator::evaluate` scores the pairs and returns the
//!    `EvaluationReport`.
//!
//! Trajectories that did not appear in the predictions file (the
//! predictor failed to produce an action for them) are skipped at
//! this stage. Counts come back from the predictor's `summary.json`
//! via `PredictorOutcome`; the caller can apply post-train readiness
//! gates against either `PredictorOutcome` (process-level health) or
//! the returned `EvaluationReport` (model-level health), or both.

use operator_evaluation_domain::prediction::step_prediction_join::join_step_predictions;

use crate::errors::training_application_result::TrainingApplicationResult;
use crate::ports::policy_evaluator::PolicyEvaluator;
use crate::ports::predictions_reader::PredictionsReader;
use crate::ports::predictor::Predictor;
use crate::use_cases::validate_trained_run_outcome::ValidateTrainedRunOutcome;
use crate::use_cases::validate_trained_run_request::ValidateTrainedRunRequest;

#[derive(Debug)]
pub struct ValidateTrainedRunUseCase<P: Predictor, R: PredictionsReader, E: PolicyEvaluator> {
    predictor: P,
    reader: R,
    evaluator: E,
}

impl<P, R, E> ValidateTrainedRunUseCase<P, R, E>
where
    P: Predictor,
    R: PredictionsReader,
    E: PolicyEvaluator,
{
    pub fn new(predictor: P, reader: R, evaluator: E) -> Self {
        Self {
            predictor,
            reader,
            evaluator,
        }
    }

    pub fn execute(
        &self,
        request: &ValidateTrainedRunRequest,
    ) -> TrainingApplicationResult<ValidateTrainedRunOutcome> {
        let predictor_outcome = self.predictor.predict(request.predictor_target())?;
        let predictions = self.reader.read()?;
        let pairs = join_step_predictions(request.ground_truth(), predictions.parsed());
        let evaluation_report = self
            .evaluator
            .evaluate(&pairs, predictions.shape_violations())?;
        Ok(ValidateTrainedRunOutcome::new(
            predictor_outcome,
            evaluation_report,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_evaluation_domain::prediction::evaluation_pair::EvaluationPair;
    use operator_evaluation_domain::prediction::predictions_read_outcome::PredictionsReadOutcome;
    use operator_evaluation_domain::prediction::shape_violation_record::ShapeViolationRecord;
    use operator_evaluation_domain::prediction::step_keyed_prediction::StepKeyedPrediction;
    use operator_evaluation_domain::report::evaluation_report::EvaluationReport;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::ids::step_id::StepId;
    use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;
    use operator_training_domain::trainer::base_model_id::BaseModelId;
    use operator_training_domain::trainer::output_directory::OutputDirectory;
    use operator_training_domain::trainer::trainer_command::TrainerCommand;
    use std::sync::Mutex;

    use crate::errors::policy_evaluator_error::PolicyEvaluatorError;
    use crate::errors::predictions_read_error::PredictionsReadError;
    use crate::errors::predictor_error::PredictorError;
    use crate::errors::training_application_error::TrainingApplicationError;
    use crate::ports::predictor_outcome::PredictorOutcome;

    #[derive(Debug)]
    struct StubPredictor {
        outcome: PredictorOutcome,
        last_target: Mutex<Option<crate::ports::predictor_target::PredictorTarget>>,
    }

    impl StubPredictor {
        fn new(outcome: PredictorOutcome) -> Self {
            Self {
                outcome,
                last_target: Mutex::new(None),
            }
        }
    }

    impl Predictor for StubPredictor {
        fn predict(
            &self,
            target: &crate::ports::predictor_target::PredictorTarget,
        ) -> Result<PredictorOutcome, PredictorError> {
            *self.last_target.lock().unwrap() = Some(target.clone());
            Ok(self.outcome.clone())
        }
    }

    #[derive(Debug)]
    struct FailingPredictor;

    impl Predictor for FailingPredictor {
        fn predict(
            &self,
            target: &crate::ports::predictor_target::PredictorTarget,
        ) -> Result<PredictorOutcome, PredictorError> {
            Err(PredictorError::SpawnFailure {
                adapter: "stub",
                command: target.command().as_str().to_string(),
                message: "not found".into(),
            })
        }
    }

    #[derive(Debug)]
    struct StubReader {
        rows: Vec<StepKeyedPrediction>,
        shape_violations: Vec<ShapeViolationRecord>,
    }

    impl PredictionsReader for StubReader {
        fn read(&self) -> Result<PredictionsReadOutcome, PredictionsReadError> {
            Ok(PredictionsReadOutcome::new(
                self.rows.clone(),
                self.shape_violations.clone(),
            ))
        }
    }

    #[derive(Debug)]
    struct FailingReader;

    impl PredictionsReader for FailingReader {
        fn read(&self) -> Result<PredictionsReadOutcome, PredictionsReadError> {
            Err(PredictionsReadError::SourceUnavailable {
                adapter: "stub",
                message: "missing".into(),
            })
        }
    }

    #[derive(Debug, Default)]
    struct RecordingEvaluator {
        last_count: Mutex<usize>,
        last_shape_count: Mutex<usize>,
    }

    impl PolicyEvaluator for RecordingEvaluator {
        fn evaluate(
            &self,
            pairs: &[EvaluationPair],
            shape_violations: &[ShapeViolationRecord],
        ) -> Result<EvaluationReport, PolicyEvaluatorError> {
            *self.last_count.lock().unwrap() = pairs.len();
            *self.last_shape_count.lock().unwrap() = shape_violations.len();
            Ok(EvaluationReport::from_outcomes_and_shape_violations(
                Vec::new(),
                shape_violations.to_vec(),
            ))
        }
    }

    fn inspect_trajectory(traj: &str, step: &str, target_ref: &str) -> TrainingTrajectory {
        let target = MemoryRef::parse(target_ref).unwrap();
        let visible = VisibleState::assemble(
            [target.clone()],
            std::iter::empty(),
            None,
            BudgetSnapshot::unbounded(),
        );
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(target),
        )));
        TrainingTrajectory::new(
            TrainingTrajectoryId::parse(traj).unwrap(),
            StepId::parse(step).unwrap(),
            AboutId::parse("about:1").unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("read.inspect").unwrap(),
            operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal::parse(
                "Inspect the visible memory node.",
            )
            .unwrap(),
            AllowedTools::for_mode(OperatorMode::Read),
            visible,
            action,
        )
        .unwrap()
    }

    fn predictor_outcome() -> PredictorOutcome {
        PredictorOutcome::new("/tmp/preds.jsonl", "/tmp/summary.json", 2, 0).unwrap()
    }

    fn predictor_target() -> crate::ports::predictor_target::PredictorTarget {
        crate::ports::predictor_target::PredictorTarget::new(
            TrainerCommand::parse("predictor").unwrap(),
            BaseModelId::parse("base").unwrap(),
            OutputDirectory::parse("adapter").unwrap(),
            "dataset.jsonl",
            OutputDirectory::parse("out").unwrap(),
        )
        .unwrap()
    }

    fn inspect_prediction(step: &str, target_ref: &str) -> StepKeyedPrediction {
        let target = MemoryRef::parse(target_ref).unwrap();
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(target),
        )));
        StepKeyedPrediction::new(StepId::parse(step).unwrap(), action)
    }

    #[test]
    fn happy_path_returns_outcome_and_passes_pairs_to_evaluator() {
        let predictor = StubPredictor::new(predictor_outcome());
        let reader = StubReader {
            rows: vec![
                inspect_prediction("step:1", "node:1"),
                inspect_prediction("step:2", "node:2"),
            ],
            shape_violations: Vec::new(),
        };
        let evaluator = RecordingEvaluator::default();
        let use_case = ValidateTrainedRunUseCase::new(predictor, reader, evaluator);

        let request = ValidateTrainedRunRequest::new(
            predictor_target(),
            vec![
                inspect_trajectory("traj:1", "step:1", "node:1"),
                inspect_trajectory("traj:2", "step:2", "node:2"),
            ],
        );
        let outcome = use_case.execute(&request).expect("happy path");

        assert_eq!(outcome.predictor_outcome().predictions(), 2);
        assert_eq!(
            *use_case.evaluator.last_count.lock().unwrap(),
            2,
            "evaluator must receive one pair per prediction"
        );
        // Verify the predictor was called with the request's target —
        // a future refactor that accidentally swaps adapter dirs or
        // dataset paths would not silently pass this test.
        let captured = use_case
            .predictor
            .last_target
            .lock()
            .unwrap()
            .clone()
            .expect("predictor must have been invoked");
        assert_eq!(&captured, request.predictor_target());
    }

    #[test]
    fn predictor_failure_bubbles_out() {
        let predictor = FailingPredictor;
        let reader = StubReader {
            rows: Vec::new(),
            shape_violations: Vec::new(),
        };
        let evaluator = RecordingEvaluator::default();
        let use_case = ValidateTrainedRunUseCase::new(predictor, reader, evaluator);

        let request = ValidateTrainedRunRequest::new(predictor_target(), Vec::new());
        let err = use_case.execute(&request).unwrap_err();
        assert!(matches!(
            err,
            TrainingApplicationError::Predictor(PredictorError::SpawnFailure { .. })
        ));
    }

    #[test]
    fn reader_failure_bubbles_out() {
        let predictor = StubPredictor::new(predictor_outcome());
        let use_case =
            ValidateTrainedRunUseCase::new(predictor, FailingReader, RecordingEvaluator::default());

        let request = ValidateTrainedRunRequest::new(predictor_target(), Vec::new());
        let err = use_case.execute(&request).unwrap_err();
        assert!(matches!(
            err,
            TrainingApplicationError::PredictionsRead(
                PredictionsReadError::SourceUnavailable { .. }
            )
        ));
    }

    #[test]
    fn predictions_with_no_matching_ground_truth_are_skipped() {
        let predictor = StubPredictor::new(predictor_outcome());
        let reader = StubReader {
            rows: vec![
                inspect_prediction("step:1", "node:1"),
                inspect_prediction("step:99-not-in-ground-truth", "node:99"),
            ],
            shape_violations: Vec::new(),
        };
        let evaluator = RecordingEvaluator::default();
        let use_case = ValidateTrainedRunUseCase::new(predictor, reader, evaluator);

        let request = ValidateTrainedRunRequest::new(
            predictor_target(),
            vec![inspect_trajectory("traj:1", "step:1", "node:1")],
        );
        use_case.execute(&request).expect("happy path");
        assert_eq!(
            *use_case.evaluator.last_count.lock().unwrap(),
            1,
            "orphan prediction is dropped before evaluation"
        );
    }

    #[test]
    fn evaluator_failure_is_wrapped_in_application_error() {
        #[derive(Debug)]
        struct FailingEval;
        impl PolicyEvaluator for FailingEval {
            fn evaluate(
                &self,
                _: &[EvaluationPair],
                _: &[ShapeViolationRecord],
            ) -> Result<EvaluationReport, PolicyEvaluatorError> {
                Err(PolicyEvaluatorError::DomainFailure {
                    adapter: "stub",
                    message: "boom".into(),
                })
            }
        }
        let predictor = StubPredictor::new(predictor_outcome());
        let reader = StubReader {
            rows: vec![inspect_prediction("step:1", "node:1")],
            shape_violations: Vec::new(),
        };
        let use_case = ValidateTrainedRunUseCase::new(predictor, reader, FailingEval);

        let request = ValidateTrainedRunRequest::new(
            predictor_target(),
            vec![inspect_trajectory("traj:1", "step:1", "node:1")],
        );
        let err = use_case.execute(&request).unwrap_err();
        assert!(matches!(
            err,
            TrainingApplicationError::PolicyEvaluator(PolicyEvaluatorError::DomainFailure { .. })
        ));
    }

    #[test]
    fn empty_predictions_yields_empty_evaluation() {
        let predictor =
            StubPredictor::new(PredictorOutcome::new("/tmp/p", "/tmp/s", 0, 0).unwrap());
        let reader = StubReader {
            rows: Vec::new(),
            shape_violations: Vec::new(),
        };
        let evaluator = RecordingEvaluator::default();
        let use_case = ValidateTrainedRunUseCase::new(predictor, reader, evaluator);

        let request = ValidateTrainedRunRequest::new(
            predictor_target(),
            vec![inspect_trajectory("traj:1", "step:1", "node:1")],
        );
        let outcome = use_case.execute(&request).expect("happy path");
        assert_eq!(outcome.evaluation_report().total(), 0);
        assert_eq!(*use_case.evaluator.last_count.lock().unwrap(), 0);
    }

    #[test]
    fn shape_violations_are_passed_to_evaluator() {
        let predictor = StubPredictor::new(predictor_outcome());
        let violation =
            ShapeViolationRecord::new(3, Some(StepId::parse("step:bad").unwrap()), "bad action")
                .unwrap();
        let reader = StubReader {
            rows: vec![inspect_prediction("step:1", "node:1")],
            shape_violations: vec![violation],
        };
        let evaluator = RecordingEvaluator::default();
        let use_case = ValidateTrainedRunUseCase::new(predictor, reader, evaluator);

        let request = ValidateTrainedRunRequest::new(
            predictor_target(),
            vec![inspect_trajectory("traj:1", "step:1", "node:1")],
        );
        let outcome = use_case.execute(&request).expect("happy path");

        assert_eq!(outcome.evaluation_report().shape_invalid_count(), 1);
        assert_eq!(*use_case.evaluator.last_count.lock().unwrap(), 1);
        assert_eq!(*use_case.evaluator.last_shape_count.lock().unwrap(), 1);
    }

    #[test]
    #[should_panic(expected = "duplicate step_id")]
    fn duplicate_ground_truth_step_id_panics_in_debug_builds() {
        // Two trajectories share `step:1` — this is a caller-side
        // invariant violation that the use case refuses to handle
        // silently. In release builds `debug_assert!` is a no-op and
        // the second trajectory wins (preserving the prior
        // behaviour); the panic here is the debug-time canary.
        let predictor = StubPredictor::new(predictor_outcome());
        let reader = StubReader {
            rows: vec![inspect_prediction("step:1", "node:1")],
            shape_violations: Vec::new(),
        };
        let evaluator = RecordingEvaluator::default();
        let use_case = ValidateTrainedRunUseCase::new(predictor, reader, evaluator);

        let request = ValidateTrainedRunRequest::new(
            predictor_target(),
            vec![
                inspect_trajectory("traj:1", "step:1", "node:1"),
                inspect_trajectory("traj:2", "step:1", "node:2"),
            ],
        );
        let _ = use_case.execute(&request);
    }
}
