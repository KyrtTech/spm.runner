use std::collections::HashMap;

pub enum Message {
    PlanDescription(String),
    StartJob,
    StrategyEnv(HashMap<String, String>)
}