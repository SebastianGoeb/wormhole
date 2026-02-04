use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use tokio::sync::watch;

        #[derive(Debug, Clone)]
        pub struct AppState {
            pub value_rx: watch::Receiver<String>,
            pub value_tx: watch::Sender<String>
        }
    }
}
