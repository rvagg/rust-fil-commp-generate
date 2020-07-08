# rust-fil-commp-generate

Generate a Filecoin CommP for a file. CommP is "Piece Commitment", a Merkle hash of a block of data that's (a) padded out to the next nearest base2 size and internally padded according to the storage proofs requirements (currently an extra 2 bits per 254 bits).

This code probably isn't useful for anyone else as it is but may serve as an interesting example for Filecoin implementers or extenders.

See [src/commp.rs](src/commp.rs) for the most interesting code, which iteracts with [rust-fil-proofs](https://github.com/filecoin-project/rust-fil-proofs) to do the heavy lifting.

## Build

Requires:

* Rust 1.43.1 or later
* A checked out copy of [rust-fil-proofs](https://github.com/filecoin-project/rust-fil-proofs) at `../rust-fil-proofs/` relative to this directory.

`make commp_local`

Builds a binary in `target/release/commp` that can be run against a file. There is a sample car file in tests/fixtures you can try it on:

```shell
target/release/commp tests/fixtures/bafyreidigczbx3d3fbpabihjh3lmeoppdlriaipuityslbl4kgaud6bkci.car
```

### Lambda

`make docker_image` to make the Docker image tagged `commp_lambda_build_rs` which is suitable for building a Lambda package.

`make commp_lambda` to build a commp_lambda.zip that can be added as a custom Lambda function.
