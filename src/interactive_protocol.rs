use std::collections::HashMap;

pub enum Message {
    PlanDescription(String),
    StartJob,
    PlanEnvs(HashMap<String, String>),
    PlanEnvsResponse(HashMap<String, String>),
    StrategyEnv(u16, HashMap<String, String>),
}
