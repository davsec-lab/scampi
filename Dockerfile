FROM ubuntu:25.04

RUN apt-get update && apt-get install -y curl git

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"

# Set working directory
WORKDIR /workspace

CMD ["bash"]