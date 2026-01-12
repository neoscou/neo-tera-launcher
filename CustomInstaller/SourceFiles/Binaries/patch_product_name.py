"""
Modify TERA.exe ProductName for Discord Rich Presence
Requires: pefile (pip install pefile)
"""

import pefile
import struct
import shutil
import os

EXE_PATH = r"D:\V100TERA\Neolithic Test Server\Binaries\TERA.exe"
BACKUP_PATH = r"D:\V100TERA\Neolithic Test Server\Binaries\TERA.exe.backup"
NEW_PRODUCT_NAME = "Neolithic TERA"

def modify_product_name(exe_path, new_name):
    """Modify the ProductName in the VERSION_INFO resource"""
    
    # Create backup
    if not os.path.exists(BACKUP_PATH):
        print(f"Creating backup: {BACKUP_PATH}")
        shutil.copy2(exe_path, BACKUP_PATH)
    
    print(f"Loading PE file: {exe_path}")
    pe = pefile.PE(exe_path)
    
    # Find VERSION_INFO resource
    if not hasattr(pe, 'VS_VERSIONINFO'):
        print("ERROR: No version info found in executable")
        return False
    
    # This requires manual hex editing or using Resource Hacker
    # pefile can read but not easily write version info
    print("\nNOTE: Automatic modification of VERSION_INFO is complex.")
    print("Please use Resource Hacker (free tool) to manually edit ProductName.")
    print(f"\nDownload from: http://www.angusj.com/resourcehacker/")
    print(f"\nSteps:")
    print(f"1. Open {exe_path} in Resource Hacker")
    print(f'2. Navigate to Version Info -> 1')
    print(f'3. Find "ProductName" and change its value to "{new_name}"')
    print(f"4. Click 'Compile Script' then Save")
    
    return False

if __name__ == "__main__":
    try:
        modify_product_name(EXE_PATH, NEW_PRODUCT_NAME)
    except ImportError:
        print("ERROR: pefile module not installed")
        print("Install with: pip install pefile")
    except Exception as e:
        print(f"ERROR: {e}")
