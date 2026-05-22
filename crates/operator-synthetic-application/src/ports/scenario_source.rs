//! Port for loading externally authored realistic corpus scenarios.

use crate::error::scenario_source_error::ScenarioSourceError;
use crate::ports::scenario::Scenario;

pub trait ScenarioSource: std::fmt::Debug + Send + Sync {
    fn read(&self) -> Result<Vec<Scenario>, ScenarioSourceError>;
}

impl<T> ScenarioSource for Box<T>
where
    T: ScenarioSource + ?Sized,
{
    fn read(&self) -> Result<Vec<Scenario>, ScenarioSourceError> {
        (**self).read()
    }
}
