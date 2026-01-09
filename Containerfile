##########
#  BASE  #
##########
FROM rust:1.91 AS base

RUN rustup target add wasm32-unknown-unknown
RUN cargo install --locked wasm-bindgen-cli --version 0.2.106
# RUN cargo install sccache
RUN cargo install --git https://github.com/wilsonzlin/minify-html minhtml --rev 2301223773dadce30a33b4c407d8b2524adeb5e2
RUN cargo install --git https://github.com/bowarc/cargo-chef
RUN cargo install --git https://github.com/DioxusLabs/dioxus dioxus-cli --locked --version 0.7.2

RUN apt update && apt upgrade -y
RUN DEBIAN_FRONTEND=noninteractive apt install libwayland-dev libgtk-3-dev libjavascriptcoregtk-4.1-dev librust-soup3-sys-dev libwebkit2gtk-4.1-dev -y

##########
# PANNER #
##########
FROM base AS planner

WORKDIR /app

# Move the essentials
COPY ./Cargo.toml ./Cargo.lock .
COPY ./server ./server
COPY ./web ./web
COPY ./ui ./ui
COPY ./osm ./osm
COPY ./mobile ./mobile

# Prepare all dependencies
RUN cargo chef prepare --recipe-path recipe.json

###########
# BUILDER #
###########
FROM base AS builder

WORKDIR /app

# Take the recipe only from tyhe planner
COPY --from=planner /app/recipe.json recipe.json

# Set up the project's build artefacts
RUN cargo chef cook --release --recipe-path recipe.json
RUN cargo chef cook -p web --release --target=wasm32-unknown-unknown --recipe-path recipe.json

# Pull the projects code
COPY ./scripts/build_web.sh ./scripts/build_mobile.sh ./scripts/build_server.sh ./scripts/shared.sh ./scripts/
COPY ./Cargo.toml ./Cargo.lock .
COPY ./server ./server
COPY ./web ./web
COPY ./ui ./ui
COPY ./osm ./osm
COPY ./mobile ./mobile

# Build it
RUN bash ./scripts/build_web.sh r
RUN bash ./scripts/build_server.sh r

## TODO: Build mobile apps

############
# MINIFIER #
############
FROM base AS minifier

WORKDIR /app

COPY --from=builder /app/target/cybermap/web/ ./static/

RUN minhtml --minify-doctype ./static/index.html --output ./static/index.html

##########
# RUNNER #
##########
FROM ubuntu:24.04 AS runner

WORKDIR /app

RUN apt update && apt upgrade -y
RUN apt install -y --no-install-recommends ca-certificates
RUN update-ca-certificates

COPY --from=builder /app/target/cybermap/server/cybermap_server .
COPY --from=minifier /app/static/ ./static/

# RUN mkdir ./log
RUN mkdir ./cache

EXPOSE 42061

CMD ["./cybermap_server"]
