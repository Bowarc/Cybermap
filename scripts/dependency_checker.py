""".
    ```
    Global dependency: Crate specified in the root Cargo.toml and imported with crate.workspace=true
    Specific dependency: Crate imported using crate = "x.x.x" (or crate = {version ="x.x.x"})
    ```
    
    1. Every global dependency used by at least GLOBAL_DEP_THRESHOLD(default 2) package(s)
    2. No global dependency is imported as specific dependency
    3. Every dependency used by 2 or more packages is a global dependency  
           (no double import which could be bad, I.E different versions)
    4. Every dependency of every package is used (no unused imports)
    
    Requirements:  
    - rg (https://github.com/BurntSushi/ripgrep)  
    - a terminal that accepts ANSI codes  

    Author: Bowarc
"""


import os
import subprocess
from typing import List, Tuple


############
## Config ##
############
GLOBAL_DEP_THRESHOLD: int = 2  # How many times a global dependency has to be used

RED: str = "\033[0;31m"
GREEN: str = "\033[0;32m"
YELLOW: str = "\033[0;33m"
RESET: str = "\033[0m"


class Dependencies:
    def __init__(self, path: str) -> None:
        self.path: str = path
        self.specifics: List[str] = []
        self.globals: List[str] = []

        self.fetch()

    def __str__(self) -> str:
        return self.name()

    def name(self) -> str:
        return "root" if self.path == "." else self.path

    def add_specific(self, dep_name: str) -> None:
        self.specifics.append(dep_name)

    def add_global(self, dep_name: str) -> None:
        self.globals.append(dep_name)

    def get_specifics(self) -> List[str]:
        return self.specifics

    def get_globals(self) -> List[str]:
        return self.globals

    def get_all(self) -> List[str]:
        return self.specifics + self.globals

    def fetch(self) -> None:
        cargo_toml: str = os.path.join(self.path, "Cargo.toml")
        try:
            with open(cargo_toml, "r", encoding="utf-8") as f:
                found_dependencies: bool = False

                for line in f:
                    line: str = line.replace(" ", "").replace("\n", "")
                    if not line:
                        continue

                    if line.startswith("["):
                        found_dependencies = line.endswith("dependencies]")
                        continue

                    if line.startswith("#") or not found_dependencies:
                        continue

                    if "=" not in line:
                        continue

                    raw_dep_name: str = line.split(".")[0].split("=")[0]
                    if "workspace=true" in line:
                        self.add_global(raw_dep_name)
                    else:
                        self.add_specific(raw_dep_name)
        except FileNotFoundError:
            print(f"File {cargo_toml} not found.")
        except Exception as e:
            print(f"Error reading {cargo_toml}: {e}")


def find_packages() -> List[str]:
    packages: List[str] = []
    for item in os.listdir(".") + ["."]:
        if not os.path.isdir(os.path.join(".", item)):
            continue

        if any(
            inner.lower() == "cargo.toml"
            for inner in os.listdir(item)
            if os.path.isfile(os.path.join(item, inner))
        ):
            packages.append(item)
    return packages


def rule1(package_dependencies: List[Dependencies]) -> None:
    # Checks for unused global dependency
    global_deps: List[str] = []
    specific_deps: List[str] = []
    for package in package_dependencies:
        if package.path == ".":
            global_deps += package.get_specifics()
        else:
            specific_deps += package.get_globals()

    unused: List[str] = [
        gdep for gdep in global_deps if specific_deps.count(gdep) < GLOBAL_DEP_THRESHOLD
    ]

    if unused:
        print(
            f"{YELLOW}(Rule 1){RESET} The global dependencies {unused} are used less than {GLOBAL_DEP_THRESHOLD} package{'s'if GLOBAL_DEP_THRESHOLD > 1 else ''}".replace(
                "'", ""
            )
        )
    else:
        print(
            f"{GREEN}(Rule 1){RESET} Every global dependency is used at least {GLOBAL_DEP_THRESHOLD} time{'s'if GLOBAL_DEP_THRESHOLD > 1 else ''}"
        )


def rule2(package_dependencies: List[Dependencies]) -> None:
    # Global dependency imported as specific
    global_deps = None

    good: bool = True

    try:
        global_deps = [
            package_dep
            for package_dep in package_dependencies
            if package_dep.path == "."
        ][0]
    except Exception:
        print(f"{RED}(Rule 2){RESET} No global package found")
        return

    for dependency in global_deps.get_specifics():
        # package.path check before to dodge list comparison on root
        for package in [
            package
            for package in package_dependencies
            if package.path != "." and dependency in package.get_specifics()
        ]:
            print(
                f"{YELLOW}(Rule 2){RESET} Global dependency {dependency} is imported as specific in {package}"
            )
            # I wonder if reads are faster than writes (is it worth to check if !good before writing ?)
            good = False
    if good:
        print(f"{GREEN}(Rule 2){RESET} No global dependency is imported as specific")


def rule3(package_dependencies: List[Dependencies]) -> None:
    seen_dependencies: dict = {}
    good = True

    for package in package_dependencies:
        for dependency in package.get_specifics():
            if dependency in seen_dependencies:
                seen_packages = seen_dependencies[dependency]
                seen_packages.append(package.name())
                print(
                    f"{YELLOW}(Rule 3){RESET} Packages {seen_packages[0]} and {package.name()} both use {dependency}"
                )
                good = False
            else:
                seen_dependencies[dependency] = [package.name()]

    if good:
        print(f"{GREEN}(Rule 3){RESET} Every specific is justified")


def rule4(package_dependencies: List[Dependencies]) -> None:
    # Unused imports
    processes: dict = {}
    good: bool = True

    for package in package_dependencies:
        if package.path == ".":
            continue

        results: List[Tuple[str, subprocess.Popen]] = []
        for dep in package.get_all():
            r_dep: str = dep.replace("-", "_")

            pattern: str = f"{r_dep}::|use {r_dep}|extern crate {r_dep}"

            process: subprocess.Popen = subprocess.Popen(
                # removed os.path.join(package.path, "src") to fix build.rs not being checked
                ["rg", pattern, f"{package.path}/"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            results.append((dep, process))

        processes[package.name()] = results

    for package, results in processes.items():
        tag: bool = True
        # iter over results, build a list of dep which processes returned nothing (rg found no trace of the dep name in the code)
        for dep in {dep for dep, process in results if process.communicate()[0] == b""}:
            if tag:
                print()
                tag = False
            print(f"{YELLOW}(Rule 4){RESET} Unused dependency in {package}: {dep}")
            good = False

    if good:
        print(f"\n{GREEN}(Rule 4){RESET} No unused dependencies")


def main() -> None:

    # Get all packages
    packages: List[str] = find_packages()
    print(packages, "\n")

    # Find all their dependencies
    package_dependencies: List[Dependencies] = [
        Dependencies(package) for package in packages
    ]

    # Make sure that every global dep is used by a package
    rule1(package_dependencies)
    print()

    # Make sure that no global is also imported as specific
    rule2(package_dependencies)
    print()

    # Verify that no package share specifc dependencies
    rule3(package_dependencies)

    # Check if all imports are used
    rule4(package_dependencies)


if __name__ == "__main__":
    main()
