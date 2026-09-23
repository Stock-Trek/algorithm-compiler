use crate::{
    constants::{BUILT_CWASM, S3_BUCKET_UPLOADS, TMP_ALGORITHM_RS, TMP_METADATA_RS},
    dto::errors::StockTrekCompileAlgorithmError,
    s3,
};
use std::path::Path;
use tracing::info;

pub async fn upload_to_s3(
    user_id: &str,
    generator_id: &str,
) -> Result<(), StockTrekCompileAlgorithmError> {
    info!("Upload files to S3");
    let s3_key_raw_code = format!("{}/{}/algorithm.rs", user_id, generator_id);
    let s3_key_metadata = format!("{}/{}/metadata.rs", user_id, generator_id);
    let s3_key_cwasm = format!("{}/{}/binary.cwasm", user_id, generator_id);
    let s3_client = s3::s3_client().await;
    s3::upload_file(
        &s3_client,
        S3_BUCKET_UPLOADS,
        &s3_key_raw_code,
        Path::new(TMP_ALGORITHM_RS),
    )
    .await?;
    if Path::new(TMP_METADATA_RS).exists() {
        s3::upload_file(
            &s3_client,
            S3_BUCKET_UPLOADS,
            &s3_key_metadata,
            Path::new(TMP_METADATA_RS),
        )
        .await?;
    }
    s3::upload_file(
        &s3_client,
        S3_BUCKET_UPLOADS,
        &s3_key_cwasm,
        Path::new(BUILT_CWASM),
    )
    .await?;
    Ok(())
}
