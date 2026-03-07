mod grpc;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let pong = grpc::ping("http://0.0.0.0:50051").await?;
    println!("Got: {pong}");
    Ok(())
}
