use std::cmp;
use std::convert::TryFrom;
use std::io;
use std::io::{BufReader, Read};

use filecoin_proofs::constants::DefaultPieceHasher;
// use filecoin_proofs::fr32::write_padded;
use filecoin_proofs::{
    generate_piece_commitment, PaddedBytesAmount, SectorSize, UnpaddedBytesAmount,
};
use storage_proofs::fr32::Fr32Ary;
// use storage_proofs::pad_reader::PadReader;
use storage_proofs::hasher::{Domain, Hasher};
// use storage_proofs::pieces::generate_piece_commitment_bytes_from_source;
use filecoin_proofs::pad_reader::PadReader;
use storage_proofs::util::NODE_SIZE;

use merkletree::merkle;

use generic_array::typenum;
use log::info;

use anyhow::{Context, Result};

mod multistore;

// type DiskStore<E> = merkletree::store::DiskStore<E>;
type VecStore<E> = merkletree::store::VecStore<E>;
type MultiStore<E> = multistore::MultiStore<E>;

// type DiskMerkleTree<T, A, U> = merkle::MerkleTree<T, A, DiskStore<T>, U>;
type VecMerkleTree<T, A, U> = merkle::MerkleTree<T, A, VecStore<T>, U>;
type MultiMerkleTree<T, A, U> = merkle::MerkleTree<T, A, MultiStore<T>, U>;

// type BinaryDiskMerkleTree<T, A> = DiskMerkleTree<T, A, typenum::U2>;
type BinaryVecMerkleTree<T, A> = VecMerkleTree<T, A, typenum::U2>;
type BinaryMultiMerkleTree<T, A> = MultiMerkleTree<T, A, typenum::U2>;

pub struct CommP {
    pub padded_size: u64,
    pub piece_size: u64,
    pub bytes: [u8; 32],
}

// use a file as an io::Reader but pad out extra length at the end with zeros up
// to the padded size
struct Base2PadReader<R: io::Read> {
    size: usize,
    padsize: usize,
    pos: usize,
    inp: R,
}

impl<R: io::Read> io::Read for Base2PadReader<R> {
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

fn piece_size(size: u64, next: bool) -> u64 {
    1u64 << (64 - size.leading_zeros() + if next { 1 } else { 0 })
}

// logic partly copied from Lotus' PadReader which is also in go-fil-markets
// figure out how big this piece will be when padded
fn padded_size(size: u64) -> u64 {
    let bound = u64::from(UnpaddedBytesAmount::from(SectorSize(piece_size(
        size, false,
    ))));
    if size <= bound {
        bound
    } else {
        u64::from(UnpaddedBytesAmount::from(SectorSize(piece_size(
            size, true,
        ))))
    }
}

// reimplemented from rust-fil-proofs/src/storage_proofs/pieces.rs
fn generate_piece_commitment_bytes_from_source_with_vecstore<H: Hasher>(
    source: &mut dyn io::Read,
    padded_piece_size: usize,
) -> anyhow::Result<Fr32Ary> {
    let mut buf = [0; NODE_SIZE];
    let mut reader = BufReader::new(source);
    let parts = (padded_piece_size as f64 / NODE_SIZE as f64).ceil() as usize;

    info!("Calculating merkle ...");

    let tree =
        BinaryVecMerkleTree::<H::Domain, H::Function>::try_from_iter((0..parts).map(|_| {
            reader.read_exact(&mut buf)?;
            <H::Domain as Domain>::try_from_bytes(&buf).context("invalid Fr element")
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
fn generate_piece_commitment_bytes_from_source_with_multistore<H: Hasher>(
    source: &mut dyn io::Read,
    padded_piece_size: usize,
) -> anyhow::Result<Fr32Ary> {
    let mut buf = [0; NODE_SIZE];
    let mut reader = BufReader::new(source);
    let parts = (padded_piece_size as f64 / NODE_SIZE as f64).ceil() as usize;

    info!("Calculating merkle with multistore ...");

    let tree =
        BinaryMultiMerkleTree::<H::Domain, H::Function>::try_from_iter((0..parts).map(|_| {
            reader.read_exact(&mut buf)?;
            <H::Domain as Domain>::try_from_bytes(&buf).context("invalid Fr element")
        }))?;

    let mut comm_p_bytes = [0; NODE_SIZE];
    let comm_p = tree.root();
    comm_p
        .write_bytes(&mut comm_p_bytes)
        .context("borked at extracting commp bytes")?;

    info!("CommP from merkle root: {:?}", comm_p_bytes);
    Ok(comm_p_bytes)
}

fn base2_padded<R: Sized + io::Read>(inp: &mut R, size: u64) -> Base2PadReader<&mut R> {
    let padded_size = padded_size(size);

    let base2_pad_reader = Base2PadReader {
        size: usize::try_from(size).unwrap(),
        padsize: usize::try_from(padded_size).unwrap(),
        pos: 0,
        inp: inp,
    };

    base2_pad_reader
}

#[allow(dead_code)]
pub fn generate_commp_filecoin_proofs<R: Sized + io::Read>(
    inp: &mut R,
    size: u64,
) -> Result<CommP, anyhow::Error> {
    let base2_pad_reader = base2_padded(inp, size);
    let padded_size = base2_pad_reader.padsize;

    let info = generate_piece_commitment(base2_pad_reader, UnpaddedBytesAmount(padded_size as u64))
        .context("failed to generate piece commitment")?;

    Ok(CommP {
        padded_size: padded_size as u64,
        piece_size: piece_size(size, false),
        bytes: info.commitment,
    })
}

/*
#[allow(dead_code)]
pub fn generate_commp_storage_proofs<R: Sized + io::Read>(
    inp: &mut R,
    size: u64,
) -> Result<CommP, std::io::Error> {
    let base2_pad_reader = base2_padded(inp, size);
    let padded_size = base2_pad_reader.padsize;

    // Grow the vector big enough so that it doesn't grow it automatically
    let mut data = Vec::with_capacity((padded_size as u64 as f64 * 1.01) as usize);
    let mut temp_piece_file = Cursor::new(&mut data);

    // send the source through the preprocessor, writing output to temp file
    let uba =
        UnpaddedBytesAmount(write_padded(base2_pad_reader, &mut temp_piece_file).unwrap() as u64);
    temp_piece_file.seek(SeekFrom::Start(0))?;
    let commitment = generate_piece_commitment_bytes_from_source::<DefaultPieceHasher>(
        &mut temp_piece_file,
        PaddedBytesAmount::from(uba).into(),
    );

    Ok(CommP {
        padded_size: padded_size as u64,
        piece_size: piece_size(size, false),
        bytes: commitment.unwrap(),
    })
}
*/

pub fn generate_commp_storage_proofs_mem<R: Sized + io::Read>(
    inp: &mut R,
    size: u64,
    multistore: bool,
) -> Result<CommP, std::io::Error> {
    let base2_pad_reader = base2_padded(inp, size);
    let padded_size = base2_pad_reader.padsize;
    let uba = UnpaddedBytesAmount(padded_size as u64);
    /*
    // Grow the vector big enough so that it doesn't grow it automatically
    let fr32_capacity = (padded_size as f64) * 1.008; // rounded up extra space for 2 in every 254 bits
    info!(
        "Padded size = {}, allocating vector with fr32 size = {}",
        padded_size, fr32_capacity
    );
    let mut data = Vec::with_capacity(fr32_capacity as usize);
    let mut temp_piece_file = Cursor::new(&mut data);

    // send the source through the preprocessor, writing output to temp file
    let uba =
        UnpaddedBytesAmount(write_padded(pad_reader, &mut temp_piece_file).unwrap() as u64);
    info!("Piece size = {:?}", uba);
    temp_piece_file.seek(SeekFrom::Start(0))?;
    */

    let mut pad_reader = PadReader::new(base2_pad_reader);

    let commitment = if multistore {
        generate_piece_commitment_bytes_from_source_with_multistore::<DefaultPieceHasher>(
            &mut pad_reader,
            PaddedBytesAmount::from(uba).into(),
        )
        .unwrap()
    } else {
        generate_piece_commitment_bytes_from_source_with_vecstore::<DefaultPieceHasher>(
            &mut pad_reader,
            PaddedBytesAmount::from(uba).into(),
        )
        .unwrap()
    };

    Ok(CommP {
        padded_size: padded_size as u64,
        piece_size: piece_size(size, false),
        bytes: commitment,
    })
}
