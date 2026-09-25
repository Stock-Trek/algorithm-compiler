use crate::error::{ACError, ACResult};
use aws_sdk_s3::{
    Client as S3Client,
    types::{Delete, ObjectIdentifier},
};
use aws_smithy_types::byte_stream::ByteStream;
use std::path::Path;

#[derive(Debug)]
pub struct S3ObjectRef {
    pub bucket: String,
    pub key: String,
}

pub struct S3 {
    pub client: S3Client,
}

impl S3 {
    pub async fn upload(&self, object_ref: &S3ObjectRef, source_file_path: &Path) -> ACResult<()> {
        let byte_stream = ByteStream::from_path(source_file_path)
            .await
            .map_err(|e| ACError::ByteStream(Box::new(e)))?;
        self.put(object_ref, byte_stream).await
    }

    pub async fn upload_bytes(&self, object_ref: &S3ObjectRef, bytes: Vec<u8>) -> ACResult<()> {
        self.put(object_ref, ByteStream::from(bytes)).await
    }

    async fn put(&self, object_ref: &S3ObjectRef, body: ByteStream) -> ACResult<()> {
        self.client
            .put_object()
            .bucket(&object_ref.bucket)
            .key(&object_ref.key)
            .body(body)
            .send()
            .await
            .map_err(|e| ACError::S3PutObject(Box::new(e.into_service_error())))?;
        Ok(())
    }

    pub async fn download(
        &self,
        object_ref: &S3ObjectRef,
        sink_file_path: &Path,
    ) -> ACResult<bool> {
        let result = self
            .client
            .get_object()
            .bucket(&object_ref.bucket)
            .key(&object_ref.key)
            .send()
            .await;
        match result {
            Ok(output) => {
                let bytes = output
                    .body
                    .collect()
                    .await
                    .map_err(|e| ACError::ByteStream(Box::new(e)))?
                    .into_bytes();
                std::fs::write(sink_file_path, &bytes).map_err(ACError::FileSystem)?;
                Ok(true)
            }
            Err(error) => {
                if error
                    .as_service_error()
                    .map(|e| e.is_no_such_key())
                    .unwrap_or(false)
                {
                    Ok(false)
                } else {
                    Err(ACError::InternalServer(format!(
                        "Failed to download object {:?}",
                        object_ref
                    )))
                }
            }
        }
    }

    pub async fn delete_objects_with_prefix(&self, bucket: &str, prefix: &str) -> ACResult<()> {
        let mut paginator = self
            .client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(prefix)
            .into_paginator()
            .send();
        let mut objects_to_delete: Vec<ObjectIdentifier> = Vec::new();
        while let Some(page) = paginator.next().await {
            let page =
                page.map_err(|e| ACError::S3ListObjects(Box::new(e.into_service_error())))?;
            for obj in page.contents() {
                if let Some(key) = obj.key() {
                    objects_to_delete.push(
                        ObjectIdentifier::builder()
                            .key(key)
                            .build()
                            .map_err(ACError::Build)?,
                    );
                }
            }
        }
        if objects_to_delete.is_empty() {
            return Ok(());
        }
        for chunk in objects_to_delete.chunks(1000) {
            let delete = Delete::builder()
                .set_objects(Some(chunk.to_vec()))
                .build()
                .map_err(ACError::Build)?;
            self.client
                .delete_objects()
                .bucket(bucket)
                .delete(delete)
                .send()
                .await
                .map_err(|e| ACError::S3DeleteObjects(Box::new(e.into_service_error())))?;
        }
        Ok(())
    }
}
