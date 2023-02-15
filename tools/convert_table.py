#!/usr/bin/env python3

import argparse
import json
import libconf
import logging
import os


BANDWIDTH_LIMIT = 384


def OutputFileType(path: str) -> str:
    parent_path = os.path.dirname(os.path.abspath(path))
    if not os.path.isdir(parent_path):
        raise argparse.ArgumentTypeError("Output file path parent directory doesn't exist: {}".format(parent_path))
    return path


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("-v", "--verbose", action="store_true")    
    parser.add_argument("-o", "--output", type=OutputFileType, default="systable.json")
    parser.add_argument("systable", type=argparse.FileType("r"))
    args = parser.parse_args()

    logging.basicConfig(
        format="[%(asctime)s] %(message)s",
        level=logging.DEBUG if args.verbose else logging.ERROR,
    )

    config = libconf.load(args.systable)

    bands = {}
    stations = {}

    for station in config["stations"]:
        name = station["name"]

        stations[name] = {
            "id": station["id"],
            "name": name,
            "lat": station["lat"],
            "lon": station["lon"],
        }

        for freq in sorted(station["frequencies"]):
            freq = int(freq)
            for band in sorted(bands.keys()):
                if -BANDWIDTH_LIMIT <= freq - bands[band][0] <= BANDWIDTH_LIMIT:
                    bands[band] = sorted(bands[band] + [freq])
                    break
            else:
                band = int(freq / 1000.0)
                bands[band] = [freq]

    args.systable.seek(0)
            
    info = {
        "stations": stations,
        "bands": bands,
        "raw": args.systable.read(),
    }

    with open(args.output, "w") as fd:
        json.dump(info, fd)
