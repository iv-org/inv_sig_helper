#!/usr/bin/env python
# -*- coding: utf-8 -*-

import yt_dlp.YoutubeDL
import yt_dlp.extractor.youtube
import sys

"""
Args:
    - sys.argv[1]: player_url
    - sys.argv[2]: signature
    - sys.argv[3]: random youtube video id
Example:
    python yt-dlp_nsig_decoder.py "https://www.youtube.com/s/player/af7f576f/player_ias.vflset/en_US/base.js" "W78n255zM6g" "W78n255zM6g"
"""

params = {}

ie = yt_dlp.extractor.YoutubeIE()
ydl = yt_dlp.YoutubeDL({})
ydl.add_info_extractor(ie)

player_url = sys.argv[1]
signature = sys.argv[2]
youtube_video_id = sys.argv[3]

print(ie._decrypt_nsig(signature, youtube_video_id, player_url), end='')
