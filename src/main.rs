use aws_config::BehaviorVersion;
use aws_lambda_events::sqs::SqsEvent;
use aws_sdk_config::config::Credentials;
use aws_sdk_dynamodb::Client;
use item_write_lambda::function_handler;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = &aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(Credentials::for_tests())
        .region("eu-central-1")
        .endpoint_url("http://localhost:4566")
        .load()
        .await;
    let client = &Client::new(config);

    run(service_fn(|event: LambdaEvent<SqsEvent>| async {
        function_handler(event, client).await
    }))
    .await
}
