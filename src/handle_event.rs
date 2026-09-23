use crate::{
    compile::compile,
    dto::{
        errors::{StockTrekCompileAlgorithmError, internal_server_e},
        metadata::Metadata,
        request::{CompileRequest, MetadataRequest},
        sqs::SqsMessage,
    },
    prepare_code::prepare_code,
    repo::{prepare_repo, repo_dir},
    upload::upload_to_s3,
};
use std::{fs, path::Path};
use tracing::{info, warn};

pub async fn handle_event(message: SqsMessage) -> Result<(), StockTrekCompileAlgorithmError> {
    info!("Handle event for repo {}", message.repo);
    prepare_repo(&message).await?;
    let request = request_from_repo(&message)?;
    prepare_code(&request)?;
    let compile_result = compile()?;
    info!("Compile result {}", compile_result.result);
    upload_to_s3(&request.user_id, &generator_id(&message)).await?;
    Ok(())
}

fn request_from_repo(
    message: &SqsMessage,
) -> Result<CompileRequest, StockTrekCompileAlgorithmError> {
    let dir = repo_dir(&message.repo);
    let code_path = dir.join("src/algorithm/algorithm.rs");
    let code = fs::read_to_string(&code_path)
        .map_err(|e| internal_server_e("Failed to read algorithm source", e))?;
    let metadata = read_metadata(&dir)?;
    Ok(CompileRequest {
        user_id: repo_owner(message),
        code,
        metadata: metadata.map(|metadata| MetadataRequest {
            generator_id: generator_id(message),
            metadata,
        }),
    })
}

fn read_metadata(dir: &Path) -> Result<Option<Metadata>, StockTrekCompileAlgorithmError> {
    let path = dir.join("metadata.json");
    if !path.exists() {
        warn!("No metadata.json found in repository");
        return Ok(None);
    }
    let contents =
        fs::read_to_string(&path).map_err(|e| internal_server_e("Failed to read metadata", e))?;
    let metadata = serde_json::from_str(&contents)
        .map_err(|e| internal_server_e("Failed to parse metadata", e))?;
    Ok(Some(metadata))
}

fn generator_id(message: &SqsMessage) -> String {
    message.repo.clone()
}

fn repo_owner(message: &SqsMessage) -> String {
    if let Some((owner, _)) = message.repo.split_once('/')
        && !owner.is_empty()
    {
        return owner.to_string();
    }
    if let Some((_, rest)) = message.url.split_once("://") {
        let mut segments = rest.split('/');
        segments.next();
        if let Some(owner) = segments.next()
            && !owner.is_empty()
        {
            return owner.to_string();
        }
    }
    "unknown".to_string()
}
