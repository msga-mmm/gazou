use deadpool_postgres::{
    tokio_postgres::NoTls, Config, Manager, ManagerConfig, Pool, RecyclingMethod, Runtime,
};
use ntex::{
    http::StatusCode,
    web::{self, Responder},
};
#[web::get("/")]

async fn hello() -> impl web::Responder {
    web::HttpResponse::Ok().body("Hello world!")
}

#[web::get("/images")]
async fn images(pool: web::types::State<Pool>, req: web::HttpRequest) -> web::HttpResponse {
    let client = pool.get().await.unwrap();

    let images_etag_statement = client
        .prepare_cached("SELECT md5(string_agg(id || name, '')) as etag FROM images")
        .await
        .unwrap();

    let headers = req.headers();
    let request_etag = headers.get("ETag");

    let rows = client.query(&images_etag_statement, &[]).await.unwrap();
    let response_etag = rows.get(0);

	// TODO: move logic into middleware
    if let Some(request_etag) = request_etag {
        if let Some(response_etag) = response_etag {
            let request_etag: String = request_etag.to_str().unwrap_or("").to_string();
            let response_etag: String = response_etag.get(0);

            if request_etag == response_etag {
                return web::HttpResponse::NotModified().into();
            }
        }
    }

    let images_statement = client
        .prepare_cached("SELECT name FROM images")
        .await
        .unwrap();

    let rows = client.query(&images_statement, &[]).await.unwrap();

    let mut response = String::new();

    for row in rows {
        response += row.get(0);
        response += "\n";
    }

    // web::HttpResponse::NotModified().body(response)

    if let Some(response_etag) = response_etag {
        let etag: String = response_etag.get(0);
        return web::HttpResponse::Ok()
            .set_header("ETag", etag)
            .body(response);
    }

    web::HttpResponse::Ok().body(response)
}

#[web::post("/echo")]
async fn echo(req_body: String) -> impl web::Responder {
    web::HttpResponse::Ok().body(req_body)
}

async fn manual_hello() -> impl web::Responder {
    web::HttpResponse::Ok().body("Hey there!")
}
#[ntex::main]
async fn main() -> std::io::Result<()> {
    let mut cfg = Config::new();
    cfg.host = Some("localhost".to_string());
    cfg.dbname = Some("postgres".to_string());
    cfg.user = Some("postgres".to_string());
    cfg.password = Some("admin".to_string());
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls).unwrap();

    // line used to check the connection was successful
    let mut client = pool.get().await.unwrap();

    web::HttpServer::new(move || {
        web::App::new()
            .state(pool.clone())
            .service(hello)
            .service(echo)
            .service(images)
            .route("/hey", web::get().to(manual_hello))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
