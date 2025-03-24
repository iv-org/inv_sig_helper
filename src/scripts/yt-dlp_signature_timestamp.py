#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import yt_dlp.YoutubeDL
import yt_dlp.extractor.youtube
import sys
from common import HiddenPrints

"""
Args:
    - sys.argv[1]: player_url
    - sys.argv[2]: random youtube video id
Example:
    python yt-dlp_signature_timestamp.py "https://www.youtube.com/s/player/af7f576f/player_ias.vflset/en_US/base.js" "W78n255zM6g"
"""

with HiddenPrints():
    params = {}

    ie = yt_dlp.extractor.YoutubeIE()
    ydl = yt_dlp.YoutubeDL({})
    ydl.add_info_extractor(ie)

    player_url = sys.argv[1]
    youtube_video_id = sys.argv[2]

    output = ie._extract_signature_timestamp(youtube_video_id, player_url)

print(output, end='')
