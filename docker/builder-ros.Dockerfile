FROM ubuntu:24.04

RUN apt update && apt upgrade -y
RUN apt install -y curl capnproto build-essential
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN cargo install cargo-watch
