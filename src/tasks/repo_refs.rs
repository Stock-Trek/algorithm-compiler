use crate::{dynamodb::DynamoDbDatumRef, s3::S3ObjectRef};

pub struct RepoRefs {
    pub lock_ref: DynamoDbDatumRef,
    pub repo_ref: S3ObjectRef,
}
