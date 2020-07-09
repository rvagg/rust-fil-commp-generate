#!/bin/bash
# This script spawns a container from the built commp_lambda_build_rs
# image and runs bash interactively.  This is helpful when debugging 
# build related issues

DOCKER_TAG=commp_lambda_build_rs
THIS_DIR=rust-fil-commp-generate

docker run \
    -ti \
    --rm \
    -v $(pwd)/../:/home/commp/build \
    -v $(pwd)/docker_cache/target/:/home/commp/build/$THIS_DIR/target/ \
    -v $(pwd)/docker_cache/cargo/:/home/commp/.cargo/ \
    $DOCKER_TAG \
    /bin/bash

