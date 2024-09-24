use std::sync::{Arc, Mutex};

use actix_web::{
    dev::Server, get, http::header::ContentType, web, App, HttpResponse, HttpServer, Responder,
};
use tokio::sync::mpsc::{Receiver, Sender};

struct JobData {
    strategies: String,
}

#[get("/job")]
async fn job(data: web::Data<Arc<Mutex<JobData>>>) -> impl Responder {
    let data = data.lock().unwrap();

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(data.strategies.clone())
}

pub fn create(
    port: u16,
    sender: Sender<String>,
    mut receiver: Receiver<String>,
) -> anyhow::Result<Server> {
    let job_data = Arc::new(Mutex::new(JobData {
        strategies: "[]".to_string(),
    }));

    let job_data_rec = job_data.clone();

    let web_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(job_data.clone()))
            .service(job)
    })
    .bind(("127.0.0.1", port))?
    .run();

    tokio::spawn(async move {
        loop {
            let strategy = receiver.recv().await;

            if let Some(strategies) = strategy {
                let mut data = job_data_rec.lock().unwrap();
                data.strategies = strategies;
            }
        }
    });

    return Ok(web_server);
}
