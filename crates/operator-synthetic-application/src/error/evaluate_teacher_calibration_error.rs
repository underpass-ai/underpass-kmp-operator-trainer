//! Error returned by teacher calibration evaluation.

use thiserror::Error;

use crate::error::calibration_episode_source_error::CalibrationEpisodeSourceError;
use crate::error::teacher_policy_error::TeacherPolicyError;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum EvaluateTeacherCalibrationError {
    #[error(transparent)]
    Source(#[from] CalibrationEpisodeSourceError),

    #[error(transparent)]
    Teacher(#[from] TeacherPolicyError),
}
