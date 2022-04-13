use std::{fs, thread};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actix_files::NamedFile;
use actix_web::{
    App,
    HttpRequest,
    HttpResponse, HttpServer, middleware::Logger, Responder, web::{self, Bytes},
};
use env_logger::Env;

const SECONDS_IN_3_DAYS: u64 = 259200;
const API_KEY: &str = "hello";

pub fn get_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

#[actix_web::post("/upload_page")]
async fn upload_page(req: HttpRequest, body: Bytes) -> impl Responder {
    //only let authorized people through (only us)
    match req.headers().get("api_key") {
        Some(key) => {
            if key != API_KEY {
                return HttpResponse::Unauthorized().body("invalid api key");
            }
        }
        None => return HttpResponse::Unauthorized().body("no api key"),
    }

    let user_id = match req.headers().get("user_id") {
        Some(id) => id.to_str().unwrap(),
        None => return HttpResponse::BadRequest().body("no user_id"),
    };

    let path = get_timestamp() + "_" + user_id + ".html";
    match fs::write("pages/".to_owned() + &path, body) {
        Ok(_) => HttpResponse::Ok().body(path),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[actix_web::get("/{filename}")]
async fn serve_file(req: HttpRequest) -> std::io::Result<NamedFile> {
    let path: String = "pages/".to_owned() + req.match_info().query("filename");
    Ok(NamedFile::open(path).unwrap())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    if !Path::new("pages").is_dir() {
        fs::create_dir("pages")?;
    }

    thread::spawn(move || -> std::io::Result<()> {
        loop {
            let dir = fs::read_dir("pages")?;

            for file in dir {
                let file = file.unwrap();
                if let Ok(creation_time) = file.metadata().unwrap().created() {
                    if SystemTime::now().duration_since(creation_time).unwrap().as_secs() > SECONDS_IN_3_DAYS {
                        println!("deleting file {}", file.path().to_str().unwrap());
                        fs::remove_file(file.path())?;
                    }
                }
            }

            std::thread::sleep(Duration::from_secs(10));
        }
    });

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::PayloadConfig::new(500_000))
            .service(upload_page)
            .service(serve_file)
    })
        .bind(("127.0.0.1", 7070))?
        .run()
        .await
}
