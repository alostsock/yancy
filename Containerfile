FROM rust:1.90-bookworm

RUN apt-get update && apt-get install -y \
    wget \
    curl \
    git \
    gnupg \
    software-properties-common \
    build-essential

WORKDIR /yancy

# Compile LibRaw to /libraw
COPY external/ external/
RUN git apply external/libraw_patch.diff && \
    cd external/LibRaw && \
    ./mkdist.sh && \
    ./configure --prefix=/libraw --disable-examples && \
    make install -lraw_r

COPY . .

RUN cargo install --path .

ENTRYPOINT ["yancy"]
