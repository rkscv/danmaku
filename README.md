[mpv](https://mpv.io) danmaku plugin powered by [dandanplay API](https://api.dandanplay.net/swagger/ui/index).

## Install

```bash
go build -buildmode=c-shared -ldflags='-w -s'
```

Append .dll/.so to the output file name. Copy the .dll/.so file and render.lua to your mpv config directory.

## Usage

Example to bind the `d` key to toggle the danmaku visibility:

```
d script-message toggle-danmaku
```
