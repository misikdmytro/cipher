use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceId(Uuid);

impl TraceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Display for TraceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Uuid> for TraceId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl From<TraceId> for Uuid {
    fn from(value: TraceId) -> Self {
        value.0
    }
}

impl FromStr for TraceId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}
