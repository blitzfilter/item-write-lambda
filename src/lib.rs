use aws_lambda_events::sqs::{BatchItemFailure, SqsBatchResponse, SqsEvent};
use aws_sdk_dynamodb::Client;
use item_core::item_data::ItemData;
use item_write::write_items;
use lambda_runtime::{Error, LambdaEvent};
use serde_json::from_str;
use tracing::{info, warn};

#[tracing::instrument(skip(event), fields(req_id = %event.context.request_id))]
pub async fn function_handler(
    client: &Client,
    event: LambdaEvent<SqsEvent>,
) -> Result<SqsBatchResponse, Error> {
    let mut batch_item_failures = Vec::new();
    let mut items = Vec::new();

    for item_rec in event.payload.records {
        let message_id = item_rec
            .message_id
            .expect("should never receive an SQS-Message without message_id because AWS sets it.");

        match item_rec.body {
            None => {
                warn!("Received empty body for message_id '{message_id}'");
                batch_item_failures.push(BatchItemFailure {
                    item_identifier: message_id,
                });
            }
            Some(ref item_json) => match from_str::<ItemData>(item_json) {
                Ok(item_datum) => items.push(item_datum.into()),
                Err(e) => {
                    warn!(
                        "Deserialization error for message_id '{message_id}': {e}. Body: '{item_json}'"
                    );
                    batch_item_failures.push(BatchItemFailure {
                        item_identifier: message_id,
                    });
                }
            },
        }
    }

    write_items(&items, client)
        .await
        .into_iter()
        .for_each(|res| {
            info!("{:?}", res);
            todo!()
        });

    Ok(SqsBatchResponse {
        batch_item_failures,
    })
}
