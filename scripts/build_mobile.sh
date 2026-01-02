#!/bin/bash

set -e # Stop the script at the first command that errors
trap "echo; exit" INT # Actually exit the script when you hit CTRL+C

scripts_dir=$(dirname "$0")
if [[ "$scripts_dir" == "" ]] then
  exit 1
fi
source "$scripts_dir/shared.sh"

parse_cargo_basics "$@"
parse_mobile_target "$@"

print_exec "dx $cargo_action -p mobile --$dx_target_os --target $cargo_target_triple $cargo_profile"

# If you're not building, your adventure stops here
if [[ $cargo_action != "build" ]] then
  exit 0
fi

### FIXING APP ICON FOR ANDROID
# https://github.com/DioxusLabs/dioxus/issues/3685
if [[ $dx_target_os == "android" ]] then
  echo -e "\nFixing app icon for android"
  gradle_project_path="$cargo_root/target/dx/Cybermap/$cargo_profile_name/android/app"

  print_exec "find $gradle_project_path/app/src/main/res/mipmap-* -name *.webp -type f -delete"

  print_exec "cp -r $cargo_root/assets/res/* $gradle_project_path/app/src/main/res/"

  print_exec "rm $gradle_project_path/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml"
  
  print_exec "cd $gradle_project_path"

  print_exec_s "./gradlew clean"
  print_exec_s "./gradlew assembleDebug"

  print_exec "cd -"

  # Cleanup the mess I made so the next dioxus build does not get confused by it
  print_exec "find $gradle_project_path/app/src/main/res/mipmap-* -name *.png -type f -delete"
fi

dioxus_output_path="$cargo_root/target/dx/Cybermap/$cargo_profile_name/android/app/app/build/outputs/apk/debug"

mobile_build_dir="$cybermap_build_root/mobile"

if [[ -d $mobile_build_dir ]] then
  echo -e  "\nRemoving old output dir"
  print_exec "rm -r $mobile_build_dir/*"
fi

echo -e "\nCreating output dir"
print_exec "mkdir -p $mobile_build_dir"

echo -e "\nCopying dioxus output to cybermap build dir"

print_exec "cp -r $dioxus_output_path/app-debug.apk $mobile_build_dir/cybermap-$cargo_target_triple.apk"

echo -e "\n\e[32mCybermap mobile app for $dx_target_os has been successfully built\nOutput directory:\e[0m $mobile_build_dir"
