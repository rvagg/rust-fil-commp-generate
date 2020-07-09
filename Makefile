CLEAN=target/release/commp target/release/bootstrap commp_lambda.zip
DOCKER_TAG=commp_lambda_build_rs
THIS_DIR=$(notdir $(CURDIR))

commp_local:
	cargo build --bin commp --release
.PHONY: commp_local

docker_image:
	cd docker && docker build -t $(DOCKER_TAG) .
	mkdir -p $(PWD)/docker_cache/cargo/
	mkdir -p $(PWD)/docker_cache/target/
	docker run \
		-ti \
		--rm \
		-v $(PWD)/docker_cache/cargo/:/home/commp/.cargo_cache/ \
		$(DOCKER_TAG) \
		bash -c "cp -a /home/commp/.cargo/bin/ /home/commp/.cargo/env /home/commp/.cargo_cache/"

commp_lambda:
	mkdir -p $(PWD)/docker_cache/target/
	docker run \
		-ti \
		--rm \
		-v $(PWD)/../:/home/commp/build \
		-v $(PWD)/docker_cache/target/:/home/commp/build/$(THIS_DIR)/target/ \
		-v $(PWD)/docker_cache/cargo/:/home/commp/.cargo/ \
		$(DOCKER_TAG) \
		bash -c "cd build/$(THIS_DIR) && make commp_lambda_in_docker"
.PHONY: commp_lambda

commp_lambda_in_docker:
	rm -f $(CLEAN)
	CFLAGS=-std=c99 cargo build --bin bootstrap --release
	mkdir -p target/release/lib/
	cp -a /usr/lib64/libOpenCL* target/release/lib/
	cd target/release/ &&	zip ../../commp_lambda.zip -r bootstrap lib/
	rm -rf lib
.PHONY: commp_lambda_in_docker

clean:
	rm -rf $(CLEAN)
.PHONY: clean
