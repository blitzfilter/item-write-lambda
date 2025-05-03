use aws_config::BehaviorVersion;
use aws_lambda_events::sqs::SqsEvent;
use aws_sdk_dynamodb::Client;
use item_write_lambda::function_handler;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use std::env;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        .with_current_span(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_ansi(false)
        .without_time()
        .init();

    match dotenvy::from_filename(".env.localstack") {
        Ok(_) => info!("Successfully loaded '.env.localstack'."),
        Err(_) => {}
    }

    let mut aws_config_builder = aws_config::defaults(BehaviorVersion::latest())
        .load()
        .await
        .into_builder();

    if let Ok(endpoint_url) = env::var("AWS_ENDPOINT_URL") {
        aws_config_builder.set_endpoint_url(Some(endpoint_url.clone()));
        info!("Using environments custom AWS_ENDPOINT_URL '{endpoint_url}'");
    }

    let ddb_client = &Client::new(&aws_config_builder.build());

    info!("Lambda cold start completed, client initialized.");

    run(service_fn(move |event: LambdaEvent<SqsEvent>| async move {
        function_handler(ddb_client, event).await
    }))
    .await
}
