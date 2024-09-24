use actix_web::{dev::Server, get, web, App, HttpServer, Responder};

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}

pub fn create(port: u16) -> anyhow::Result<Server> {
    let web_server = HttpServer::new(|| App::new().service(greet))
        .bind(("127.0.0.1", port))?
        .run();

    return Ok(web_server);
}
