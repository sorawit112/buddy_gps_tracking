use actix_web::{get, post, web};
use leptos::logging::log;

use buddy::gps_data::{IncomingData, StoredData};
use std::sync::{Arc, RwLock};

/// Our in-memory database, wrapped for safe concurrent access.
struct AppState {
    data_points: Arc<RwLock<Vec<StoredData>>>,
}

// --- API Handlers (Actix) ---

/**
 * Handles POST requests from the ESP32.
 * It parses the incoming JSON and stores the data.
 */
#[cfg(any(feature = "ssr", feature = "csr"))]
#[post("/api/data")]
async fn receive_data(
    item: web::Json<IncomingData>,
    state: web::Data<AppState>,
) -> impl actix_web::Responder {
    use actix_web::HttpResponse;
    log!("Received data: {:?}", item);

    // Parse string data into numeric values
    // We use unwrap_or(0) for simplicity. In production, you'd handle errors.
    // (longitude, latitude, battery): (u16, u16, u8)
    let parsed_item: Result<(u16, u16, u8), Box<dyn std::error::Error>> = item.parse_hex_payload();

    if let Err(e) = parsed_item {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"status": "error", "message": format!("Failed to parse payload: {}", e)}));
    };

    let (longitude, latitude, battery) = parsed_item.unwrap();

    let new_data = StoredData {
        id: item.id.clone(),
        longitude: longitude,
        latitude: latitude,
        battery: battery,
        timestamp: format!("{} {}", item.date, item.time),
    };

    // Lock the data store and add the new entry
    match state.data_points.write() {
        Ok(mut data_store) => {
            data_store.push(new_data);
            HttpResponse::Ok().json(serde_json::json!({"status": "success"}))
        }
        Err(_) => HttpResponse::InternalServerError()
            .json(serde_json::json!({"status": "error", "message": "Failed to lock data store"})),
    }
}

/**
 * Handles GET requests from the Leptos frontend.
 * It returns all currently stored data.
 */
#[cfg(any(feature = "ssr", feature = "csr"))]
#[get("/api/data")]
async fn get_data(state: web::Data<AppState>) -> impl actix_web::Responder {
    use actix_web::HttpResponse;
    match state.data_points.read() {
        Ok(data_store) => HttpResponse::Ok().json(&*data_store),
        Err(_) => HttpResponse::InternalServerError()
            .json(serde_json::json!({"status": "error", "message": "Failed to read data store"})),
    }
}

// --- Main Function ---
#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Import the frontend app component
    use actix_files::Files;
    use actix_web::*;
    use buddy::app::App;
    use leptos::config::get_configuration;

    use leptos::prelude::*;
    use leptos_actix::{LeptosRoutes, generate_route_list};
    use leptos_meta::MetaTags;

    // Set up the in-memory state
    let state = web::Data::new(AppState {
        data_points: Arc::new(RwLock::new(vec![])),
    });

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;

    HttpServer::new(move || {
        let routes = generate_route_list(App);
        let leptos_options = &conf.leptos_options;
        let site_root = leptos_options.site_root.clone().to_string();
        log!("Listening on http://{}", &addr);

        App::new()
            .app_data(state.clone()) // Add state to Actix
            .service(receive_data) // Add POST handler
            .service(get_data) // Add GET handler
            .leptos_routes(routes.to_owned(), {
                let leptos_options = leptos_options.clone();
                move || {
                    view! {
                        <!DOCTYPE html>
                        <html lang="en">
                            <head>
                                <meta charset="utf-8"/>
                                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                                <AutoReload options=leptos_options.clone() />
                                <HydrationScripts options=leptos_options.clone()/>
                                <MetaTags/>
                            </head>
                            <body>
                                <App/>
                            </body>
                        </html>
                    }
                }
            })
            .service(Files::new("/", site_root))
    })
    .bind(&addr)?
    .run()
    .await
}

// #[cfg(feature = "ssr")]
// #[actix_web::main]
// async fn main() -> std::io::Result<()> {
//     use actix_files::Files;
//     use actix_web::*;
//     use leptos::prelude::*;
//     use leptos::config::get_configuration;
//     use leptos_meta::MetaTags;
//     use leptos_actix::{generate_route_list, LeptosRoutes};
//     use buddy::app::*;

//     let conf = get_configuration(None).unwrap();
//     let addr = conf.leptos_options.site_addr;

//     HttpServer::new(move || {
//         // Generate the list of routes in your Leptos App
//         let routes = generate_route_list(App);
//         let leptos_options = &conf.leptos_options;
//         let site_root = leptos_options.site_root.clone().to_string();

//         println!("listening on http://{}", &addr);

//         App::new()
//             // serve JS/WASM/CSS from `pkg`
//             .service(Files::new("/pkg", format!("{site_root}/pkg")))
//             // serve other assets from the `assets` directory
//             .service(Files::new("/assets", &site_root))
//             // serve the favicon from /favicon.ico
//             .service(favicon)
//             .leptos_routes(routes, {
//                 let leptos_options = leptos_options.clone();
//                 move || {
//                     view! {
//                         <!DOCTYPE html>
//                         <html lang="en">
//                             <head>
//                                 <meta charset="utf-8"/>
//                                 <meta name="viewport" content="width=device-width, initial-scale=1"/>
//                                 <AutoReload options=leptos_options.clone() />
//                                 <HydrationScripts options=leptos_options.clone()/>
//                                 <MetaTags/>
//                             </head>
//                             <body>
//                                 <App/>
//                             </body>
//                         </html>
//                     }
//                 }
//             })
//             .app_data(web::Data::new(leptos_options.to_owned()))
//         //.wrap(middleware::Compress::default())
//     })
//     .bind(&addr)?
//     .run()
//     .await
// }
//
#[cfg(not(any(feature = "ssr", feature = "csr")))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
    // see optional feature `csr` instead
}

#[cfg(all(not(feature = "ssr"), feature = "csr"))]
pub fn main() {
    // a client-side main function is required for using `trunk serve`
    // prefer using `cargo leptos serve` instead
    // to run: `trunk serve --open --features csr`
    use buddy::app::*;

    console_error_panic_hook::set_once();

    leptos::mount_to_body(App);
}
