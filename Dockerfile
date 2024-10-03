# Use a base image with the latest version of Rust installed
FROM rust:latest as builder

# Set the working directory in the container
WORKDIR /app

RUN apt-get update && apt-get install -y cmake build-essential libatk1.0-dev libgtk-3-dev libgdk-pixbuf-2.0-dev libpango1.0-0 libcairo2-dev libwayland-dev libpulse-dev

# Copy only the dependencies
COPY Cargo.toml Cargo.lock .

# Copy the emulator
COPY emulator emulator/.

# Copy the sdl-frontend
COPY sdl_frontend sdl_frontend/.

# Copy the self-test
COPY self_test self_test/.

# Copy ROMS
COPY resource resource/.

# install rust nightly
RUN rustup install nightly

# A dummy build to get the dependencies compiled and cached
RUN cargo +nightly build --release --bin emu6502

# (Optional) Remove debug symbols
RUN strip target/release/emu6502

# Use a slim image for running the application
FROM debian:bookworm-slim as runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends libgtk-3.0 libpango1.0-0 pulseaudio \
 && rm -rf /var/cache/debconf/* \
 && apt-get clean \
 && rm -rf /var/lib/apt/lists/*

# Copy only the compiled binary from the builder stage to this image
COPY --from=builder /app/target/release/emu6502 /bin/emu6502
COPY sample.dsk /sample.dsk

# Specify the command to run when the container starts
CMD ["/bin/emu6502","/sample.dsk"]
