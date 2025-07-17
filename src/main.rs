use masm_project_template::common::instantiate_client;
use miden_client::rpc::Endpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = instantiate_client(Endpoint::testnet()).await?;

    client.sync_state().await?;

    let sync_height = client.get_sync_height().await?;
    println!("sync_height: {}", sync_height);

    Ok(())
}
