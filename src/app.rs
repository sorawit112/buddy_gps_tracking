use crate::gps_data::StoredData;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::*;
use leptos_meta::*;
use leptos_struct_table::*;

use gloo_net::http::Request;
use wasm_bindgen::JsCast;
use web_sys::BlobPropertyBag;

/// Asynchronously fetches all data from the backend API
async fn fetch_api_data() -> Result<Vec<StoredData>, ServerFnError<()>> {
    let url = "/api/data";

    // --- REPLACED reqwest WITH gloo_net::http ---
    let response = Request::get(url)
        .send()
        .await
        .map_err(|e| ServerFnError::<()>::ServerError(format!("Fetch failed: {}", e)))?;

    if !response.ok() {
        return Err(ServerFnError::<()>::ServerError(format!(
            "Server returned status code {}",
            response.status()
        )));
    }

    // gloo-net has a built-in method to deserialize the body
    let data = response
        .json::<Vec<StoredData>>()
        .await
        .map_err(|e| ServerFnError::<()>::Deserialization(format!("JSON parsing failed: {}", e)))?;
    log!("Data fetched successfully, rows: {}", data.len());
    Ok(data)
}

/// Triggers a client-side download of the provided data as a CSV file
fn trigger_csv_download(data: Vec<StoredData>) {
    // 1. Build CSV content
    let mut csv_content = "id,timestamp,longitude,latitude,battery\n".to_string();
    for entry in data {
        csv_content.push_str(&format!(
            "{},{},{},{},{}\n",
            entry.id, entry.timestamp, entry.longitude, entry.latitude, entry.battery
        ));
    }

    // 2. Use web_sys to create a blob and trigger download
    let document = document();
    let body = document.body().expect("document to have a body");

    let properties = BlobPropertyBag::new();
    properties.set_type("text/csv;charset=utf-8;");

    let blob = web_sys::Blob::new_with_str_sequence_and_options(
        &js_sys::Array::of1(&csv_content.into()),
        &properties,
    )
    .expect("Failed to create blob");

    // Create an object URL
    let url =
        web_sys::Url::create_object_url_with_blob(&blob).expect("Failed to create object URL");

    // Create a temporary <a> element to trigger download
    let a = document
        .create_element("a")
        .expect("Failed to create 'a' element")
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .expect("Failed to cast to HtmlAnchorElement");

    a.set_href(&url);
    a.set_download("gps_data.csv");

    // Append, click, and remove
    body.append_child(&a).unwrap();
    a.click();
    body.remove_child(&a).unwrap();

    // Clean up the object URL
    web_sys::Url::revoke_object_url(&url).unwrap();
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    // Resource to hold the data from the API
    let data_resource = LocalResource::new(|| async move { fetch_api_data().await });

    // This signal will be used to store the data for the download button
    let (csv_data, set_csv_data) = signal(Vec::<StoredData>::new());

    // When the resource loads, update the csv_data signal
    Effect::new(move |_| {
        if let Some(Ok(data)) = data_resource.get() {
            set_csv_data.set(data);
        } else {
            log!("Resource is still loading or None");
        }
    });

    Effect::new(move |_| {
        if let Some(result) = data_resource.get() {
            log!("Resource state changed!");
            match result {
                Ok(data) => log!("Fetched {} rows", data.len()),
                Err(e) => log!("Error: {:?}", e),
            }
        } else {
            log!("Resource is still loading or None");
        }
    });

    let on_download_click = move |_| {
        let data_to_download = csv_data.get_untracked();
        if !data_to_download.is_empty() {
            trigger_csv_download(data_to_download);
        } else {
            log!("No data to download.");
        }
    };

    view! {
        <Stylesheet id="leptos" href="/pkg/buddy_app.css"/>
        // Outer container: Soft, welcoming background and minimum screen height
        <div class="min-h-screen bg-amber-50 p-4 font-sans antialiased">
            // Main Card: Centered, rounded, shadowed container for the dashboard content
            <main class="max-w-4xl mx-auto bg-white rounded-3xl shadow-2xl p-6 md:p-10">

                // Header: Large, bold, and themed
                <header class="text-center mb-8">
                    <h1 class="text-4xl font-extrabold text-teal-800 tracking-tight">
                        <i class="fas fa-paw mr-3 text-amber-500"></i>
                        "ESP32 Pet Tracker Dashboard"
                    </h1>
                    <p class="text-gray-600 mt-2">"GPS tracking for your furry companion."</p>
                </header>

                // Action Buttons: Grouped, well-styled, and responsive
                <div class="flex flex-wrap justify-center gap-4 mb-10 border-b pb-6 border-amber-200">
                    <button
                        on:click=move |_| data_resource.refetch()
                        class="flex items-center space-x-2 bg-teal-600 hover:bg-teal-700 text-white font-semibold py-3 px-6 rounded-xl shadow-lg transition duration-300 transform hover:scale-[1.02] active:scale-[0.98] focus:outline-none focus:ring-4 focus:ring-teal-300"
                    >
                        <i class="fas fa-sync-alt"></i>
                        <span>"Refresh Data"</span>
                    </button>
                    <button
                        on:click=on_download_click
                        class="flex items-center space-x-2 bg-amber-500 hover:bg-amber-600 text-white font-semibold py-3 px-6 rounded-xl shadow-lg transition duration-300 transform hover:scale-[1.02] active:scale-[0.98] focus:outline-none focus:ring-4 focus:ring-amber-300"
                    >
                        <i class="fas fa-download"></i>
                        <span>"Download CSV"</span>
                    </button>
                </div>

                // Data Section Header
                <h2 class="text-2xl font-bold text-teal-800 mb-4 border-l-4 border-amber-400 pl-3">
                    "Latest Refreshed Datas"
                </h2>

                // Data Display Area: Card-like container for the data table
                <div class="overflow-x-auto rounded-xl shadow-lg ring-1 ring-gray-200">
                    <Suspense fallback=move || view! {
                        <p class="p-6 text-center text-gray-500 bg-gray-50 rounded-xl">"Fetching cuddly data..." <i class="fas fa-bone animate-pulse ml-2"></i></p>
                    }>
                        <ErrorBoundary
                            fallback=|_| view! {
                                <p class="p-6 text-center text-red-600 bg-red-50 rounded-xl">"Ruh-roh! Error loading data. Check the tracker connection." <i class="fas fa-exclamation-triangle ml-2"></i></p>
                            }
                        >
                            {
                                move || data_resource.get().map(|data| match data {
                                    Ok(data_vec) if data_vec.is_empty() => {
                                        view! {
                                            <p class="p-6 text-center text-gray-500">
                                                "No sensor data received yet. Is the tracker awake?"
                                            </p>
                                        }.into_any()
                                    }
                                    Ok(data_vec) => {
                                        view! {
                                            <div class="rounded-md overflow-clip m-10 border dark:border-gray-700".to_string()>
                                                <table class="text-sm text-left text-gray-500 dark:text-gray-400 mb-[-1px] w-[calc(100vw-5rem)]">
                                                    <TableContent rows=data_vec scroll_container="html" />
                                                </table>
                                            </div>
                                        }.into_any()
                                    },
                                    Err(_e) => {
                                        view! {
                                            <p class="p-6 text-center text-red-500">
                                                {format!("Error: Failed to load data from the tracker.")}
                                            </p>
                                        }.into_any()
                                    }
                                })
                            }
                        </ErrorBoundary>
                    </Suspense>
                </div>

                // Friendly Footer
                <footer class="mt-8 pt-4 text-center text-sm text-gray-500 border-t border-amber-100">
                    "Adventures tracked with love." <i class="fas fa-heart ml-1 text-red-400"></i>
                </footer>
            </main>
        </div>
    }
}

// use leptos::prelude::*;
// use leptos_meta::{provide_meta_context, Stylesheet, Title};
// use leptos_router::{
//     components::{Route, Router, Routes},
//     StaticSegment, WildcardSegment,
// };

// #[component]
// pub fn App() -> impl IntoView {
//     // Provides context that manages stylesheets, titles, meta tags, etc.
//     provide_meta_context();

//     view! {
//         // injects a stylesheet into the document <head>
//         // id=leptos means cargo-leptos will hot-reload this stylesheet
//         <Stylesheet id="leptos" href="/pkg/buddy.css"/>

//         // sets the document title
//         <Title text="Welcome to Leptos"/>

//         // content for this welcome page
//         <Router>
//             <main>
//                 <Routes fallback=move || "Not found.">
//                     <Route path=StaticSegment("") view=HomePage/>
//                     <Route path=WildcardSegment("any") view=NotFound/>
//                 </Routes>
//             </main>
//         </Router>
//     }
// }

// /// Renders the home page of your application.
// #[component]
// fn HomePage() -> impl IntoView {
//     // Creates a reactive value to update the button
//     let count = RwSignal::new(0);
//     let on_click = move |_| *count.write() += 1;

//     view! {
//         <h1>"Welcome to Leptos!"</h1>
//         <button on:click=on_click>"Click Me: " {count}</button>
//     }
// }

// /// 404 - Not Found
// #[component]
// fn NotFound() -> impl IntoView {
//     // set an HTTP status code 404
//     // this is feature gated because it can only be done during
//     // initial server-side rendering
//     // if you navigate to the 404 page subsequently, the status
//     // code will not be set because there is not a new HTTP request
//     // to the server
//     #[cfg(feature = "ssr")]
//     {
//         // this can be done inline because it's synchronous
//         // if it were async, we'd use a server function
//         let resp = expect_context::<leptos_actix::ResponseOptions>();
//         resp.set_status(actix_web::http::StatusCode::NOT_FOUND);
//     }

//     view! {
//         <h1>"Not Found"</h1>
//     }
// }
