use anyhow::Result;
use bongas_ai::engine::execution::BongasRuntime;

#[tokio::main]
async fn main() -> Result<()> {
    // Bootstrap and Run the Symphony 2.0 Engine
    let runtime: BongasRuntime = BongasRuntime::init().await?;
    
    runtime.run().await
}
