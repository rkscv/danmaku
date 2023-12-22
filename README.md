[mpv](https://mpv.io) danmaku plugin powered by [dandanplay API](https://api.dandanplay.net/swagger/ui/index).

## Install

Install libmpv. Run:

```bash
go build -buildmode=c-shared -ldflags='-w -s' -trimpath
```

Append .dll/.so to the output file name. Copy the .dll/.so file to the `scripts` subdirectory of your mpv configuration directory.

## Usage

Example to bind the `d` key to toggle the danmaku visibility in your `input.conf`:

```
d script-message toggle-danmaku
```
