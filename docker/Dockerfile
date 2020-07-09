FROM amazonlinux:1

RUN yum install -y https://dl.fedoraproject.org/pub/epel/epel-release-latest-6.noarch.rpm
RUN yum --enablerepo=epel install -y ocl-icd ocl-icd-devel opencl-headers zip unzip shadow-utils gcc openssl-devel elfutils-libelf-devel libdwarf-devel binutils-devel zlib-devel make perl

RUN groupadd --gid 1000 commp && \
  useradd --gid 1000 --uid 1000 --create-home --shell /bin/bash commp

RUN yum install sudo -y \
    && echo commp ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/commp\
    && chmod 0440 /etc/sudoers.d/commp

USER commp

# Install version 1.43.1 of the toolchain since the latest resulted in compile fails in OpenSSL
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain 1.43.1 -y

ENV PATH /usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/home/commp/.cargo/bin

VOLUME /home/commp/build

WORKDIR /home/commp

USER commp:commp
