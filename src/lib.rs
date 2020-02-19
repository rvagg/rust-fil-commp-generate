use std::cmp;
use std::convert::TryFrom;
use std::io;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};

use filecoin_proofs::constants::DefaultPieceHasher;
use filecoin_proofs::fr32::write_padded;
use filecoin_proofs::{
    generate_piece_commitment, PaddedBytesAmount, SectorSize, UnpaddedBytesAmount,
};
use storage_proofs::fr32::Fr32Ary;
use storage_proofs::hasher::{Domain, Hasher};
use storage_proofs::pieces::generate_piece_commitment_bytes_from_source;
use storage_proofs::util::NODE_SIZE;

use generic_array::typenum;
use log::info;

use anyhow::{Context, Result};

mod multistore;

type VecStore<E> = merkletree::store::VecStore<E>;
pub type MerkleTree<T, A> = merkletree::merkle::MerkleTree<T, A, VecStore<T>, typenum::U2>;
pub type MerkleTreeMultiStore<T, A> =
    merkletree::merkle::MerkleTree<T, A, multistore::MultiStore<T>>;

pub struct CommP {
    pub padded_size: u64,
    pub piece_size: u64,
    pub bytes: [u8; 32],
}

// use a file as an io::Reader but pad out extra length at the end with zeros up
// to the padded size
struct PadReader<R: io::Read> {
    size: usize,
    padsize: usize,
    pos: usize,
    inp: R,
}

impl<R: io::Read> io::Read for PadReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let cs = if self.pos >= self.size {
            for i in 0..buf.len() {
                buf[i] = 0;
            }
            cmp::min(self.padsize - self.pos, buf.len())
        } else {
            self.inp.read(buf)?
        };
        self.pos = self.pos + cs;
        Ok(cs)
    }
}

// logic partly copied from Lotus' PadReader which is also in go-fil-markets
// figure out how big this piece will be when padded
fn padded_size(size: u64) -> u64 {
    let logv = 64 - size.leading_zeros();
    let sect_size = (1u64) << logv;
    let bound = u64::from(UnpaddedBytesAmount::from(SectorSize(sect_size)));
    if size <= bound {
        bound
    } else {
        u64::from(UnpaddedBytesAmount::from(SectorSize(1 << (logv + 1))))
    }
}

// reimplemented from rust-fil-proofs/src/storage_proofs/pieces.rs
fn local_generate_piece_commitment_bytes_from_source<H: Hasher, R: Sized + io::Read>(
    source: &mut R,
    padded_piece_size: usize,
) -> anyhow::Result<Fr32Ary> {
    let mut buf = [0; NODE_SIZE];
    let mut reader = BufReader::new(source);
    let parts = (padded_piece_size as f64 / NODE_SIZE as f64).ceil() as usize;

    info!("Calculating merkle ...");

    let tree = MerkleTree::<H::Domain, H::Function>::try_from_iter((0..parts).map(|_| {
        reader.read_exact(&mut buf)?;
        <H::Domain as Domain>::try_from_bytes(&buf)
    }))?;

    let mut comm_p_bytes = [0; NODE_SIZE];
    let comm_p = tree.root();
    comm_p
        .write_bytes(&mut comm_p_bytes)
        .context("borked at extracting commp bytes")?;

    info!("CommP from merkle root: {:?}", comm_p_bytes);
    Ok(comm_p_bytes)
}

// same as local_generate_piece_commitment_bytes_from_source but uses a custom MerkleTree
// caching store from multistore.rs
fn local_multistore_generate_piece_commitment_bytes_from_source<H: Hasher, R: Sized + io::Read>(
    source: &mut R,
    padded_piece_size: usize,
) -> anyhow::Result<Fr32Ary> {
    let mut buf = [0; NODE_SIZE];
    let mut reader = BufReader::new(source);
    let parts = (padded_piece_size as f64 / NODE_SIZE as f64).ceil() as usize;

    info!("Calculating merkle with multistore ...");

    let tree =
        MerkleTreeMultiStore::<H::Domain, H::Function>::try_from_iter((0..parts).map(|_| {
            reader.read_exact(&mut buf)?;
            <H::Domain as Domain>::try_from_bytes(&buf)
        }))?;

    let mut comm_p_bytes = [0; NODE_SIZE];
    let comm_p = tree.root();
    comm_p
        .write_bytes(&mut comm_p_bytes)
        .context("borked at extracting commp bytes")?;

    info!("CommP from merkle root: {:?}", comm_p_bytes);
    Ok(comm_p_bytes)
}

fn padded<R: Sized + io::Read>(inp: &mut R, size: u64) -> PadReader<&mut R> {
    let padded_size = padded_size(size);

    let pad_reader = PadReader {
        size: usize::try_from(size).unwrap(),
        padsize: usize::try_from(padded_size).unwrap(),
        pos: 0,
        inp: inp,
    };

    pad_reader
}

#[allow(dead_code)]
pub fn generate_commp_filecoin_proofs<R: Sized + io::Read>(
    inp: &mut R,
    size: u64,
) -> Result<CommP, anyhow::Error> {
    let pad_reader = padded(inp, size);
    let padded_size = pad_reader.padsize;

    let info = generate_piece_commitment(pad_reader, UnpaddedBytesAmount(padded_size as u64))
        .context("failed to generate piece commitment")?;

    Ok(CommP {
        padded_size: padded_size as u64,
        piece_size: u64::from(info.size),
        bytes: info.commitment,
    })
}

#[allow(dead_code)]
pub fn generate_commp_storage_proofs<R: Sized + io::Read>(
    inp: &mut R,
    size: u64,
) -> Result<CommP, std::io::Error> {
    let pad_reader = padded(inp, size);
    let padded_size = pad_reader.padsize;

    // Grow the vector big enough so that it doesn't grow it automatically
    let mut data = Vec::with_capacity((padded_size as u64 as f64 * 1.01) as usize);
    let mut temp_piece_file = Cursor::new(&mut data);

    // send the source through the preprocessor, writing output to temp file
    let piece_size =
        UnpaddedBytesAmount(write_padded(pad_reader, &mut temp_piece_file).unwrap() as u64);
    temp_piece_file.seek(SeekFrom::Start(0))?;
    let commitment = generate_piece_commitment_bytes_from_source::<DefaultPieceHasher>(
        &mut temp_piece_file,
        PaddedBytesAmount::from(piece_size).into(),
    );

    Ok(CommP {
        padded_size: padded_size as u64,
        piece_size: u64::from(piece_size),
        bytes: commitment.unwrap(),
    })
}

pub fn generate_commp_storage_proofs_mem<R: Sized + io::Read>(
    inp: &mut R,
    size: u64,
    multistore: bool,
) -> Result<CommP, std::io::Error> {
    let pad_reader = padded(inp, size);
    let padded_size = pad_reader.padsize;

    // Grow the vector big enough so that it doesn't grow it automatically
    let fr32_capacity = (padded_size as f64) * 1.008; // rounded up extra space for 2 in every 254 bits
    info!(
        "Padded size = {}, allocating vector with fr32 size = {}",
        padded_size, fr32_capacity
    );
    let mut data = Vec::with_capacity(fr32_capacity as usize);
    let mut temp_piece_file = Cursor::new(&mut data);

    // send the source through the preprocessor, writing output to temp file
    let piece_size =
        UnpaddedBytesAmount(write_padded(pad_reader, &mut temp_piece_file).unwrap() as u64);
    info!("Piece size = {:?}", piece_size);
    temp_piece_file.seek(SeekFrom::Start(0))?;

    let commitment = if multistore {
        local_multistore_generate_piece_commitment_bytes_from_source::<
            DefaultPieceHasher,
            Cursor<&mut Vec<u8>>,
        >(
            &mut temp_piece_file,
            PaddedBytesAmount::from(piece_size).into(),
        )
        .unwrap()
    } else {
        local_generate_piece_commitment_bytes_from_source::<
            DefaultPieceHasher,
            Cursor<&mut Vec<u8>>,
        >(
            &mut temp_piece_file,
            PaddedBytesAmount::from(piece_size).into(),
        )
        .unwrap()
    };

    Ok(CommP {
        padded_size: padded_size as u64,
        piece_size: u64::from(piece_size),
        bytes: commitment,
    })
}
