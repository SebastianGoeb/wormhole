use cfg_if::cfg_if;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};


cfg_if! {
    if #[cfg(feature = "ssr")] {
        use crate::user::*;
        use crate::state::*;
    }
}

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
async fn update(message: String) -> Result<(), ServerFnError> {
    let state = expect_context::<AppState>();

    state.value_service.update(UserId(DEFAULT_USER.to_string()), message).await;

    // let mut map = state.txs.lock().await;

    // let tx_weak = map.entry(UserId(DEFAULT_USER.to_string())).or_insert_with(|| {
    //     let (tx, _rx) = watch::channel(DEFAULT_VALUE.to_string());
    //     let sdr = Arc::new(tx);
    //     Arc::downgrade(&sdr)
    // });

    // let (tx, _rx) = watch::channel(DEFAULT_VALUE.to_string());
    // let sdr = Arc::new(tx);
    // map.insert(UserId(DEFAULT_USER.to_string()), Arc::downgrade(&sdr));

    // let tx = state.txs.get_with_by_ref(DEFAULT_USER, async move {
    //     log!("update: must create new channel first");
    //     let (tx, _rx) = watch::channel(DEFAULT_VALUE.to_string());
    //     NamedSender {name: COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), tx}
    // }).await;

    // log!("updating {}", tx.name);
    
    // tx.tx.send_if_modified(|state| {
    //     if *state != message {
    //         *state = message;
    //         return true;
    //     }
    //     return false;
    // });

    // // state.sender_cache.remove(DEFAULT_USER);
    // state.sender_cache.insert(DEFAULT_USER.to_string(), tx).await;

    Ok(())
}

#[server]
async fn get_current_value() -> Result<String, ServerFnError> {
    let state = expect_context::<AppState>();
    state.value_service.get_current_value(UserId(DEFAULT_USER.to_string())).await
}

#[server]
async fn await_new_value(last_seen: String) -> Result<String, ServerFnError> {
    let state = expect_context::<AppState>();
    state.value_service.await_different_value(UserId(DEFAULT_USER.to_string()), last_seen).await

    // let (tx, rx) = oneshot::channel::<Option<String>>();
    // state.command_tx.send(Command::AwaitDifferentValue(UserId(DEFAULT_USER.to_string()), last_seen, tx));
    // let value = rx.await.map_err(|_| ServerFnError::new("the sender dropped"))?; // TODO message

    // let tx = state.sender_cache.get_with_by_ref(DEFAULT_USER, async move {
    //     println!("await: must create new channel first");
    //     let (tx, _rx) = watch::channel(DEFAULT_VALUE.to_string());
    //     NamedSender {name: COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), tx}
    // }).await;

    // log!("awaiting {}", tx.name);
    
    // let mut rx = tx.tx.subscribe();

    // loop {
    //     let current = rx.borrow_and_update().clone();
    //     if current == "" {
    //         log!("received '' on {}", tx.name)
    //     }

    //     if current != last_seen {
    //         return Ok(current);
    //     }
        
    //     // Wait for the next update.
    //     // If an update happened between the check above and this line,
    //     // .changed() resolves immediately.
    //     rx.changed().await.map_err(|_| ServerFnError::new("Channel closed"))?;
    // }
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
                None =>  get_current_value().await.ok().or(Some("".to_string())),
                Some(last_seen) => Some(await_new_value(last_seen.clone()).await.unwrap_or_else(|_| last_seen))
            }
        },
    );

    // sync signal with poll-results
    Effect::new(move |_| {
        if let Some(Some(new_value)) = wait.get() {
            set_value.set(Some(new_value));
        }
    });

    let update_action = Action::new(|from_input: &String| {
        let from_input = from_input.clone();
        async move { update(from_input).await }
    });

    view! {
        <div class="grid min-h-dvh place-items-center">
            <input
                class="w-[50dvmin] h-[50dvmin] rounded-full bg-transparent border-10 border-amber-600 text-2xl text-orange-300 text-center"
                on:input=move |ev| {
                    let value = event_target_value(&ev);
                    update_action.dispatch(value.clone());
                }
                prop:value=move || { value }
            />
        </div>
    }
}
