//! BONGAS-AI service entrypoint.

use anyhow::Result;
use bongas_ai::engine::runtime::BongasRuntime;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the application components via the engine runtime
    let runtime = BongasRuntime::init().await?;

    // Run the application
    runtime.run().await
}
