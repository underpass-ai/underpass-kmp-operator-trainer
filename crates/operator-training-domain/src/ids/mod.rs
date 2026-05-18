//! Identifier value objects used by the training context. Some are
//! re-exports from `operator-shared-domain` (e.g., `TrainingRunId`),
//! because they are vocabulary the rest of the operator already knows
//! about. The re-export keeps consumer imports inside the training
//! crate boundary.

pub use operator_shared_domain::ids::training_run_id::TrainingRunId;
