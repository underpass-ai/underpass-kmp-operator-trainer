use serde::{Deserialize, Serialize};

use crate::cursor_dto::CursorDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GotoArgumentsDto {
    pub cursor: CursorDto,
}
