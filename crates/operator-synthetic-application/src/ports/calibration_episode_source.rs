//! Port: load gold-standard teacher calibration cases.

use operator_synthetic_domain::calibration::calibration_case::CalibrationCase;

use crate::error::calibration_episode_source_error::CalibrationEpisodeSourceError;

pub trait CalibrationEpisodeSource: std::fmt::Debug + Send + Sync {
    fn read(&self) -> Result<Vec<CalibrationCase>, CalibrationEpisodeSourceError>;
}

impl<T> CalibrationEpisodeSource for Box<T>
where
    T: CalibrationEpisodeSource + ?Sized,
{
    fn read(&self) -> Result<Vec<CalibrationCase>, CalibrationEpisodeSourceError> {
        (**self).read()
    }
}
