//! End-to-end description of a planned training run. Holds the run
//! identifier, the dataset provenance, the trainer target, and the
//! readiness report computed against this dataset.
//!
//! The manifest is representable in any readiness state — the
//! `TrainingRun` aggregate root is the one that refuses to build when
//! readiness fails. This split lets workflows surface "would-not-
//! launch" manifests (e.g., for diagnostics, dry-runs, archival).

use crate::ids::TrainingRunId;
use crate::provenance::dataset_provenance::DatasetProvenance;
use crate::readiness::readiness_report::ReadinessReport;
use crate::trainer::trainer_target::TrainerTarget;

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingManifest {
    run_id: TrainingRunId,
    dataset: DatasetProvenance,
    trainer_target: TrainerTarget,
    readiness: ReadinessReport,
}

impl TrainingManifest {
    pub fn new(
        run_id: TrainingRunId,
        dataset: DatasetProvenance,
        trainer_target: TrainerTarget,
        readiness: ReadinessReport,
    ) -> Self {
        Self {
            run_id,
            dataset,
            trainer_target,
            readiness,
        }
    }

    pub fn run_id(&self) -> &TrainingRunId {
        &self.run_id
    }

    pub fn dataset(&self) -> &DatasetProvenance {
        &self.dataset
    }

    pub fn trainer_target(&self) -> &TrainerTarget {
        &self.trainer_target
    }

    pub fn readiness(&self) -> &ReadinessReport {
        &self.readiness
    }
}
