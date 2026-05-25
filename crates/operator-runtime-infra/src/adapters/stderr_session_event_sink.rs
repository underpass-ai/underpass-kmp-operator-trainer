use operator_runtime_application::ports::session_event_sink_port::SessionEventSink;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::session_outcome::SessionOutcome;
use operator_shared_domain::action::operator_action::OperatorAction;

#[derive(Debug, Default)]
pub struct StderrSessionEventSink;

impl StderrSessionEventSink {
    pub fn new() -> Self {
        Self
    }
}

impl SessionEventSink for StderrSessionEventSink {
    fn on_request_received(&self, request: &OperatorRequest) {
        eprintln!(
            "event=operator_runtime.request_received session_id={} mode={}",
            request.session_id(),
            request.mode()
        );
    }

    fn on_action_predicted(&self, action: &OperatorAction) {
        let label = action
            .tool()
            .map_or_else(|| action.kind().as_str(), |tool| tool.as_str());
        eprintln!("event=operator_runtime.action_predicted action={label}");
    }

    fn on_observation(&self, observation: &Observation) {
        eprintln!(
            "event=operator_runtime.observation is_tool_error={}",
            observation.is_tool_error()
        );
    }

    fn on_session_complete(&self, outcome: &SessionOutcome) {
        eprintln!(
            "event=operator_runtime.session_complete session_id={} outcome={} elapsed_ms={}",
            outcome.session_id(),
            outcome.outcome_class().as_str(),
            outcome.elapsed_ms()
        );
    }
}
