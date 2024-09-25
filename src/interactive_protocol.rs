use std::{collections::HashMap, sync::Arc};

pub enum Message {
    PlanDescription(String),
    StartJob,
    StrategyEnv(u16, Arc<HashMap<String, String>>),
}
