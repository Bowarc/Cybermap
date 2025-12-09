#!/bin/bash

print_exec(){
  if [[ $1 == "" ]] then
    echo "Print exec called without argument"
    exit1
  fi
  echo "- Executing: $1"
  $1
}

# Silences the program's output (I see you gradle)
print_exec_s(){
  if [[ $1 == "" ]] then
    echo "Print exec called without argument"
    exit1
  fi
  echo "- Executing (s): $1"
  $1 >/dev/null
}

parse_cargo_basics(){
  cargo_profile="" # Default is debug
  cargo_profile_name="debug"
  
  cargo_action="build"

  for arg in "$@"; do
    if [[ "${arg,,}" == "release" || "${arg,,}" == "r" || "${arg,,}" == "--release" ]]; then
      cargo_profile="--release"
      cargo_profile_name="release"
    elif [[ "${arg,,}" == "build" || "${arg,,}" == "b" ]]; then
      cargo_action="build"
    elif [[ "${arg,,}" == "check" || "${arg,,}" == "c" ]]; then
      cargo_action="check"
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
    echo "cargo_root is empty"
    exit 1
  fi

  cybermap_build_root="$cargo_root/target/cybermap"
  mkdir -p $cybermap_build_root
}

# # Rename binaries with their target triple
# build_dir_mobile(){
#   if [[ "$cargo_root" == "" ]] then
#     exit 1
#   fi
#   if [[ "$1" == "" ]]
#     echo Expected one argument: Target triple
#     exit 1
#   fi

#   mobile_build_dir="$cargo_root/target/cybermap/mobile/$1"
#   mkdir -p $mobile_build_dir
#   echo $mobile_build_dir
#   unset mobile_build_dir
# }

build_dir
