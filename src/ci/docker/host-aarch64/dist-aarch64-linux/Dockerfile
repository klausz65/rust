FROM ubuntu:22.04

WORKDIR /build

RUN apt-get update && \
	apt-get install -y pkg-config curl xz-utils wget unzip git make patch perl lbzip2 gcc g++ libc6-dev cmake python3 doxygen graphviz zstd libzstd-dev openssl libssl-dev

ENV PATH=/rustroot/bin:$PATH

WORKDIR /tmp
COPY host-aarch64/dist-aarch64-linux/shared.sh /tmp/

ENV CC=gcc CXX=g++

# Build LLVM+Clang
COPY host-aarch64/dist-aarch64-linux/build-clang.sh /tmp/
RUN ./build-clang.sh
ENV CC=clang CXX=clang++

COPY scripts/sccache.sh /scripts/
RUN sh /scripts/sccache.sh

ENV PGO_HOST=aarch64-unknown-linux-gnu
ENV HOSTS=aarch64-unknown-linux-gnu

ENV CPATH=/usr/include/aarch64-linux-gnu/:$CPATH

ENV RUST_CONFIGURE_ARGS \
      --build=aarch64-unknown-linux-gnu \
      --enable-full-tools \
      --enable-profiler \
      --enable-sanitizers \
      --enable-compiler-docs \
      --set target.aarch64-unknown-linux-gnu.linker=clang \
      --set target.aarch64-unknown-linux-gnu.ar=/rustroot/bin/llvm-ar \
      --set target.aarch64-unknown-linux-gnu.ranlib=/rustroot/bin/llvm-ranlib \
      --set llvm.link-shared=true \
      --set llvm.thin-lto=false \
      --set llvm.libzstd=true \
      --set llvm.ninja=false \
      --set rust.debug-assertions=false \
      --set rust.jemalloc \
      --set rust.use-lld=true \
      --set rust.codegen-units=1

ENV SCRIPT python3 ../x.py dist --host $HOSTS --target $HOSTS

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=clang
ENV DIST_SRC 1
ENV DIST_REQUIRE_ALL_TOOLS 1
