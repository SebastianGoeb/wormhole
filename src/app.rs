use leptos::{prelude::*, reactive::spawn_local};
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};


#[cfg(feature = "ssr")]
use crate::state::AppState;

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
                <script src="https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4"></script>
            </head>
            <body class="bg-slate-950">
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
        <Title text="Wormhole" />

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
async fn update_and_await_value(message: String) -> Result<String, ServerFnError> {
    update(message.clone()).await?;
    await_value(message).await
}

#[server]
async fn update(message: String) -> Result<(), ServerFnError> {
    let state = expect_context::<AppState>();
    let sender = state.value_tx.clone();
    sender.send_if_modified(|state| {
        if *state != message {
            *state = message;
            return true;
        }
        return false;
    });
    Ok(())
}

#[server]
async fn get_value() -> Result<String, ServerFnError> {
    let state = expect_context::<AppState>();
    let value = state.value_tx.subscribe().borrow().clone();
    Ok(value)
}

#[server]
async fn await_value(last_seen: String) -> Result<String, ServerFnError> {
    let state = expect_context::<AppState>();
    let mut rx = state.value_tx.subscribe();

    loop {
        let current = rx.borrow().clone();
        if current != last_seen {
            return Ok(current);
        }
        
        // Wait for the next update.
        // If an update happened between the check above and this line,
        // .changed() resolves immediately.
        rx.changed().await.map_err(|_| ServerFnError::new("Channel closed"))?;
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let (value, set_value) = signal::<Option<String>>(None);

    // long polling
    let wait = Resource::new(
        move || value.get(),
        move |last_seen| async move {
            return match last_seen {
                None =>  get_value().await.ok(),
                Some(last_seen) => await_value(last_seen).await.ok()
            }
        },
    );

    // sync signal with poll-results
    Effect::new(move |_| {
        if let Some(Some(new_value)) = wait.get() {
            set_value.set(Some(new_value));
        }
    });

    view! {
        <div class="grid min-h-dvh place-items-center">
            <input
                class="w-[50dvmin] h-[50dvmin] rounded-full bg-transparent border-10 border-amber-600 text-2xl text-orange-300 text-center"
                on:input=move |ev| {
                    let value = event_target_value(&ev);
                    spawn_local(async move {
                        let new_value = update_and_await_value(value).await.ok();
                        set_value.set(new_value.clone());
                    });
                }
                prop:value=move || { value }
            />
        </div>
    }
}
