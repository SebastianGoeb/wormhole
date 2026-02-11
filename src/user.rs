// TODO perf: i32 user-ids
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(pub String);

pub const DEFAULT_USER: &str = "god";
