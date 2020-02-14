mod commp;

use std::error::Error;
use std::str::FromStr;

#[macro_use]
extern crate lambda_runtime as lambda;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate simple_logger;

use lambda::error::HandlerError;

use rusoto_core::credential::{AwsCredentials, StaticProvider};
use rusoto_core::region::Region;
use rusoto_s3::{GetObjectRequest, S3Client, S3};

use hex;

#[derive(Deserialize, Clone)]
struct CommPRequest {
    region: String,
    bucket: String,
    key: String,
}

#[derive(Serialize, Clone)]
struct CommPResponse {
    region: String,
    bucket: String,
    key: String,
    commp: String,
    size: u64,
    #[serde(rename = "paddedSize")]
    padded_size: u64,
    #[serde(rename = "pieceSize")]
    piece_size: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info)?;
    lambda!(commp_handler);

    Ok(())
}

fn commp_handler(
    request: CommPRequest,
    _c: lambda::Context,
) -> Result<CommPResponse, HandlerError> {
    info!(
        "Received request: {}/{}/{}",
        request.region, request.bucket, request.key
    );

    let region = Region::from_str(request.region.as_str()).unwrap();

    let client = S3Client::new_with(
        rusoto_core::request::HttpClient::new().expect("Failed to creat HTTP client"),
        StaticProvider::from(AwsCredentials::default()),
        region,
    );

    let get_req = GetObjectRequest {
        bucket: request.bucket.to_owned(),
        key: request.key.to_owned(),
        ..Default::default()
    };

    let result = client
        .get_object(get_req)
        .sync()
        .expect("Couldn't GET object");

    let size = result.content_length.unwrap() as u64;

    info!("Got object, size = {}", size);

    let mut stream = result.body.unwrap().into_blocking_read();

    let commp = commp::generate_commp_storage_proofs_mem(&mut stream, size).unwrap();

    Ok(CommPResponse {
        region: request.region,
        bucket: request.bucket,
        key: request.key,
        commp: hex::encode(commp.bytes),
        size: size,
        padded_size: commp.padded_size,
        piece_size: commp.piece_size,
    })
}
