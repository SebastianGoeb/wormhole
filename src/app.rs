use leptos::{logging, prelude::*, reactive::spawn_local};
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/wormhole.css" />

        // sets the document title
        <Title text="Bro, welcome to Leptos" />

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage />
                </Routes>
            </main>
        </Router>
    }
}

#[server]
async fn log_something(message: String) -> Result<(), ServerFnError> {
    logging::log!("{message}");
    Ok(())
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <h1>"Welcome to Wormhole!"</h1>

        <input
            type="text"
            on:input=move |ev| {
                let v = event_target_value(&ev);
                spawn_local(async {
                    log_something(v).await;
                });
            }
        />
    }
}
