// Generate a Filecoin CommP for a file
// Usage: commp <path to file> [fp]
// specify "fp" to run through filecoin_proofs

use std::cmp;
use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::io;
use std::io::{Cursor, Read, Seek, SeekFrom};

use anyhow::ensure;
use bytesize;
use hex;

use filecoin_proofs::constants::DefaultPieceHasher;
use filecoin_proofs::fr32::write_padded;
use filecoin_proofs::{
    generate_piece_commitment, PaddedBytesAmount, SectorSize, UnpaddedBytesAmount,
};
use storage_proofs::fr32::Fr32Ary;
use storage_proofs::hasher::{Domain, Hasher};
use storage_proofs::pieces::generate_piece_commitment_bytes_from_source;
use storage_proofs::util::NODE_SIZE;

type VecStore<E> = merkletree::store::VecStore<E>;
pub type MerkleTree<T, A> = merkletree::merkle::MerkleTree<T, A, VecStore<T>>;

// use a file as an io::Reader but pad out extra length at the end with zeros up
// to the padded size
struct PadReader {
    file: File,
    fsize: usize,
    padsize: usize,
    pos: usize,
}

impl io::Read for PadReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        /*
        if self.pos == self.fsize {
          print!("reached file size, now padding ...")
        }
        */
        if self.pos >= self.fsize {
            for i in 0..buf.len() {
                buf[i] = 0;
            }
            let cs = cmp::min(self.padsize - self.pos, buf.len());
            self.pos = self.pos + cs;
            /*
            if cs < buf.len() {
              print!("done with file ...")
            }
            */
            Ok(cs)
        } else {
            let cs = self.file.read(buf)?;
            self.pos = self.pos + cs;
            Ok(cs)
        }
    }
}

// logic partly copied from Lotus' PadReader which is also in go-fil-markets
// figure out how big this piece will be when padded
fn padded_size(size: u64) -> u64 {
    let logv = 64 - size.leading_zeros();
    let sect_size = (1 as u64) << logv;
    let bound = u64::from(UnpaddedBytesAmount::from(SectorSize(sect_size)));
    if size <= bound {
        return bound;
    }
    return u64::from(UnpaddedBytesAmount::from(SectorSize(1 << (logv + 1))));
}

// from rust-fil-proofs/src/storage_proofs/pieces.rs
fn local_generate_piece_commitment_bytes_from_source<H: Hasher>(
    source: &mut dyn Read,
    padded_piece_size: usize,
) -> anyhow::Result<Fr32Ary> {
    ensure!(padded_piece_size > 32, "piece is too small");
    ensure!(padded_piece_size % 32 == 0, "piece is not valid size");

    let mut buf = [0; NODE_SIZE];
    use std::io::BufReader;

    let mut reader = BufReader::new(source);

    let parts = (padded_piece_size as f64 / NODE_SIZE as f64).ceil() as usize;

    let tree = MerkleTree::<H::Domain, H::Function>::try_from_iter((0..parts).map(|_| {
        reader.read_exact(&mut buf)?;
        <H::Domain as Domain>::try_from_bytes(&buf)
    }))?;

    let mut comm_p_bytes = [0; NODE_SIZE];
    let comm_p = tree.root();
    comm_p.write_bytes(&mut comm_p_bytes)?;

    Ok(comm_p_bytes)
}

fn usage() {
    print!("Usage: commp [-fp|-sp|-spl] <file>\n");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        usage();
        return Err(From::from("Not enough arguments".to_string()));
    }
    let filename = &args[2];
    let file = File::open(filename).expect("Unable to open file");
    let file_size = file.metadata().unwrap().len();
    let padded_file_size = padded_size(file_size);
    let mut pad_reader = PadReader {
        file: file,
        fsize: usize::try_from(file_size).unwrap(),
        padsize: usize::try_from(padded_file_size).unwrap(),
        pos: 0,
    };

    let piece_size: UnpaddedBytesAmount;
    let commitment: Fr32Ary;

    if &args[1] == "-fp" {
        print!("Using filecoin_proofs method on {}\n", filename);
        let info = generate_piece_commitment(pad_reader, UnpaddedBytesAmount(padded_file_size))
            .expect("failed to generate piece commitment");
        commitment = info.commitment;
        piece_size = info.size;
    } else if &args[1] == "-sp" || &args[1] == "-spl" {
        if &args[1] == "-sp" {
            print!("Using storage_proofs method on {}\n", filename);
        } else if &args[1] == "-spl" {
            print!(
                "Using storage_proofs local (reimplemented) method on {}\n",
                filename
            );
        }
        // Grow the vector big enough so that it doesn't grow it automatically
        let mut data = Vec::with_capacity((padded_file_size as f64 * 1.01) as usize);
        let mut temp_piece_file = Cursor::new(&mut data);

        // send the source through the preprocessor, writing output to temp file
        piece_size =
            UnpaddedBytesAmount(write_padded(&mut pad_reader, &mut temp_piece_file)? as u64);
        temp_piece_file.seek(SeekFrom::Start(0))?;
        if &args[1] == "-sp" {
            commitment = generate_piece_commitment_bytes_from_source::<DefaultPieceHasher>(
                &mut temp_piece_file,
                PaddedBytesAmount::from(piece_size).into(),
            )?;
        } else {
            // -spl
            commitment = local_generate_piece_commitment_bytes_from_source::<DefaultPieceHasher>(
                &mut temp_piece_file,
                PaddedBytesAmount::from(piece_size).into(),
            )?;
        }
    } else {
        usage();
        return Err(From::from("Supply one of -fp (filecoin-proofs), -sp (storage-proofs) or -spl (storage-proofs local / reimplemented)".to_string()));
    }

    print!(
        "{}\n\tSize: {}\n\tPadded Size: {}\n\tPiece Size: {}\n\tCommP {}\n",
        filename,
        bytesize::ByteSize::b(file_size),
        bytesize::ByteSize::b(padded_file_size),
        bytesize::ByteSize::b(u64::from(piece_size)),
        hex::encode(commitment)
    );

    Ok(())
}
