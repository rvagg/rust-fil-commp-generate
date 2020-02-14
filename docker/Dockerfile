FROM amazonlinux:1

RUN yum install -y https://dl.fedoraproject.org/pub/epel/epel-release-latest-6.noarch.rpm
RUN yum --enablerepo=epel install -y ocl-icd ocl-icd-devel opencl-headers zip unzip shadow-utils gcc openssl-devel elfutils-libelf-devel libdwarf-devel binutils-devel zlib-devel make perl

RUN groupadd --gid 1000 commp && \
  useradd --gid 1000 --uid 1000 --create-home --shell /bin/bash commp

USER commp

RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly -y

ENV PATH /usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/home/commp/.cargo/bin

VOLUME /home/commp/build

WORKDIR /home/commp

USER commp:commp
