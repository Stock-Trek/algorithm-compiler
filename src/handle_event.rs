use crate::{
    compile::compile,
    dto::{
        errors::StockTrekCompileAlgorithmError,
        request::body_to_request,
        response::{HandlerResult, HttpResponse},
    },
    prepare_code::prepare_code,
    upload::upload_to_s3,
};
use lambda_runtime::{LambdaEvent, tracing::info};
use serde_json::Value;

pub async fn handle_event(
    event: LambdaEvent<Value>,
) -> Result<HttpResponse, StockTrekCompileAlgorithmError> {
    info!("handle event");
    let request = body_to_request(&event.payload)?;
    prepare_code(&request)?;
    let compile_result = compile()?;
    if let Some(m) = &request.metadata {
        upload_to_s3(&request.user_id, &m.generator_id).await?;
    }
    let saved_generator_id = request.metadata.map(|m| m.generator_id);
    Ok(HttpResponse {
        status_code: 200,
        body: HandlerResult {
            compile_result,
            saved_generator_id,
        },
    })
}
