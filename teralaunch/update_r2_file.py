#!/usr/bin/env python3
"""
Script to update a file in Cloudflare R2 bucket and update the hash-file.json
Requires: boto3, requests

Install dependencies:
    pip install boto3 requests

Setup R2 credentials:
    Set environment variables:
    - R2_ACCESS_KEY_ID
    - R2_SECRET_ACCESS_KEY
    - R2_ENDPOINT_URL (e.g., https://<account-id>.r2.cloudflarestorage.com)
    - R2_BUCKET_NAME
"""

import os
import sys
import json
import hashlib
import boto3
import requests
from pathlib import Path

# R2 Configuration - Load from environment variables
R2_ACCESS_KEY_ID = os.getenv('R2_ACCESS_KEY_ID')
R2_SECRET_ACCESS_KEY = os.getenv('R2_SECRET_ACCESS_KEY')
R2_ENDPOINT_URL = os.getenv('R2_ENDPOINT_URL')
R2_BUCKET_NAME = os.getenv('R2_BUCKET_NAME')

# Public URL for hash file
HASH_FILE_URL = 'https://www.neolithictera.com/TeraDirect/hash-file.json'
HASH_FILE_R2_PATH = 'tera/teraDirect/launcher/hash-file.json'


def calculate_sha256(file_path):
    """Calculate SHA-256 hash of a file"""
    sha256_hash = hashlib.sha256()
    with open(file_path, "rb") as f:
        # Read the file in chunks to handle large files
        for byte_block in iter(lambda: f.read(4096), b""):
            sha256_hash.update(byte_block)
    return sha256_hash.hexdigest().upper()


def get_file_size(file_path):
    """Get file size in bytes"""
    return os.path.getsize(file_path)


def download_hash_file(s3_client):
    """Download current hash-file.json from R2"""
    print(f"Downloading hash file from R2: {HASH_FILE_R2_PATH}")
    try:
        response = s3_client.get_object(Bucket=R2_BUCKET_NAME, Key=HASH_FILE_R2_PATH)
        hash_data = json.loads(response['Body'].read().decode('utf-8'))
        print("✓ Hash file downloaded successfully")
        return hash_data
    except Exception as e:
        print(f"✗ Error downloading hash file: {e}")
        print("Creating new hash file structure...")
        return {"files": []}


def upload_file_to_r2(s3_client, local_file_path, r2_path):
    """Upload a file to R2"""
    print(f"Uploading {local_file_path} to R2: {r2_path}")
    try:
        s3_client.upload_file(
            local_file_path,
            R2_BUCKET_NAME,
            r2_path,
            ExtraArgs={'ContentType': 'application/octet-stream'}
        )
        print(f"✓ File uploaded successfully to {r2_path}")
        return True
    except Exception as e:
        print(f"✗ Error uploading file: {e}")
        return False


def update_hash_entry(hash_data, file_path, new_hash, new_size, file_url):
    """Update or add a file entry in the hash data"""
    # Find existing entry
    file_entry = None
    for i, entry in enumerate(hash_data['files']):
        if entry['path'] == file_path:
            file_entry = i
            break
    
    new_entry = {
        "path": file_path,
        "hash": new_hash,
        "size": new_size,
        "url": file_url
    }
    
    if file_entry is not None:
        print(f"Updating existing entry for {file_path}")
        hash_data['files'][file_entry] = new_entry
    else:
        print(f"Adding new entry for {file_path}")
        hash_data['files'].append(new_entry)
    
    return hash_data


def upload_hash_file(s3_client, hash_data):
    """Upload updated hash-file.json to R2"""
    print(f"Uploading updated hash file to R2: {HASH_FILE_R2_PATH}")
    try:
        json_content = json.dumps(hash_data, indent=2)
        s3_client.put_object(
            Bucket=R2_BUCKET_NAME,
            Key=HASH_FILE_R2_PATH,
            Body=json_content.encode('utf-8'),
            ContentType='application/json'
        )
        print("✓ Hash file uploaded successfully")
        return True
    except Exception as e:
        print(f"✗ Error uploading hash file: {e}")
        return False


def main():
    if len(sys.argv) < 3:
        print("Usage: python update_r2_file.py <local_file_path> <r2_relative_path>")
        print("\nExample:")
        print("  python update_r2_file.py DataCenter_Final_EUR.dat S1Game/S1Data/DataCenter_Final_EUR.dat")
        print("\nThis will:")
        print("  1. Calculate SHA-256 hash of the local file")
        print("  2. Upload file to R2 at tera/teradirect/<r2_relative_path>")
        print("  3. Download current hash-file.json")
        print("  4. Update hash entry for the file")
        print("  5. Upload updated hash-file.json")
        sys.exit(1)
    
    local_file_path = sys.argv[1]
    r2_relative_path = sys.argv[2]
    
    # Validate file exists
    if not os.path.exists(local_file_path):
        print(f"✗ Error: File not found: {local_file_path}")
        sys.exit(1)
    
    # Validate R2 credentials
    if not all([R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, R2_ENDPOINT_URL, R2_BUCKET_NAME]):
        print("✗ Error: Missing R2 credentials in environment variables")
        print("Required: R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, R2_ENDPOINT_URL, R2_BUCKET_NAME")
        sys.exit(1)
    
    # Initialize S3 client for R2
    s3_client = boto3.client(
        's3',
        endpoint_url=R2_ENDPOINT_URL,
        aws_access_key_id=R2_ACCESS_KEY_ID,
        aws_secret_access_key=R2_SECRET_ACCESS_KEY,
        region_name='auto'
    )
    
    print("=" * 60)
    print("R2 File Update Tool")
    print("=" * 60)
    print(f"Local file: {local_file_path}")
    print(f"R2 path: tera/teradirect/{r2_relative_path}")
    print()
    
    # Step 1: Calculate hash and get size
    print("Step 1: Calculating file hash and size...")
    file_hash = calculate_sha256(local_file_path)
    file_size = get_file_size(local_file_path)
    print(f"  Hash (SHA-256): {file_hash}")
    print(f"  Size: {file_size:,} bytes ({file_size / 1024 / 1024:.2f} MB)")
    print()
    
    # Step 2: Upload file to R2
    print("Step 2: Uploading file to R2...")
    r2_full_path = f"tera/teradirect/{r2_relative_path}"
    if not upload_file_to_r2(s3_client, local_file_path, r2_full_path):
        print("✗ Failed to upload file. Aborting.")
        sys.exit(1)
    print()
    
    # Step 3: Download current hash file
    print("Step 3: Downloading current hash-file.json...")
    hash_data = download_hash_file(s3_client)
    print()
    
    # Step 4: Update hash entry
    print("Step 4: Updating hash entry...")
    file_url = f"https://www.neolithictera.com/TeraDirect/{r2_relative_path}"
    hash_data = update_hash_entry(hash_data, r2_relative_path, file_hash, file_size, file_url)
    print()
    
    # Step 5: Upload updated hash file
    print("Step 5: Uploading updated hash-file.json...")
    if not upload_hash_file(s3_client, hash_data):
        print("✗ Failed to upload hash file.")
        sys.exit(1)
    print()
    
    print("=" * 60)
    print("✓ Update completed successfully!")
    print("=" * 60)
    print(f"\nUpdated entry:")
    print(f"  Path: {r2_relative_path}")
    print(f"  Hash: {file_hash}")
    print(f"  Size: {file_size:,} bytes")
    print(f"  URL: {file_url}")
    print("\nThe launcher will now download the updated file on next check.")


if __name__ == '__main__':
    main()
