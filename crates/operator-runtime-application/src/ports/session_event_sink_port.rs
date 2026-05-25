use operator_runtime_domain::session::observation::Observation;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::session_outcome::SessionOutcome;
use operator_shared_domain::action::operator_action::OperatorAction;

pub trait SessionEventSink: std::fmt::Debug + Send + Sync {
    fn on_request_received(&self, request: &OperatorRequest);
    fn on_action_predicted(&self, action: &OperatorAction);
    fn on_observation(&self, observation: &Observation);
    fn on_session_complete(&self, outcome: &SessionOutcome);
}
