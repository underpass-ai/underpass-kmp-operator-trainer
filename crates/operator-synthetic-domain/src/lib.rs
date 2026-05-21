//! Operator: synthetic bounded context — domain.
//!
//! Owns the canonical vocabulary for synthetic trajectory generation:
//! `KmpMcpCapability`, episode specs, corpus quality specs,
//! `SyntheticCaseSpec`, `SyntheticDatasetBlueprint`, `SyntheticDataset`,
//! `SyntheticCaseGenerationMetric` and `SyntheticDatasetGenerationReport`.
//! Depends only on
//! `operator-shared-domain` and `thiserror`.

pub mod capability;
pub mod case;
pub mod dataset;
pub mod episode;
pub mod error;
pub mod quality;
pub mod specifications;
