use std::{collections::HashMap, sync::Arc};

pub enum Message {
    PlanDescription(String),
    StartJob,
    PlanEnvs(Arc<HashMap<String, String>>),
    PlanEnvsResponse(Arc<HashMap<String, String>>),
    StrategyEnv(u16, Arc<HashMap<String, String>>),
}
