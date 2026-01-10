#!/bin/bash

print_exec(){
  if [[ $1 == "" ]] then
    echo "[FATAL] Print exec called without argument"
    exit 1
  fi
  echo "- Executing: $1"
  $1
}

# Silences the program's output (I see you gradle)
print_exec_s(){
  if [[ $1 == "" ]] then
    echo "[FATAL] Print exec s called without argument"
    exit 1
  fi
  echo "- Executing (s): $1"
  $1 > /dev/null
}

parse_cargo_action(){
  cargo_action="build"

  for arg in "$@"; do
    if [[ "${arg,,}" == "build" || "${arg,,}" == "b" ]]; then
      cargo_action="build"
    elif [[ "${arg,,}" == "check" || "${arg,,}" == "c" ]]; then
      cargo_action="check"
    fi
  done
}

parse_cargo_profile(){
  cargo_profile="" # Default is debug
  cargo_profile_name="debug"
  
  for arg in "$@"; do
    if [[ "${arg,,}" == "release" || "${arg,,}" == "r" || "${arg,,}" == "--release" ]]; then
      cargo_profile="--release"
      cargo_profile_name="release"
    fi
  done
}

parse_mobile_target(){
  machine="aarch64"
  vendor="linux"
  os="android"

  for arg in "$@"; do
    if [[ "${arg,,}" == "arm64" || "${arg,,}" == "aarch64" ]]; then
      machine="aarch64"
    elif [[ "${arg,,}" == "amd" || "${arg,,}" == "amd64" ]]; then
      machine="x86_64"
    elif [[ "${arg,,}" == "android" ]]; then
      vendor="linux"
      os="android"
    elif [[ "${arg,,}" == "apple" || "${arg,,}" == "ios" ]]; then
      vendor="apple"
      os="ios"
    fi
  done
  cargo_target_triple="$machine-$vendor-$os"
  dx_target_os=$os
  unset machine vendor os
}

cargo_root="$(dirname $(cargo locate-project --message-format plain))"

build_dir(){
  if [[ "$cargo_root" == "" ]] then
    echo "[FATAL] variable cargo_root is empty"
    exit 1
  fi

  cybermap_build_root="$cargo_root/target/cybermap"
  mkdir -p $cybermap_build_root
}

build_dir
