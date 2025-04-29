use aws_lambda_events::sqs::{SqsEvent, SqsMessage};
use item_core::item_data::ItemData;
use item_core::item_model::ItemModel;
use item_read::item::get_item_events_by_item_id_sort_latest;
use item_write_lambda::function_handler;
use lambda_runtime::{Context, LambdaEvent};
use serde_json::to_string;
use test_api::generator::Generator;
use test_api::localstack::get_dynamodb_client;
use test_api::test_api_macros::blitzfilter_dynamodb_test;

#[blitzfilter_dynamodb_test]
async fn should_write_events_in_payload() {
    let client = get_dynamodb_client().await;
    let items = ItemModel::generate_many(10);
    let item_payloads = items
        .iter()
        .map(|item| Into::<ItemData>::into(item.clone()))
        .map(|item| {
            let mut msg = SqsMessage::default();
            msg.body = to_string(&item).ok();
            msg
        })
        .collect();
    let event = LambdaEvent::new(
        SqsEvent {
            records: item_payloads,
        },
        Context::default(),
    );

    let res = function_handler(&client, event).await;
    assert!(res.is_ok());

    for item in items {
        let read_res = get_item_events_by_item_id_sort_latest(&item.item_id, client).await;
        assert!(read_res.is_ok());
        let read = read_res.unwrap();
        assert_eq!(read.len(), 1);
        let read_item = read.get(0).unwrap();
        assert_eq!(*read_item, item);
    }
}
