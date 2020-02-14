// Generate a Filecoin CommP for a file
// Usage: commp <path to file> [fp]
// specify "fp" to run through filecoin_proofs

mod commp;

use std::env;
use std::fs::File;

#[macro_use]
extern crate log;

use bytesize;
use hex;
extern crate simple_logger;

fn usage() {
    print!("Usage: commp [-fp|-sp|-spl] <file>\n");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info)?;

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        usage();
        return Err(From::from("Not enough arguments".to_string()));
    }
    let filename = &args[2];
    let mut file = File::open(filename).expect("Unable to open file");
    let file_size = file.metadata().unwrap().len();

    let commp: commp::CommP;

    if &args[1] == "-fp" {
        commp = commp::generate_commp_filecoin_proofs(&mut file, file_size).unwrap();
    } else if &args[1] == "-sp" {
        commp = commp::generate_commp_storage_proofs(&mut file, file_size).unwrap();
    } else if &args[1] == "-spl" {
        commp = commp::generate_commp_storage_proofs_mem(&mut file, file_size, false).unwrap();
    } else if &args[1] == "-splm" {
        commp = commp::generate_commp_storage_proofs_mem(&mut file, file_size, true).unwrap();
    } else {
        usage();
        return Err(From::from("Supply one of -fp (filecoin-proofs), -sp (storage-proofs) or -spl (storage-proofs local / reimplemented)".to_string()));
    }

    print!(
        "{}\n\tSize: {}\n\tPadded Size: {}\n\tPiece Size: {}\n\tCommP {}\n",
        filename,
        bytesize::ByteSize::b(file_size),
        bytesize::ByteSize::b(commp.padded_size),
        bytesize::ByteSize::b(commp.piece_size),
        hex::encode(commp.bytes)
    );

    Ok(())
}
