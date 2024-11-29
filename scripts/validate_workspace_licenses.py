import subprocess
import json
import toml

ALLOWED_GPL_LICENSE = "GPL-3.0-or-later WITH Classpath-exception-2.0"
CLASSPATH_EXCEPTION_CRATES = {"partner-chains-node"}  # Crates explicitly allowed to have dependencies licensed with ALLOWED_GPL_LICENSE

# Clarifications for crates with unknown licenses
CLARIFICATIONS = {
    "ring": "OpenSSL AND ISC AND MIT",
    "webpki": "ISC",
    "fuchsia-cprng": "BSD-3-Clause",
    "raw-scripts": "Apache-2.0",
}

def get_workspace_crates():
    try:
        result = subprocess.run(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"],
            capture_output=True,
            text=True,
            check=True
        )
        metadata = json.loads(result.stdout)
        return [pkg["manifest_path"] for pkg in metadata["packages"]]
    except subprocess.CalledProcessError as e:
        print("Error retrieving workspace metadata:", e.stderr)
        exit(1)

def get_crate_name(crate_manifest_path):
    try:
        with open(crate_manifest_path, "r") as f:
            data = toml.load(f)
            return data["package"]["name"]
    except (FileNotFoundError, KeyError) as e:
        print(f"Error retrieving crate name from {crate_manifest_path}: {e}")
        return "UNKNOWN"

def get_crate_license(crate_manifest_path):
    try:
        with open(crate_manifest_path, "r") as f:
            data = toml.load(f)
            return data["package"]["license"]
    except FileNotFoundError:
        print(f"Manifest file not found: {crate_manifest_path}")
        return "UNKNOWN"
    except KeyError:
        print(f"License not specified: {crate_manifest_path}")
        return "UNKNOWN"

def list_licenses_for_crate_deps(crate_manifest_path):
    try:
        result = subprocess.run(
            ["cargo", "license", "--manifest-path", crate_manifest_path, "--json", "--avoid-build-deps"],
            capture_output=True,
            text=True,
            check=True
        )
        return json.loads(result.stdout)
    except subprocess.CalledProcessError as e:
        print(f"Error checking licenses for {crate_manifest_path}: {e.stderr}")
        return None

# Naive (but probably good enough) way of checking if a license is non-GPL
def is_non_gpl_license(license_str):
    return "GPL" not in license_str

def is_allowed_gpl_license(license_str):
    return license_str == ALLOWED_GPL_LICENSE

def is_valid_license_combination(license_str, crate_license, crate_name):
    licenses = [l.strip() for l in license_str.split("OR")]
    # Check if at least one license is valid
    return any(
        is_non_gpl_license(lic) or
        (is_allowed_gpl_license(lic) and
         (crate_license == ALLOWED_GPL_LICENSE or crate_name in CLASSPATH_EXCEPTION_CRATES))
        for lic in licenses
    )

def main():
    print("Fetching workspace crates...")
    workspace_crates = get_workspace_crates()
    violations = []

    for crate_manifest_path in workspace_crates:
        crate_name = get_crate_name(crate_manifest_path)
        print(f"Checking licenses for {crate_name}...")

        crate_license = get_crate_license(crate_manifest_path)
        dependencies = list_licenses_for_crate_deps(crate_manifest_path)

        if dependencies is None:
            violations.append(f"{crate_name} -> Failed to retrieve license information for dependencies.")
            continue

        for dep in dependencies:
            dep_name = dep["name"]
            dep_license = dep.get("license")

            if dep_license is None or dep_license == "UNKNOWN":
                if dep_name in CLARIFICATIONS:
                    dep_license = CLARIFICATIONS[dep_name]
                else:
                    violations.append(f"{crate_name} -> {dep_name} has no license specified and no clarification provided")
                    continue

            if not is_valid_license_combination(dep_license, crate_license, crate_name):
                violations.append(
                    f"{crate_name} -> {dep_name} ({dep_license}) is not allowed. "
                    f"Only 'non-GPL' or '{ALLOWED_GPL_LICENSE}' licenses are permitted, "
                    f"with additional restrictions for GPL licenses."
                )

    if violations:
        print("\nLicense violations detected:")
        print("\n".join(violations))
        exit(1)
    else:
        print("All licenses comply with policies.")

if __name__ == "__main__":
    main()
