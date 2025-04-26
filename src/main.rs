use aws_config::BehaviorVersion;
use aws_sdk_config::config::Credentials;
use aws_sdk_dynamodb::Client;
use item_core::item_data::ItemData;
use item_core::item_model::ItemModel;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use serde_json::{Value, from_value};
use item_write::write_items;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = &aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(Credentials::for_tests())
        .region("eu-central-1")
        .endpoint_url("http://localhost:4566")
        .load()
        .await;
    let client = &Client::new(config);

    run(service_fn(|event: LambdaEvent<Value>| async move {
        function_handler(event, client).await
    }))
    .await
}

async fn function_handler(event: LambdaEvent<Value>, client: &Client) -> Result<(), Error> {
    let payload = event.payload;
    let items = from_value::<Vec<ItemData>>(payload)?
        .iter()
        .map(|datum| datum.to_owned().into())
        .collect::<Vec<ItemModel>>();
    
    let write_responses = write_items(&items, client).await;

    // TODO: Retries, DLQ, partial failure, ...
    
    Ok(())
}
