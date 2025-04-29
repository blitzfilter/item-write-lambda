use aws_config::BehaviorVersion;
use aws_lambda_events::sqs::SqsEvent;
use aws_sdk_config::config::Credentials;
use aws_sdk_dynamodb::Client;
use item_write_lambda::function_handler;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt().json()
        .with_max_level(tracing::Level::INFO)
        .with_current_span(false)
        .with_ansi(false)
        .without_time()
        .init();

    let config = &aws_config::defaults(BehaviorVersion::latest())
        .credentials_provider(Credentials::for_tests())
        .region("eu-central-1")
        .endpoint_url("http://localhost.localstack.cloud:4566")
        .load()
        .await;
    let client = &Client::new(config);

    run(service_fn(move |event: LambdaEvent<SqsEvent>| async move {
        function_handler(client, event).await
    }))
    .await
}
