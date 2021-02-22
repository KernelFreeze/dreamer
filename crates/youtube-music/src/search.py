#!/usr/bin/python

import sys
import json

from ytmusicapi import YTMusic

arguments = " ".join(sys.argv[1:])

if len(arguments) == 0:
    print("[]")
    exit()

ytmusic = YTMusic()
search_results = ytmusic.search(query=arguments, filter="songs", limit=1)

print(json.dumps(search_results))
