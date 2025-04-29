use aws_lambda_events::sqs::SqsEvent;
use aws_sdk_dynamodb::Client;
use item_core::item_data::ItemData;
use item_core::item_model::ItemModel;
use item_write::write_items;
use lambda_runtime::{Error, LambdaEvent};
use serde_json::{from_str};
use tracing::info;

#[tracing::instrument(skip(event), fields(req_id = %event.context.request_id))]
pub async fn function_handler(client: &Client, event: LambdaEvent<SqsEvent>) -> Result<(), Error> {
    info!("Invoked lambda.");
    
    info!("{:?}", event);
    
    let items = event
        .payload
        .records
        .into_iter()
        .filter_map(|datum| datum.body)
        .map(|item_datum_json| from_str::<ItemData>(&item_datum_json))
        // TODO: Don't swallow error, handle
        .filter_map(|item_datum| item_datum.ok())
        .map(ItemData::into)
        .collect::<Vec<ItemModel>>();

    info!("{:?}", items);
    
    write_items(&items, client).await;

    info!("Wrote items, finished invocation");
    
    // TODO: Retries, DLQ, partial failure, ...

    Ok(())
}
