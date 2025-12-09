#!/bin/bash

set -e # Stop the script at the first command that errors
trap "echo; exit" INT # Actually exit the script when you hit CTRL+C

scripts_dir=$(dirname "$0")
if [[ "$scripts_dir" == "" ]] then
  exit 1
fi
source "$scripts_dir/shared.sh"

parse_cargo_basics "$@"

print_exec "dx $cargo_action -p web $cargo_profile"

# If you're not building, your adventure stops here
if [[ $cargo_action != "build" ]] then
  exit 0
fi

echo

dioxus_output_path="$cargo_root/target/dx/web/$cargo_profile_name/web/public"

web_build_dir="$cybermap_build_root/web"

if [[ -d $web_build_dir ]] then
  echo -e  "Removing old output dir ($web_build_dir)"
  print_exec "rm -r $web_build_dir"
  echo 
fi

echo -e "Creating output dir ($web_build_dir)"
print_exec "mkdir -p $web_build_dir"
echo 

echo -e "Copying dioxus output to cybermap build dir"

print_exec "cp -r $dioxus_output_path/* $web_build_dir/"
# print_exec "cp -r $cargo_root/web/static/* $web_build_dir/" # This overwrides the index.html

echo -e "\n\e[32mCybermap web app has been successfully built\nOutput directory:\e[0m $web_build_dir"
