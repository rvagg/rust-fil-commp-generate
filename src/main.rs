// Generate a Filecoin CommP for a file
// Usage: commp <path to file>

use std::cmp;
use std::convert::TryFrom;
use std::env;
use std::fs::File;
use std::io;

use filecoin_proofs::constants::DefaultPieceHasher;
use filecoin_proofs::fr32::write_padded;
use filecoin_proofs::{
    generate_piece_commitment, PaddedBytesAmount, SectorSize, UnpaddedBytesAmount,
};
use hex;
use storage_proofs::pieces::generate_piece_commitment_bytes_from_source;

use std::io::{Cursor, Seek, SeekFrom};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let file = File::open(filename).expect("Unable to open file");
    let file_size = file.metadata().unwrap().len();
    let padded_file_size = padded_size(file_size);
    let mut pad_reader = PadReader {
        file: file,
        fsize: usize::try_from(file_size).unwrap(),
        padsize: usize::try_from(padded_file_size).unwrap(),
        pos: 0,
    };

    //// Old code
    //let info = generate_piece_commitment(pad_reader, UnpaddedBytesAmount(padded_file_size))
    //   .expect("failed to generate piece commitment");
    //let commitment = info.commitment;
    //let piece_size = info.size;

    let mut data = Vec::new();
    let mut temp_piece_file = Cursor::new(&mut data);
    // send the source through the preprocessor, writing output to temp file
    let piece_size =
       UnpaddedBytesAmount(write_padded(&mut pad_reader, &mut temp_piece_file)? as u64);
    temp_piece_file.seek(SeekFrom::Start(0))?;
    let commitment = generate_piece_commitment_bytes_from_source::<DefaultPieceHasher>(
       &mut temp_piece_file,
       PaddedBytesAmount::from(piece_size).into(),
    )?;

    print!(
        "{} Size: {:?}, Padded: {:?}, CommP {}\n",
        args[1],
        piece_size,
        padded_file_size,
        hex::encode(commitment)
    );

    Ok(())
}
