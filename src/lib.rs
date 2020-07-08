use std::cmp;
use std::convert::TryFrom;
use std::io;
use std::io::{BufReader, Read};

use filecoin_proofs::constants::DefaultPieceHasher;
use filecoin_proofs::fr32_reader::Fr32Reader;
use filecoin_proofs::{PaddedBytesAmount, SectorSize, UnpaddedBytesAmount};
use storage_proofs::fr32::Fr32Ary;
use storage_proofs::hasher::{Domain, Hasher};
use storage_proofs::util::NODE_SIZE;

use merkletree::merkle;

use generic_array::typenum;
use log::info;

use anyhow::{Context, Result};

// how filecoin_proofs uses it if we go directly through it:
//
// type DiskStore<E> = merkletree::store::DiskStore<E>;
// type DiskMerkleTree<T, A, U> = merkle::MerkleTree<T, A, DiskStore<T>, U>;
// type BinaryDiskMerkleTree<T, A> = DiskMerkleTree<T, A, typenum::U2>;

// if we want to control merklisation resource usage:

type VecStore<E> = merkletree::store::VecStore<E>;
type VecMerkleTree<T, A, U> = merkle::MerkleTree<T, A, VecStore<T>, U>;
type BinaryVecMerkleTree<T, A> = VecMerkleTree<T, A, typenum::U2>;

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

// logic partly copied from Lotus' Fr32Reader which is also in go-fil-markets
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

/*
 * generate commp storage proof for the specifified input stream.  This
 * implementation is completely in memory so may have issues with very large
 * files (it is desinged specifically for dumbo-drop car files which are 1GB).
 */
pub fn generate_commp_storage_proofs_mem<R: Sized + io::Read>(
    inp: &mut R,
    size: u64,
) -> Result<CommP, std::io::Error> {
    // zero-pad the end of the input so we'll end up with a ^2 size once fr32 padded.
    let base2_pad_reader = base2_padded(inp, size);
    let padded_size = base2_pad_reader.padsize;

    let uba = UnpaddedBytesAmount(padded_size as u64);

    let mut pad_reader = Fr32Reader::new(base2_pad_reader);

    let commitment =
        generate_piece_commitment_bytes_from_source_with_vecstore::<DefaultPieceHasher>(
            &mut pad_reader,
            PaddedBytesAmount::from(uba).into(),
        )
        .unwrap();

    Ok(CommP {
        padded_size: padded_size as u64,
        piece_size: piece_size(size, false),
        bytes: commitment,
    })
}

#[cfg(test)]
mod tests {

    use super::generate_commp_storage_proofs_mem;
    use std::fs::File;

    #[test]
    fn generate_succeeds() {
        let mut file =
            File::open("../bafyreidigczbx3d3fbpabihjh3lmeoppdlriaipuityslbl4kgaud6bkci.car")
                .unwrap();
        let file_size = file.metadata().unwrap().len();
        let commp = generate_commp_storage_proofs_mem(&mut file, file_size).unwrap();
        assert_eq!(commp.padded_size, 2080768);
        assert_eq!(commp.piece_size, 2097152);
        assert_eq!(commp.bytes.len(), 32);
        assert_eq!(
            commp.bytes,
            [
                222, 177, 39, 25, 67, 163, 107, 5, 68, 132, 31, 203, 228, 182, 208, 97, 57, 177,
                182, 227, 179, 213, 117, 201, 82, 197, 192, 106, 191, 85, 6, 9
            ]
        );
    }
}
