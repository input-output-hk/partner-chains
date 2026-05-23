#!/usr/bin/env python3
import os
import re
import glob
import json


def extract_host_from_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as file:
            for _, line in enumerate(file, 1):
                if 'Common labels:' in line and '"host":' in line:
                    match = re.search(r'Common labels:\s*({.*?})', line)
                    if match:
                        try:
                            json_str = match.group(1)
                            labels = json.loads(json_str)
                            host = labels.get('host')
                            if host:
                                return host
                        except json.JSONDecodeError:
                            host_match = re.search(
                                r'"host"\s*:\s*"([^"]+)"', line
                            )
                            if host_match:
                                host = host_match.group(1)
                                return host
    except Exception as e:
        print(f"Error reading file {filepath}: {e}")
    return None


def rename_log_files():
    txt_files = glob.glob("*.txt")

    if not txt_files:
        print("No .txt files found in current directory")
        return
    print(f"Found {len(txt_files)} .txt files to process")
    processed_count = 0
    error_files = []
    for txt_file in txt_files:
        print(f"\nProcessing: {txt_file}")

        if re.match(r'^[a-zA-Z0-9_-]+\.txt$', txt_file) and not txt_file.startswith('temp_'):
            base_name = os.path.splitext(txt_file)[0]
            if len(base_name) < 20:
                print(f"Skipping {txt_file} - appears to already be renamed")
                continue

        host = extract_host_from_file(txt_file)

        if host:
            new_filename = f"{host}.txt"

            if os.path.exists(new_filename) and new_filename != txt_file:
                print(f"Warning: {new_filename} already exists!")
                counter = 1
                while os.path.exists(f"{host}_{counter}.txt"):
                    counter += 1
                new_filename = f"{host}_{counter}.txt"
                print(f"Using alternative name: {new_filename}")

            if new_filename != txt_file:
                try:
                    os.rename(txt_file, new_filename)
                    print(f"Renamed '{txt_file}' -> '{new_filename}'")
                    processed_count += 1
                except Exception as e:
                    print(f"Error renaming {txt_file}: {e}")
                    error_files.append(txt_file)
            else:
                print(f"File {txt_file} already has correct name")
        else:
            print(
                f"Can't find host name - please rename {txt_file} manually"
            )
            error_files.append(txt_file)

    print("\n=== Summary ===")
    print(f"Files processed successfully: {processed_count}")
    if error_files:
        print(f"Files requiring manual attention: {len(error_files)}")
        for error_file in error_files:
            print(f"  - {error_file}")


if __name__ == "__main__":
    print("Node Log Transformer")
    print("=" * 40)
    rename_log_files()
