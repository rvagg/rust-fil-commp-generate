CLEAN=target/release/commp target/release/bootstrap commp_lambda.zip
DOCKER_TAG=commp_lambda_build_rs
THIS_DIR=$(notdir $(CURDIR))

commp_local:
	cargo build --bin commp --release
.PHONY: commp_local

docker_image:
	rm -rf docker_cache
	-docker image rm commp_lambda_build_rs
	cd docker && docker build -t $(DOCKER_TAG) .
	mkdir -p $(PWD)/docker_cache/cargo/
	docker run \
		-ti \
		--rm \
		-v $(PWD)/docker_cache/cargo/:/home/commp/.cargo_cache/ \
		$(DOCKER_TAG) \
		bash -c "cp -a /home/commp/.cargo/bin/ /home/commp/.cargo/env /home/commp/.cargo_cache/"

commp_lambda:
	mkdir -p $(PWD)/docker_cache/cargo/
	mkdir -p $(PWD)/docker_cache/target/
	mkdir -p $(PWD)/target/
	docker run \
		-ti \
		--rm \
		-v $(PWD)/../:/home/commp/build \
		-v $(PWD)/docker_cache/target/:/home/commp/build/$(THIS_DIR)/target/ \
		-v $(PWD)/docker_cache/cargo/:/home/commp/.cargo/ \
		$(DOCKER_TAG) \
		bash -c "cd build/$(THIS_DIR) && make commp_lambda_in_docker"
.PHONY: commp_lambda

# NOTE: Amazon Lambda Custom Runtimes are built using the amazonlinux:1 image which
# is based on CentOS 6.  CentOS 6 is fairly old (released in 2011?) and has out of
# date dependencies such as gcc 4.8.5.  This old version of gcc does not compile
# the C code in the neptune-triton crate using the default C standard version in 
# 4.8.5.  We can get it to compile by specifying a newer version -std=gnu11 but
# we using that for everything causes OpenSSL to fail to build.  We currently
# use a super hacky workaround where we first do a build with the default C standard
# version (which builds OpenSSL) but fails on neptune-triton and then doing a second
# build where we enable gnu11 C standard to get neptune-triton to build.
commp_lambda_in_docker:
	rm -f $(CLEAN)
	-cargo build --bin bootstrap --release # first compile, will fail on neptune-triton
	CFLAGS=-std=gnu11 cargo build --bin bootstrap --release # second build, should complete
	mkdir -p target/release/lib/
	cp -a /usr/lib64/libOpenCL* target/release/lib/
	cd target/release/ &&	zip ../../commp_lambda.zip -r bootstrap lib/
	rm -rf lib
.PHONY: commp_lambda_in_docker

clean:
	rm -rf $(CLEAN)
.PHONY: clean
