//! Port: talk to a KMP server over MCP. One method per `KernelTool`
//! variant; each method takes the typed argument value object from
//! `operator-shared-domain::tool_arguments` and returns the typed
//! outcome value object from `operator-shared-domain::tool_outcomes`,
//! or a `KmpClientError` on adapter / wire failure.
//!
//! Adapters that implement this trait live in
//! `operator-replay-infra`. The application use cases (in a follow-up
//! PR) depend on the trait, never on a concrete adapter.

use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
use operator_shared_domain::tool_arguments::ingest_arguments::IngestArguments;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::near_arguments::NearArguments;
use operator_shared_domain::tool_arguments::rewind_arguments::RewindArguments;
use operator_shared_domain::tool_arguments::trace_arguments::TraceArguments;
use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
use operator_shared_domain::tool_outcomes::ask_outcome::AskOutcome;
use operator_shared_domain::tool_outcomes::forward_outcome::ForwardOutcome;
use operator_shared_domain::tool_outcomes::goto_outcome::GotoOutcome;
use operator_shared_domain::tool_outcomes::ingest_outcome::IngestOutcome;
use operator_shared_domain::tool_outcomes::inspect_outcome::InspectOutcome;
use operator_shared_domain::tool_outcomes::near_outcome::NearOutcome;
use operator_shared_domain::tool_outcomes::rewind_outcome::RewindOutcome;
use operator_shared_domain::tool_outcomes::trace_outcome::TraceOutcome;
use operator_shared_domain::tool_outcomes::wake_outcome::WakeOutcome;
use operator_shared_domain::tool_outcomes::write_memory_outcome::WriteMemoryOutcome;

use crate::error::kmp_client_error::KmpClientError;

pub trait KmpMcpClient: std::fmt::Debug + Send + Sync {
    fn ingest(&self, args: &IngestArguments) -> Result<IngestOutcome, KmpClientError>;
    fn wake(&self, args: &WakeArguments) -> Result<WakeOutcome, KmpClientError>;
    fn ask(&self, args: &AskArguments) -> Result<AskOutcome, KmpClientError>;
    fn near(&self, args: &NearArguments) -> Result<NearOutcome, KmpClientError>;
    fn goto(&self, args: &GotoArguments) -> Result<GotoOutcome, KmpClientError>;
    fn rewind(&self, args: &RewindArguments) -> Result<RewindOutcome, KmpClientError>;
    fn forward(&self, args: &ForwardArguments) -> Result<ForwardOutcome, KmpClientError>;
    fn trace(&self, args: &TraceArguments) -> Result<TraceOutcome, KmpClientError>;
    fn inspect(&self, args: &InspectArguments) -> Result<InspectOutcome, KmpClientError>;
    fn write_memory(
        &self,
        args: &WriteMemoryArguments,
    ) -> Result<WriteMemoryOutcome, KmpClientError>;
}
