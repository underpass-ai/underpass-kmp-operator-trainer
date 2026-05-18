use serde::{Deserialize, Serialize};

use crate::tool_arguments_dto::ToolArgumentsDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallActionDto {
    #[serde(flatten)]
    pub arguments: ToolArgumentsDto,
}
