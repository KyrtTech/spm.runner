use std::sync::{Arc, Mutex};

use actix_web::{
    dev::Server, get, http::header::ContentType, patch, web, App, HttpResponse, HttpServer,
    Responder,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::interactive_protocol;

struct JobData {
    strategies: String,
}

#[derive(Clone, Debug)]
enum AppState {
    GettingPlan,
    WaitingForPlanApproval,
    GettingStrategyEnv(u16),
    WaitingForStrategyApproval(u16),
    RunningStrategy(u16),
}

#[derive(Eq, PartialEq)]
enum InternalMessage {
    JobApproved,
}
struct AppData {
    state: AppState,
    sender: Sender<InternalMessage>,
}

#[get("/job")]
async fn job(job_data: web::Data<Arc<Mutex<JobData>>>) -> impl Responder {
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
        _ => "already started",
    };

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(message)
}

pub fn create(
    port: u16,
    sender: Sender<interactive_protocol::Message>,
    mut receiver: Receiver<interactive_protocol::Message>,
) -> anyhow::Result<Server> {
    let (internal_tx, mut internal_rx) = tokio::sync::mpsc::channel::<InternalMessage>(32);

    let job_data = Arc::new(Mutex::new(JobData {
        strategies: "[]".to_string(),
    }));
    let job_data_comm = job_data.clone();

    let app_data = Arc::new(Mutex::new(AppData {
        sender: internal_tx,
        state: AppState::GettingPlan,
    }));
    let app_data_comm = app_data.clone();

    let web_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_data.clone()))
            .service(
                web::scope("")
                    .app_data(web::Data::new(job_data.clone()))
                    .service(job)
                    .service(start_job),
            )
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
                                let mut data = job_data_comm.lock().unwrap();
                                data.strategies = strategies;

                                app_data_comm.lock().unwrap().state =
                                    AppState::WaitingForPlanApproval;
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

                            app_data_comm.lock().unwrap().state = AppState::GettingStrategyEnv(0);
                        }
                    }
                }
                AppState::GettingStrategyEnv(index) => {}
                AppState::WaitingForStrategyApproval(_index) => {}
                AppState::RunningStrategy(_) => todo!(),
            }
        }
    });

    return Ok(web_server);
}
