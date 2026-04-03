mod checker;
mod export;
mod result;
mod server;
mod sites;

use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!();
    println!("  \x1b[36m######  ##  ## ###### ##### ##    ####   #### ##  ##\x1b[0m");
    println!("  \x1b[36m##      ##  ## ##     ##  # ##   ##  ## ##    ## ##\x1b[0m");
    println!("  \x1b[36m######  ###### ####   ####  ##   ##  ## ##    ####\x1b[0m");
    println!("  \x1b[36m    ## ##  ## ##     ## ## ##   ##  ## ##    ## ##\x1b[0m");
    println!("  \x1b[36m###### ##  ## ###### ##  # ##### ####   #### ##  ##\x1b[0m");
    println!("  \x1b[90m────────────────── RS Edition v1.0.0 ──────────────────\x1b[0m");
    println!("  \x1b[90m  Author: Olivier Hoarau <tarraw974@gmail.com>\x1b[0m");
    println!();

    let state = Arc::new(server::AppState::new());

    // Load sites database in background
    let state_clone = state.clone();
    tokio::spawn(async move {
        print!("  Loading sites database... ");
        match sites::load_sites().await {
            Ok(s) => {
                let count = s.len();
                *state_clone.sites.write().await = Some(s);
                println!("\x1b[32m{} sites loaded\x1b[0m", count);
            }
            Err(e) => {
                println!("\x1b[31mFailed: {}\x1b[0m", e);
                *state_clone.load_error.write().await = Some(e.to_string());
            }
        }
    });

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    println!("  \x1b[32m Server:  http://127.0.0.1:{}\x1b[0m", port);
    println!("  \x1b[90m Press Ctrl+C to stop\x1b[0m");
    println!();

    // Open browser
    let _ = open::that(format!("http://127.0.0.1:{}", port));

    let app = server::create_router(state);
    axum::serve(listener, app).await?;

    Ok(())
}
