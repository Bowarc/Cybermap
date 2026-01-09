#!/bin/bash

set -e # Stop the script at the first command that errors
trap "echo; exit" INT # Actually exit the script when you hit CTRL+C

scripts_dir=$(dirname "$0")
if [[ "$scripts_dir" == "" ]] then
  exit 1
fi
source "$scripts_dir/shared.sh"

parse_cargo_basics "$@"

print_exec "cargo $cargo_action -p server $cargo_profile"

# If you're not building, your adventure stops here
if [[ $cargo_action != "build" ]] then
  exit 0
fi

echo

cargo_output_path="$cargo_root/target/$cargo_profile_name"

server_build_dir="$cybermap_build_root/server"

if [[ -d $server_build_dir ]] then
  echo -e  "Removing old output dir ($server_build_dir)"
  print_exec "rm -r $server_build_dir/*"
  echo 
fi

echo -e "Creating output dir ($server_build_dir)"
print_exec "mkdir -p $server_build_dir"
echo 

echo -e "Copying cargo output to cybermap build dir"

print_exec "cp -r $cargo_output_path/cybermap_server $server_build_dir/"

echo -e "\n\e[32mCybermap server app has been successfully built\nOutput directory:\e[0m $server_build_dir"
