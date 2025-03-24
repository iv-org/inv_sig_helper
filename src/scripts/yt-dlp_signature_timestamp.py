#!/usr/bin/env python
# -*- coding: utf-8 -*-

import yt_dlp.YoutubeDL
import yt_dlp.extractor.youtube
import sys

"""
Args:
    - sys.argv[1]: player_url
    - sys.argv[2]: random youtube video id
Example:
    python yt-dlp_sig_decoder.py "https://www.youtube.com/s/player/af7f576f/player_ias.vflset/en_US/base.js" "W78n255zM6g"
"""


params = {}

ie = yt_dlp.extractor.YoutubeIE()
ydl = yt_dlp.YoutubeDL({})
ydl.add_info_extractor(ie)

player_url = sys.argv[1]
youtube_video_id = sys.argv[2]

print(ie._extract_signature_timestamp(youtube_video_id, player_url))
