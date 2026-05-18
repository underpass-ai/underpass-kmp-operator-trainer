use operator_shared_contract::per_tool::ask_arguments_dto::AskArgumentsDto;
use operator_shared_contract::per_tool::forward_arguments_dto::ForwardArgumentsDto;
use operator_shared_contract::per_tool::goto_arguments_dto::GotoArgumentsDto;
use operator_shared_contract::per_tool::inspect_arguments_dto::InspectArgumentsDto;
use operator_shared_contract::per_tool::near_arguments_dto::NearArgumentsDto;
use operator_shared_contract::per_tool::rewind_arguments_dto::RewindArgumentsDto;
use operator_shared_contract::per_tool::trace_arguments_dto::TraceArgumentsDto;
use operator_shared_contract::per_tool::wake_arguments_dto::WakeArgumentsDto;
use operator_shared_contract::per_tool::write_memory_arguments_dto::WriteMemoryArgumentsDto;
use operator_shared_contract::tool_arguments_dto::ToolArgumentsDto;
use operator_shared_domain::cursor::temporal_anchor::TemporalAnchor;
use operator_shared_domain::cursor::temporal_cursor::TemporalCursor;
use operator_shared_domain::cursor::temporal_cursor_key::TemporalCursorKey;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::near_arguments::NearArguments;
use operator_shared_domain::tool_arguments::rewind_arguments::RewindArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::trace_arguments::TraceArguments;
use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use serde::de::DeserializeOwned;

use crate::mappers::cursor_mapper::CursorMapper;
use crate::mappers::mapping_error::MappingError;

#[derive(Debug)]
pub struct ToolArgumentsMapper;

impl ToolArgumentsMapper {
    pub fn to_domain(dto: &ToolArgumentsDto) -> Result<ToolArguments, MappingError> {
        let tool = KernelTool::parse(&dto.tool)?;
        match tool {
            KernelTool::Wake => {
                let wake: WakeArgumentsDto = decode(&dto.arguments, "kernel_wake")?;
                Ok(ToolArguments::Wake(WakeArguments::new(AboutId::parse(
                    wake.about,
                )?)))
            }
            KernelTool::Ask => {
                let ask: AskArgumentsDto = decode(&dto.arguments, "kernel_ask")?;
                Ok(ToolArguments::Ask(AskArguments::new(ask.query)?))
            }
            KernelTool::Near => {
                let near: NearArgumentsDto = decode(&dto.arguments, "kernel_near")?;
                let anchor = MemoryRef::parse(near.anchor)?;
                let mut dimensions = Vec::with_capacity(near.dimensions.len());
                for raw in near.dimensions {
                    dimensions.push(DimensionRef::parse(raw)?);
                }
                let limit = match near.limit {
                    Some(value) => Some(PositiveCount::parse(value, "kernel_near.limit")?),
                    None => None,
                };
                Ok(ToolArguments::Near(NearArguments::new(
                    anchor, dimensions, limit,
                )?))
            }
            KernelTool::Goto => {
                let goto: GotoArgumentsDto = decode(&dto.arguments, "kernel_goto")?;
                let cursor = CursorMapper::to_domain(&goto.cursor)?;
                Ok(ToolArguments::Goto(GotoArguments::new(cursor)))
            }
            KernelTool::Rewind => {
                let rewind: RewindArgumentsDto = decode(&dto.arguments, "kernel_rewind")?;
                let cursor = TemporalCursor::new(
                    TemporalCursorKey::parse(&rewind.cursor_key)?,
                    TemporalAnchor::parse(rewind.cursor_anchor)?,
                );
                Ok(ToolArguments::Rewind(RewindArguments::new(
                    cursor,
                    PositiveCount::parse(rewind.window, "kernel_rewind.window")?,
                )))
            }
            KernelTool::Forward => {
                let forward: ForwardArgumentsDto = decode(&dto.arguments, "kernel_forward")?;
                let cursor = TemporalCursor::new(
                    TemporalCursorKey::parse(&forward.cursor_key)?,
                    TemporalAnchor::parse(forward.cursor_anchor)?,
                );
                Ok(ToolArguments::Forward(ForwardArguments::new(
                    cursor,
                    PositiveCount::parse(forward.window, "kernel_forward.window")?,
                )))
            }
            KernelTool::Trace => {
                let trace: TraceArgumentsDto = decode(&dto.arguments, "kernel_trace")?;
                let from = MemoryRef::parse(trace.from)?;
                let to = match trace.to {
                    Some(value) => Some(MemoryRef::parse(value)?),
                    None => None,
                };
                Ok(ToolArguments::Trace(TraceArguments::new(
                    from,
                    to,
                    PositiveCount::parse(trace.page, "kernel_trace.page")?,
                )))
            }
            KernelTool::Inspect => {
                let inspect: InspectArgumentsDto = decode(&dto.arguments, "kernel_inspect")?;
                Ok(ToolArguments::Inspect(InspectArguments::new(
                    MemoryRef::parse(inspect.target)?,
                )))
            }
            KernelTool::WriteMemory => {
                let write: WriteMemoryArgumentsDto = decode(&dto.arguments, "kernel_write_memory")?;
                let mut related = Vec::with_capacity(write.related.len());
                for raw in write.related {
                    related.push(MemoryRef::parse(raw)?);
                }
                Ok(ToolArguments::WriteMemory(WriteMemoryArguments::new(
                    write.summary,
                    write.body,
                    related,
                )?))
            }
        }
    }

    pub fn to_dto(domain: &ToolArguments) -> Result<ToolArgumentsDto, MappingError> {
        let (tool, arguments) = match domain {
            ToolArguments::Wake(w) => (
                KernelTool::Wake,
                serde_json::to_value(WakeArgumentsDto {
                    about: w.about().as_str().to_string(),
                })
                .map_err(|e| serde_err("kernel_wake", &e))?,
            ),
            ToolArguments::Ask(a) => (
                KernelTool::Ask,
                serde_json::to_value(AskArgumentsDto {
                    query: a.query().as_str().to_string(),
                })
                .map_err(|e| serde_err("kernel_ask", &e))?,
            ),
            ToolArguments::Near(n) => (
                KernelTool::Near,
                serde_json::to_value(NearArgumentsDto {
                    anchor: n.anchor().as_str().to_string(),
                    dimensions: n
                        .dimensions()
                        .iter()
                        .map(|d| d.as_str().to_string())
                        .collect(),
                    limit: n.limit().map(operator_shared_domain::value_objects::positive_count::PositiveCount::as_usize),
                })
                .map_err(|e| serde_err("kernel_near", &e))?,
            ),
            ToolArguments::Goto(g) => (
                KernelTool::Goto,
                serde_json::to_value(GotoArgumentsDto {
                    cursor: CursorMapper::to_dto(g.cursor()),
                })
                .map_err(|e| serde_err("kernel_goto", &e))?,
            ),
            ToolArguments::Rewind(r) => (
                KernelTool::Rewind,
                serde_json::to_value(RewindArgumentsDto {
                    cursor_key: r.cursor().key().as_str().to_string(),
                    cursor_anchor: r.cursor().anchor().as_str().to_string(),
                    window: r.window().as_usize(),
                })
                .map_err(|e| serde_err("kernel_rewind", &e))?,
            ),
            ToolArguments::Forward(f) => (
                KernelTool::Forward,
                serde_json::to_value(ForwardArgumentsDto {
                    cursor_key: f.cursor().key().as_str().to_string(),
                    cursor_anchor: f.cursor().anchor().as_str().to_string(),
                    window: f.window().as_usize(),
                })
                .map_err(|e| serde_err("kernel_forward", &e))?,
            ),
            ToolArguments::Trace(t) => (
                KernelTool::Trace,
                serde_json::to_value(TraceArgumentsDto {
                    from: t.from().as_str().to_string(),
                    to: t.to().map(|r| r.as_str().to_string()),
                    page: t.page().as_usize(),
                })
                .map_err(|e| serde_err("kernel_trace", &e))?,
            ),
            ToolArguments::Inspect(i) => (
                KernelTool::Inspect,
                serde_json::to_value(InspectArgumentsDto {
                    target: i.target().as_str().to_string(),
                })
                .map_err(|e| serde_err("kernel_inspect", &e))?,
            ),
            ToolArguments::WriteMemory(w) => (
                KernelTool::WriteMemory,
                serde_json::to_value(WriteMemoryArgumentsDto {
                    summary: w.summary().as_str().to_string(),
                    body: w.body().as_str().to_string(),
                    related: w.related().iter().map(|r| r.as_str().to_string()).collect(),
                })
                .map_err(|e| serde_err("kernel_write_memory", &e))?,
            ),
        };
        Ok(ToolArgumentsDto {
            tool: tool.as_str().to_string(),
            arguments,
        })
    }
}

fn decode<T: DeserializeOwned>(
    value: &serde_json::Value,
    tool: &'static str,
) -> Result<T, MappingError> {
    serde_json::from_value(value.clone()).map_err(|e| MappingError::InvalidToolArguments {
        tool,
        message: e.to_string(),
    })
}

fn serde_err(tool: &'static str, source: &serde_json::Error) -> MappingError {
    MappingError::InvalidToolArguments {
        tool,
        message: source.to_string(),
    }
}
