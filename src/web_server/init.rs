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
    WaitingForStrategyApproval(u16),
    RunningStrategy(u16),
}

struct AppData {
    state: AppState,
    sender: Sender<String>,
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
            let _ = app_data
                .sender
                .send(interactive_protocol::START_JOB.to_string())
                .await;
            "started"
        }
        AppState::WaitingForStrategyApproval(_) => "already started",
        AppState::RunningStrategy(_) => "already started",
    };

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(message)
}

pub fn create(
    port: u16,
    sender: Sender<String>,
    mut receiver: Receiver<String>,
) -> anyhow::Result<Server> {
    let (internal_tx, mut internal_rx) = tokio::sync::mpsc::channel::<String>(1024);

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
                    let strategy = receiver.recv().await;

                    if let Some(strategies) = strategy {
                        let mut data = job_data_comm.lock().unwrap();
                        data.strategies = strategies;

                        app_data_comm.lock().unwrap().state = AppState::WaitingForPlanApproval;
                    }
                }
                AppState::WaitingForPlanApproval => {
                    let should_start = internal_rx.recv().await;

                    if let Some(should_start) = should_start {
                        if should_start == interactive_protocol::START_JOB {
                            let _ = sender.send(interactive_protocol::START_JOB.to_string()).await;

                            app_data_comm.lock().unwrap().state =
                                AppState::WaitingForStrategyApproval(0);
                        }
                    }
                }
                AppState::WaitingForStrategyApproval(_index) => {}
                AppState::RunningStrategy(_) => todo!(),
            }
        }
    });

    return Ok(web_server);
}
