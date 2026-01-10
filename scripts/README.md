# Scripts directory

Everything here should be ran from the project's root directory

## build_*.sh

Scripts to build the different packages of Cybermap

- build_server.sh  
  Builds the server

  Outputs a ready to go server binary in `target/cybermap/server/` along with the necessary directories

- build_web.sh  
  Builds the dioxus wasm front end

  Outputs a ready to go web wasm front end in `target/cybermap/web/`

- build_mobile.sh  
  Builds the dioxus front end for mobile platforms

  Outputs a ready to go APK in `target/cybermap/mobile/`, they are renamed with their target triple so you can build for ARM and AMD and keep both

Command line args are:
- (all) `c` | `check`  
  Only checks the code (cargo check) instead of building it  
  Also disable anything build related in the scripts

- (server, mobile) `r` | `release` | `--release`  
  Enable release mode

- (mobile) `arm64` | `aarch64`  
  Default  
  Target ARM cpus  
  
- (mobile) `amd` | `amd64`  
  Target AMD cpus  

- (mobile) `android`  
  Default  
  Target Android devices  

- (mobile) `apple` | `ios`  
  ⚠️ NOT SUPPORTED YET

  Target IOS devices

## shared.sh
Shared functions used by the build scripts

## dependency_checker.py
https://github.com/bowarc/rust_dependency_checker

Python script to check for dependency mistakes in cargo workspaces

## container_build.sh

Builds a ready-to-deploy docker / podman container of the project
