# rust-fil-commp-generate

This repository contains code to generate a Filecoin Commp for a file as part of the Filecoin DumboDrop project.  The DumboDrop
project runs in AWS which utilizes Lambda serverless functions for processing logic.  This leads to the following constraints
that the design of this project take into account:

1. Input files of maximum size 1GB
2. AWS Lambda Limits (Maximum RAM of 3008MB and 512MB of Disk)
3. Deployment size < 50 MB
4. High performance (to keep costs low)

The design takes these requirements into account by utilizing the Rust implementation of Filecoin Commp generation configured for stream parsing
of input data and an in memory backing store.  The Filecoin Rust implementation is an ideal match given its high performance, stream parsing
and in memory backing store.  The use of an in memory backing store does limit the size of files that can be processed using this project
to those that fit in available memory.

A Filecoin Commp (Piece Commitment) is a Merkle hash of a block of data that's (a) padded out to the next nearest base2 size and internally padded according to the storage proofs requirements (currently an extra 2 bits per 254 bits).  Commp is a 32 byte value pacakged in a CID with a custom
multi-format.  

## Dependencies

* Linux 64 bit (may work on other platforms but has not been tested) 
* Rust 1.43.1 (and probably later...)
* OpenCL libraries (apt-get -y install ocl-icd-opencl-dev)

This project supports containerized development using Visual Studio Code
Remote Containers extension.  You can read more about this here:
https://code.visualstudio.com/docs/remote/containers

## Building Local Version

> make commp_local

Builds a binary in `target/release/commp` that can be run against a file. There is a sample car file in tests/fixtures you can try it on:

> target/release/commp tests/fixtures/bafyreidigczbx3d3fbpabihjh3lmeoppdlriaipuityslbl4kgaud6bkci.car

### Building AWS Lambda Version

To run this in AWS Lambda, a custom runtime must be which requires building it to run inside the amazonlinux:1
docker image.  This docker image is based on CentOS 6 which is quite old (released in 2011).  This process
is automated by building the binary inside a docker image based on amazonlinux.  To build this build image:

> make docker_image

Once this image is built, execute a build of commp inside it:

> make commp_lambda

Which will result in the file commp_lambda.zip in the root directory.

### Deploying the Commp Lambda

1. Log into AWS Console
2. Navigate to Lambda Service -> Functions
3. Click "Create Function" button
4. Give the function a name "e.g. CommP"
5. For "Runtime" choose "Custom Runtime/Provide your own bootstrap"
6. Click "Create Function"
7. Under "Basic settings" click "Edit"
8. Set "Handler" to 'commp_lambda'
9. Set memory to max (3008 MB)
10. Set timeout to max (15 min)
11. Choose an execution role (or create one) that grants access to the S3 bucket commp will read from
12. ClicK "Save"
13. In "Function code" click "Actions->Upload a .zip file".  Navigate to commp_lambda.zip and click OK
