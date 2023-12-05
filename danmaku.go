package main

// #cgo windows CFLAGS: -DMPV_CPLUGIN_DYNAMIC_SYM
// #include "mpv/client.h"
// #include <stdlib.h>
//
// mpv_event *bridge_mpv_wait_event(mpv_handle *mpv, double timeout);
// int bridge_mpv_observe_property(mpv_handle *mpv, uint64_t reply_userdata, const char *name, mpv_format format);
// const char *bridge_mpv_error_string(int error);
// int bridge_mpv_get_property(mpv_handle *mpv, const char *name, mpv_format format, void *data);
// void bridge_mpv_free(void *data);
// int emit_danmaku(mpv_handle *mpv, const char *data);
import "C"
import (
	"bytes"
	"context"
	"crypto/md5"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"path"
	"slices"
	"strconv"
	"strings"
	"unsafe"
)

type match struct {
	IsMatched bool `json:"isMatched"`
	Matches   []struct {
		EpisodeID int `json:"episodeId"`
	} `json:"matches"`
}

type comment struct {
	Count    int `json:"count"`
	Comments []struct {
		Cid int    `json:"cid"`
		P   string `json:"p"`
		M   string `json:"m"`
	} `json:"comments"`
}

type danmaku struct {
	Message  string  `json:"message"`
	Duration float64 `json:"duration"`
	R        uint8   `json:"r"`
	G        uint8   `json:"g"`
	B        uint8   `json:"b"`
}

var ctx, cancel = context.WithCancel(context.Background())

//export mpv_open_cplugin
func mpv_open_cplugin(mpv *C.struct_mpv_handle) C.int {
	name := C.CString("path")
	err := C.bridge_mpv_observe_property(mpv, 0, name, C.MPV_FORMAT_STRING)
	C.free(unsafe.Pointer(name))
	if err < 0 {
		log.Print(C.GoString(C.bridge_mpv_error_string(err)))
		return err
	}

	for {
		event := C.bridge_mpv_wait_event(mpv, -1)
		switch event.event_id {
		case C.MPV_EVENT_SHUTDOWN:
			return 0
		case C.MPV_EVENT_FILE_LOADED:
			cancel()
			ctx, cancel = context.WithCancel(context.Background())

			if event.error < 0 {
				log.Print(C.GoString(C.bridge_mpv_error_string(event.error)))
				break
			}
			name := C.CString("path")
			var data *C.char
			err := C.bridge_mpv_get_property(mpv, name, C.MPV_FORMAT_STRING, unsafe.Pointer(&data))
			C.free(unsafe.Pointer(name))
			if err < 0 {
				log.Print(C.GoString(C.bridge_mpv_error_string(err)))
				break
			}
			go func(ctx context.Context, name string) {
				danmaku, err := comments(ctx, name)
				if err != nil {
					log.Print(err)
					return
				}
				b, err := json.Marshal(danmaku)
				if err != nil {
					panic(err)
				}
				data := C.CString(string(b))
				errno := C.emit_danmaku(mpv, data)
				C.free(unsafe.Pointer(data))
				if errno < 0 {
					log.Print(C.GoString(C.bridge_mpv_error_string(errno)))
				}
			}(ctx, C.GoString(data))
			C.bridge_mpv_free(unsafe.Pointer(data))
		}
	}
}

func comments(ctx context.Context, name string) ([]danmaku, error) {
	f, err := os.Open(name)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	h := md5.New()
	// https://api.dandanplay.net/swagger/ui/index
	if _, err = io.CopyN(h, f, 16*1024*1024); err != nil {
		return nil, err
	}
	b, err := json.Marshal(map[string]string{
		"fileName": path.Base(name),
		"fileHash": fmt.Sprintf("%x", h.Sum(nil)),
	})
	if err != nil {
		panic(err)
	}
	req, err := http.NewRequestWithContext(ctx, "POST", "https://api.dandanplay.net/api/v2/match", bytes.NewReader(b))
	if err != nil {
		panic(err)
	}
	req.Header.Add("Content-Type", "application/json")
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	d := json.NewDecoder(resp.Body)
	var match match
	if err = d.Decode(&match); err != nil {
		panic(err)
	}
	if len(match.Matches) > 1 {
		return nil, errors.New("multiple matching episodes")
	} else if !match.IsMatched {
		return nil, errors.New("no matching episode")
	}

	url := fmt.Sprintf("https://api.dandanplay.net/api/v2/comment/%d?withRelated=true", match.Matches[0].EpisodeID)
	req, err = http.NewRequestWithContext(ctx, "GET", url, nil)
	if err != nil {
		panic(err)
	}
	resp, err = http.DefaultClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	d = json.NewDecoder(resp.Body)
	var comment comment
	if err = d.Decode(&comment); err != nil {
		panic(err)
	}
	comments := make([]danmaku, comment.Count)
	for i, comment := range comment.Comments {
		p := strings.SplitN(comment.P, ",", 4)
		t, err := strconv.ParseFloat(p[0], 64)
		if err != nil {
			panic(err)
		}
		c, err := strconv.Atoi(p[2])
		if err != nil {
			panic(err)
		}
		comments[i] = danmaku{
			Message:  comment.M,
			Duration: t,
			R:        uint8(c / (256 * 256)),
			G:        uint8(c % (256 * 256) / 256),
			B:        uint8(c % 256),
		}
	}
	slices.SortFunc(comments, func(a, b danmaku) int {
		return int(a.Duration - b.Duration)
	})
	return comments, nil
}

func main() {
}
