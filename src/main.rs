use askama::Template; // Import Template trait from askama for HTML templating
use axum::{
    // Import axum crate for building web applications
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Form,
    Router,
};
use serde::Deserialize; // Import Deserialize trait from serde for deserializing form data
use std::sync::Arc; // Import Arc for thread-safe reference counting
use std::sync::Mutex; // Import Mutex for thread-safe mutual exclusion
use tower_http::services::ServeDir; // Import ServeDir for serving static files
use tracing::info; // Import info! macro from tracing for logging
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt}; // Import traits from tracing_subscriber for initializing tracing

// Define a struct to hold the application state
struct AppState {
    todos: Mutex<Vec<String>>, // Use Mutex to safely share todos among multiple threads
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_htmx=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create a shared state object using Arc
    let app_state = Arc::new(AppState {
        todos: Mutex::new(vec![]), // Initialize an empty vector for todos
    });

    // Initialize the router
    info!("router init...");
    // Get the current directory for serving assets
    let assets_path = std::env::current_dir().unwrap();
    let router = Router::new()
        .route("/", get(main_page)) // Define a route for the home page
        .route("/another-page", get(another_page)) // Define a route for another page
        .route("/api", get(api_sample)) // Define a route for API sample
        .route("/todos", post(add_todo)) // Define a route for adding todos
        .nest_service(
            "/assets",
            ServeDir::new(format!("{}/assets", assets_path.to_str().unwrap())), // Serve static assets
        )
        .with_state(app_state); // Pass the shared state to the router

    // Configure the server to listen on a specific port
    let port = 8086_u16;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap(); // Bind the listener to the specified port

    info!("router init complete: now listening on port {}", port);
    // Start serving the router
    axum::serve(listener, router).await.unwrap();

    Ok(())
}

// Handler for API sample route
async fn api_sample() -> &'static str {
    "Hello from Axum backend!" // Return a simple message
}

// Struct for holding the main page template data
#[derive(Template)]
#[template(path = "app.html")]
struct MainPageTemplate;

// Handler for the home page route
async fn main_page() -> impl IntoResponse {
    let template = MainPageTemplate {}; // Initialize the HelloTemplate struct
    HtmlTemplate(template) // Wrap the template in HtmlTemplate for serving
}

// Struct for holding the another page template data
#[derive(Template)]
#[template(path = "another-page.html")]
struct AnotherPageTemplate;

// Handler for another page route
async fn another_page() -> impl IntoResponse {
    let template = AnotherPageTemplate {}; // Initialize the AnotherPageTemplate struct
    HtmlTemplate(template) // Wrap the template in HtmlTemplate for serving
}

// Struct for holding the todo list template data
#[derive(Template)]
#[template(path = "todo-list.html")]
struct TodoList {
    todos: Vec<String>, // Define a vector to hold todo items
}

// Struct for holding form data
#[derive(Deserialize)]
struct FormData {
    todo: String, // Define a field for the todo item
}

// Handler for adding todo items
async fn add_todo(
    State(state): State<Arc<AppState>>, // Get the shared state
    Form(form_data): Form<FormData>,    // Extract form data
) -> impl IntoResponse {
    let mut lock = state.todos.lock().unwrap(); // Lock the todos vector for mutation
    lock.push(form_data.todo); // Add the new todo item

    let template = TodoList {
        todos: lock.clone(), // Clone the todos vector for rendering the template
    };

    HtmlTemplate(template) // Wrap the template in HtmlTemplate for serving
}

// Wrapper type for encapsulating HTML templates
struct HtmlTemplate<T>(T);

// Implementation of IntoResponse trait for converting HTML templates into valid HTML for serving
impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        // Attempt to render the template with askama
        match self.0.render() {
            // If successful, serve the rendered HTML
            Ok(html) => Html(html).into_response(),
            // If an error occurs, return an internal server error response
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}
