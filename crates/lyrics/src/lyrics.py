from lyrics_extractor import SongLyrics
import json
import sys

arguments = " ".join(sys.argv[1:])

if len(arguments) == 0:
    print("{}")
    exit()

extract_lyrics = SongLyrics("AIzaSyBlgJL1dIkayICRTTjgsXi3DfF5G3PS06A", "c5501e5b7d1939ed0")
print(json.dumps(extract_lyrics.get_lyrics(arguments)))