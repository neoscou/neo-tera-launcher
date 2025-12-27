#!/usr/bin/env python3
"""Analyze pcapng file to find server list data"""

try:
    from scapy.all import rdpcap, TCP, Raw
    import sys
    
    pcap_file = "wireshark_working.pcapng"
    print(f"Reading {pcap_file}...")
    
    packets = rdpcap(pcap_file)
    print(f"Total packets: {len(packets)}")
    
    # Look for HTTP traffic on port 8090
    for i, pkt in enumerate(packets):
        if TCP in pkt and Raw in pkt:
            payload = bytes(pkt[Raw].load)
            
            # Look for ServerList.json in HTTP response
            if b"ServerList.json" in payload or b'"servers"' in payload:
                print(f"\n=== Packet {i} - HTTP ServerList ===")
                print(payload.decode('utf-8', errors='ignore')[:500])
            
            # Look for protobuf-like data (starts with 0x0A which is field 1, type 2)
            if len(payload) > 200 and payload[0] == 0x0A:
                print(f"\n=== Packet {i} - Possible Protobuf (len={len(payload)}) ===")
                print("Hex:", payload[:100].hex(' '))
                
except ImportError:
    print("scapy not installed. Install with: pip install scapy")
    sys.exit(1)
except Exception as e:
    print(f"Error: {e}")
    import traceback
    traceback.print_exc()
