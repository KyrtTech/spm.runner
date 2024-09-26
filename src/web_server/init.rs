use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use actix_web::{
    dev::Server, get, http::header::ContentType, patch, web, App, HttpResponse, HttpServer,
    Responder,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::interactive_protocol;

#[derive(Clone, Debug)]
enum AppState {
    GettingPlan,
    WaitingForPlanApproval,
    GettingPlanEnv,
    WaitingForPlanEnv,
    GettingStrategyEnv(u16),
    WaitingForStrategyApproval(u16),
    RunningStrategy(u16),
}

#[derive(Eq, PartialEq)]
enum InternalMessage {
    JobApproved,
    PlanEnvsApproved,
}
struct AppData {
    state: AppState,
    sender: Sender<InternalMessage>,
    strategies: String,
    initial_envs: HashMap<String, String>,
    updated_envs: HashMap<String, String>,
}

#[get("/job")]
async fn job(job_data: web::Data<Arc<Mutex<AppData>>>) -> impl Responder {
    let job_data = job_data.lock().unwrap();

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(job_data.strategies.clone())
}

#[patch("/start-job")]
async fn start_job(app_data: web::Data<Arc<Mutex<AppData>>>) -> impl Responder {
    let app_data = app_data.lock().unwrap();

    let message = match app_data.state {
        AppState::GettingPlan => "plan not found",
        AppState::WaitingForPlanApproval => {
            let _ = app_data.sender.send(InternalMessage::JobApproved).await;
            "started"
        }
        _ => return HttpResponse::Conflict().body("already started"),
    };

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(message)
}

#[get("/envs")]
async fn plan_envs(app_data: web::Data<Arc<Mutex<AppData>>>) -> impl Responder {
    let serialized = match serde_json::to_string(&app_data.lock().unwrap().initial_envs) {
        Ok(val) => val,
        Err(_) => "{}".to_string(),
    };

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serialized)
}

#[patch("/envs")]
async fn update_plan_envs(
    app_data: web::Data<Arc<Mutex<AppData>>>,
    body: web::Json<HashMap<String, String>>,
) -> impl Responder {
    let mut app_data = app_data.lock().unwrap();

    match app_data.state {
        AppState::WaitingForPlanEnv => {
            let mut envs = app_data.initial_envs.clone();
            for (key, _) in envs.clone().into_iter() {
                if let Some(new_val) = body.get(&key) {
                    envs.insert(key, new_val.clone());
                }
            }

            app_data.updated_envs = envs;

            let _ = app_data
                .sender
                .send(InternalMessage::PlanEnvsApproved)
                .await;
        }
        _ => return HttpResponse::Conflict().finish(),
    }

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body("envs updated")
}

pub fn create(
    port: u16,
    sender: Sender<interactive_protocol::Message>,
    mut receiver: Receiver<interactive_protocol::Message>,
) -> anyhow::Result<Server> {
    let (internal_tx, mut internal_rx) = tokio::sync::mpsc::channel::<InternalMessage>(32);

    let app_data = Arc::new(Mutex::new(AppData {
        sender: internal_tx,
        state: AppState::GettingPlan,
        strategies: "[]".to_string(),
        initial_envs: HashMap::new(),
        updated_envs: HashMap::new(),
    }));
    let app_data_comm = app_data.clone();

    let web_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_data.clone()))
            .service(job)
            .service(start_job)
            .service(plan_envs)
            .service(update_plan_envs)
    })
    .bind(("127.0.0.1", port))?
    .run();

    tokio::spawn(async move {
        loop {
            let app_state = app_data_comm.lock().unwrap().state.clone();

            match app_state {
                AppState::GettingPlan => {
                    let maybe_message = receiver.recv().await;

                    if let Some(message) = maybe_message {
                        match message {
                            interactive_protocol::Message::PlanDescription(strategies) => {
                                let mut data = app_data_comm.lock().unwrap();

                                data.strategies = strategies;
                                data.state = AppState::WaitingForPlanApproval;
                            }
                            _ => (),
                        }
                    }
                }
                AppState::WaitingForPlanApproval => {
                    let maybe_message = internal_rx.recv().await;

                    if let Some(should_start) = maybe_message {
                        if should_start == InternalMessage::JobApproved {
                            let _ = sender.send(interactive_protocol::Message::StartJob).await;

                            app_data_comm.lock().unwrap().state = AppState::GettingPlanEnv;
                        }
                    }
                }
                AppState::GettingPlanEnv => {
                    let maybe_message = receiver.recv().await;

                    if let Some(message) = maybe_message {
                        match message {
                            interactive_protocol::Message::PlanEnvs(map) => {
                                let mut app_data = app_data_comm.lock().unwrap();

                                app_data.initial_envs = map;
                                app_data.state = AppState::WaitingForPlanEnv
                            }
                            _ => (),
                        }
                    }
                }
                AppState::WaitingForPlanEnv => {
                    let maybe_message = internal_rx.recv().await;

                    if let Some(message) = maybe_message {
                        match message {
                            InternalMessage::PlanEnvsApproved => {
                                let vals = app_data_comm.lock().unwrap().updated_envs.clone();

                                let _ = sender
                                    .send(interactive_protocol::Message::PlanEnvsResponse(vals))
                                    .await;

                                app_data_comm.lock().unwrap().state =
                                    AppState::GettingStrategyEnv(0);
                            }
                            _ => (),
                        }
                    }
                }
                AppState::GettingStrategyEnv(index) => {
                    let maybe_message = receiver.recv().await;

                    if let Some(message) = maybe_message {
                        match message {
                            interactive_protocol::Message::StrategyEnv(received_index, arc) => {
                                if index != received_index {
                                    eprintln!(
                                        "Strategy indexes do not match current: {} recv: {}",
                                        index, received_index
                                    );
                                    continue;
                                }

                                app_data_comm.lock().unwrap().state =
                                    AppState::WaitingForStrategyApproval(index)
                            }
                            _ => (),
                        };
                    }
                }
                AppState::WaitingForStrategyApproval(_index) => {}
                AppState::RunningStrategy(_) => todo!(),
            }
        }
    });

    return Ok(web_server);
}
