#!/usr/bin/env python3

import sys
import socket
import queue
import threading
import time


def tx_thread(c1: queue.Queue):

    line = "INITIAL"
    while len(line) > 0:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM, socket.IPPROTO_TCP)
        s.connect(("localhost", 9000))

        line = ch.get()
        s.sendall(line.encode("utf-8"))

        time.sleep(0.15)
        
        s.close()

if __name__ == "__main__":
    ch = queue.Queue(maxsize=0)
    

    t = threading.Thread(target=tx_thread, args=(ch, ))
    t.start()
    
    line = "INITIAL"
    
    while len(line) > 0:
        line = sys.stdin.readline()
        if len(line) > 0:
            ch.put("{}\n".format(line.strip()))

    t.join()     
