// Generate a Filecoin CommP for a file
// Usage: commp <path to file>

use std::fs::File;
use std::env;
use std::cmp;
use std::io;
use std::convert::TryFrom;
extern crate filecoin_proofs;
use filecoin_proofs::{ UnpaddedBytesAmount, SectorSize, generate_piece_commitment };
extern crate hex;

// use a file as an io::Reader but pad out extra length at the end with zeros up
// to the padded size
struct PadReader {
  file: File,
  fsize: usize,
  padsize: usize,
  pos: usize
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
      let frb = self.file.read(buf);
      if !frb.is_ok() {
        return Err(frb.unwrap_err())
      }
      let cs = frb.unwrap();
      self.pos = self.pos + cs;
      Ok(cs)
    }
  }
}

// logic partly copied from Lotus' PadReader which is also in go-fil-markets
// figure out how big this piece will be when padded
fn padded_size (size: u64) -> u64 {
  let logv = 64 - size.leading_zeros();
  let sect_size = (1 as u64) << logv;
  let bound = u64::from(UnpaddedBytesAmount::from(SectorSize(sect_size)));
  if size <= bound {
    return bound
  }
  return u64::from(UnpaddedBytesAmount::from(SectorSize(1 << (logv + 1))));
}

fn main() {
  let args: Vec<String> = env::args().collect();
  let filename = &args[1];
  let file = File::open(filename).expect("Unable to open file");
  let file_size = file.metadata().unwrap().len();
  let padded_file_size = padded_size(file_size);
  let pad_reader = PadReader { file: file, fsize: usize::try_from(file_size).unwrap(), padsize: usize::try_from(padded_file_size).unwrap(), pos: 0 };
  let info = generate_piece_commitment(pad_reader, UnpaddedBytesAmount(padded_file_size))
    .expect("failed to generate piece commitment");
  
  print!("{} Size: {:?}, Padded: {:?}, CommP {}\n", args[1], info.size, padded_file_size, hex::encode(info.commitment));
}