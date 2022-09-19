mod models;
mod schema;
use models::*;
use r2d2::Pool;
use schema::cats::dsl::*;

use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use serde::Serialize;
use diesel::{pg::PgConnection, prelude::*, r2d2::ConnectionManager};
use dotenvy::dotenv;
use handlebars::Handlebars;

type DbPool = Pool<ConnectionManager<PgConnection>>;
async fn index(hb: web::Data<Handlebars<'_>>, dbpool: web::Data<DbPool>) -> HttpResponse {
    let mut db_conn = dbpool.get().expect("Failed to get database connection");
    //load cats from db
    let cats_data = cats
        .limit(64)
        .load::<Cat>(&mut db_conn)
        .expect("Error loading cats data");

    #[derive(Serialize, Debug)]
    struct IndexTemplateData {
        project_name: String,
        cats: Vec<Cat>,
    }

    let data = IndexTemplateData {
        project_name: "Catdex".to_string(),
        cats: cats_data,
    };

    if let Ok(body) = hb.render("index", &data) {
        //if the template name incorrect, it will throw error
        //if data name is incorrect, compiler complains
        //if either data type or data content incorrect, compiler silennce,
        //  and will return the template without data attached
        HttpResponse::Ok().body(body)
    } else {
        //in case template name not found
        //Give message to user and ask them for bug report
        HttpResponse::Ok().body(
            "Aww.. Nothing to show, server error. Please file an issue at github.com/ahmad-su",
        )
    }
}
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use std::env;
    dotenv().ok();

    let mut handlebars = Handlebars::new();
    //method from feature='dir_source' to register template dir
    //the template filename *.html or *.else will be the registered template 'name'
    //used in rendering the template
    handlebars
        .register_templates_directory(".html", "./static/")
        .unwrap();
    //shared_handlebars will be distributed between threads
    //the web::Data is nothing more than just a Arc<Mutex<T>> behind the scene
    //so we only need to call clone on it in the App factory
    //to distribute it between threads
    let shared_handlebars = web::Data::new(handlebars);

    //Configure database connection pool using .env file via dotenvy
    //But you must apply security best practices when realeasing for production
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL variable not found");
    let db_man = ConnectionManager::<PgConnection>::new(&database_url);
    //Build db pool
    let db_pool = r2d2::Pool::builder()
        .build(db_man)
        .expect("Failed to create DB connection pool");
    //wrap db_pool inside web::Data (Arc) so it became sharable
    //between threads
    let db_pool = web::Data::new(db_pool);

    println!("Listening on port 8080..");
    HttpServer::new(move || {
        App::new()
            //clone AppData which contains the shared handlebars (wrapped inside web<Data<T>>)
            .app_data(shared_handlebars.clone())
            //share DB conn pool Arc to all threads
            //so each threads will have ownership over the pool
            .app_data(db_pool.clone())
            //Warning! Don't activate show_files_listing on production
            .service(Files::new("/static", "static").show_files_listing())
            .route("/", web::get().to(index))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
