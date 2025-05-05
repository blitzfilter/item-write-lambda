use aws_lambda_events::sqs::{BatchItemFailure, SqsBatchResponse, SqsEvent};
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::types::WriteRequest;
use item_core::item_data::ItemData;
use item_core::item_model::ItemModel;
use item_write::write_item_batch;
use lambda_runtime::{Error, LambdaEvent};
use serde_json::from_str;
use std::collections::HashMap;
use tracing::{error, info, warn};

#[tracing::instrument(skip(client, event), fields(req_id = %event.context.request_id))]
pub async fn function_handler(
    client: &Client,
    event: LambdaEvent<SqsEvent>,
) -> Result<SqsBatchResponse, Error> {
    let records_count = event.payload.records.len();
    info!(total = records_count, "Handler invoked.",);

    let mut batch_item_failures = Vec::new();
    let mut items = Vec::new();
    let mut item_id_message_id: HashMap<String, String> = HashMap::new();

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
                Ok(item_datum) => {
                    let item_model: ItemModel = item_datum.into();
                    item_id_message_id.insert(item_model.item_id.clone(), message_id);
                    items.push(item_model);
                }
                Err(e) => {
                    warn!(
                        messageId = %message_id,
                        error = %e,
                        body = %item_json,
                        "Deserialization error"
                    );
                    batch_item_failures.push(BatchItemFailure {
                        item_identifier: message_id,
                    });
                }
            },
        }
    }

    for item_chunk in items.chunks(item_write::MAX_BATCH_SIZE) {
        match write_item_batch(item_chunk, client).await {
            Ok(batch_output) => {
                if let Some(mut unprocessed) = batch_output.unprocessed_items {
                    let total_count = item_chunk.len();
                    let unprocessed_items = unprocessed.remove("items").unwrap_or_default();
                    let unprocessed_items_count = unprocessed_items.len();
                    info!(
                        total = total_count,
                        processed = total_count - unprocessed_items_count,
                        unprocessed = unprocessed_items_count,
                        "Successfully wrote items to DynamoDB"
                    );

                    if unprocessed_items_count > 0 {
                        unprocessed_items.into_iter().for_each(|wr| {
                            handle_unprocessed(
                                wr,
                                &mut batch_item_failures,
                                &mut item_id_message_id,
                            )
                        });
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Writing items in batch to DynamoDB failed.");
                handle_batch_error(
                    item_chunk,
                    &mut batch_item_failures,
                    &mut item_id_message_id,
                );
            }
        }
    }

    let failure_count = batch_item_failures.len();
    info!(
        successfull = records_count - failure_count,
        failed = failure_count,
        "Handler finished.",
    );

    Ok(SqsBatchResponse {
        batch_item_failures,
    })
}

fn handle_unprocessed(
    unprocessed: WriteRequest,
    batch_item_failures: &mut Vec<BatchItemFailure>,
    item_id_message_id: &mut HashMap<String, String>,
) {
    let item_map = unprocessed
        .put_request
        .expect("should always return PutRequest because it was attempted to write to DynamoDB")
        .item;
    match serde_dynamo::from_item::<_, ItemModel>(item_map) {
        Ok(item_model) => {
            let item_id = item_model.item_id.as_str();
            let message_id = item_id_message_id.remove(item_id);
            match message_id {
                Some(message_id) => batch_item_failures.push(BatchItemFailure {
                    item_identifier: message_id.to_owned(),
                }),
                None => {
                    error!(
                        messageId = item_id,
                        "Could not find the message_id for a failed item.",
                    )
                }
            }
        }
        Err(e) => {
            error!(
                error = %e,
                "Failed converting an unprocessed item from DynamoDB to ItemModel."
            )
        }
    }
}

fn handle_batch_error(
    item_chunk: &[ItemModel],
    batch_item_failures: &mut Vec<BatchItemFailure>,
    item_id_message_id: &mut HashMap<String, String>,
) {
    item_chunk
        .into_iter()
        .for_each(|item| match item_id_message_id.remove(&item.item_id) {
            Some(message_id) => batch_item_failures.push(BatchItemFailure {
                item_identifier: message_id,
            }),
            None => {
                error!(
                    messageId = &item.item_id,
                    "Could not find the message_id for a failed item.",
                )
            }
        });
}
