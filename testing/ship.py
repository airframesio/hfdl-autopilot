#!/usr/bin/env python3

import sys
import socket

if __name__ == "__main__":
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM, socket.IPPROTO_TCP)
    s.connect(("localhost", 9000))
    
    line = "INITIAL"
    
    while len(line) > 0:
        line = sys.stdin.readline()
        if len(line) > 0:
            line = line.strip() + "\r\n"
            s.sendall(line.encode("utf-8"))
                
    s.close()