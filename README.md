[mpv](https://mpv.io) danmaku plugin powered by [dandanplay API](https://api.dandanplay.net/swagger/ui/index). The plugin sends the name and hash value of the currently playing file to the dandanplay server to get matching danmaku comments.

## Install

Run:

```bash
cargo build --release
```

Copy the `.dll`/`.so` file to the `scripts` subdirectory of your mpv configuration directory.

## Usage

Example to bind the `d` key to toggle the danmaku visibility in your `input.conf` (default invisible):

```
d script-message toggle-danmaku
```

It may take some time to load the danmaku after first enabling it.

Set the following options in `script-opts/danmaku.conf` to configure the plugin:

```
font_size=40
```
