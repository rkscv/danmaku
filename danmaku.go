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
// int osd_overlay(mpv_handle *mpv, char *data, int64_t w, int64_t h);
// int remove_overlay(mpv_handle *mpv);
// int show_text(mpv_handle *mpv, const char *text);
import "C"
import (
	"context"
	"fmt"
	"log"
	"math"
	"strings"
	"sync/atomic"
	"unsafe"
)

type danmaku struct {
	Message              string
	GraphemeClusterCount int
	Time                 float64
	R, G, B              uint8
	X                    float64
	Y                    int
}

const DURATION = 12.
const INTERVAL = .005
const INVALID_X = -math.MaxFloat64
const INVALID_Y = math.MinInt

//export mpv_open_cplugin
func mpv_open_cplugin(mpv *C.struct_mpv_handle) C.int {
	name := C.CString("pause")
	errno := C.bridge_mpv_observe_property(mpv, 0, name, C.MPV_FORMAT_NONE)
	C.free(unsafe.Pointer(name))
	if errno < 0 {
		log.Print(C.GoString(C.bridge_mpv_error_string(errno)))
		return errno
	}

	var (
		comments           []danmaku
		ctx, cancel        = context.WithCancel(context.Background())
		enabled, available atomic.Bool
	)
	get := func(ctx context.Context) {
		path, errno := getPropertyString(mpv, "path")
		if errno < 0 {
			return
		}
		var err error
		comments, err = dandanplayComments(ctx, path)
		if err != nil {
			log.Print(err)
			if enabled.Load() {
				showText(mpv, "dandanplay: "+err.Error())
			}
			return
		}
		available.Store(true)
		if enabled.Load() {
			if pause, err := getPropertyFlag(mpv, "pause"); err >= 0 && pause {
				render(mpv, comments)
			}
			loaded(mpv, len(comments))
		}
	}
	for {
		timeout := -1.
		if enabled.Load() {
			if pause, err := getPropertyFlag(mpv, "pause"); err >= 0 && !pause {
				timeout = INTERVAL
			}
		}
		event := C.bridge_mpv_wait_event(mpv, C.double(timeout))
		switch event.event_id {
		case C.MPV_EVENT_SHUTDOWN:
			cancel()
			return 0

		case C.MPV_EVENT_FILE_LOADED:
			cancel()
			ctx, cancel = context.WithCancel(context.Background())
			available.Store(false)
			if enabled.Load() {
				if err := C.remove_overlay(mpv); err < 0 {
					log.Print(C.GoString(C.bridge_mpv_error_string(err)))
				}
				go get(ctx)
			}

		case C.MPV_EVENT_PROPERTY_CHANGE:

		case C.MPV_EVENT_CLIENT_MESSAGE:
			data := (*C.mpv_event_client_message)(event.data)
			args := unsafe.Slice(data.args, data.num_args)
			if len(args) == 0 || C.GoString(args[0]) != "toggle-danmaku" {
				break
			}
			value := !enabled.Load()
			enabled.Store(value)
			if value {
				if available.Load() {
					reset(mpv, comments)
					loaded(mpv, len(comments))
				} else {
					showText(mpv, "Danmaku: on")
					cancel()
					ctx, cancel = context.WithCancel(context.Background())
					go get(ctx)
				}
			} else {
				if err := C.remove_overlay(mpv); err < 0 {
					log.Print(C.GoString(C.bridge_mpv_error_string(err)))
				}
				showText(mpv, "Danmaku: off")
			}

		case C.MPV_EVENT_SEEK:
			if enabled.Load() && available.Load() {
				reset(mpv, comments)
			}
		}

		if enabled.Load() && available.Load() {
			render(mpv, comments)
		}
	}
}

func render(mpv *C.mpv_handle, comments []danmaku) {
	w, err := getPropertyDouble(mpv, "osd-width")
	if err < 0 || w == 0 {
		return
	}
	h, err := getPropertyDouble(mpv, "osd-height")
	if err < 0 || h == 0 {
		return
	}
	pos, err := getPropertyDouble(mpv, "time-pos")
	if err < 0 {
		return
	}
	size, err := getPropertyDouble(mpv, "osd-font-size")
	if err < 0 {
		return
	}
	speed, err := getPropertyDouble(mpv, "speed")
	if err < 0 {
		return
	}
	// https://mpv.io/manual/stable/#options-sub-font-size
	size = size / 720 * h / 2
	spacing := size / 10
	rows := make([]float64, int(math.Max(h/(size+spacing), 1)))
	for i := range rows {
		rows[i] = INVALID_X
	}

	var danmaku []string
	for i := range comments {
		comment := &comments[i]
		if comment.Time > pos+DURATION/2 {
			break
		}

		if comment.X == INVALID_X {
			comment.X = w - (pos-comment.Time)*w/DURATION
		}
		if comment.X+float64(comment.GraphemeClusterCount)*size+spacing < 0 {
			continue
		}
		if comment.Y < 0 {
			for i, row := range rows {
				if row < comment.X {
					comment.Y = i
					break
				}
			}
			if comment.Y < 0 {
				for i, row := range rows {
					if comment.Y < 0 || row < rows[comment.Y] {
						comment.Y = i
					}
				}
			}
		}

		danmaku = append(danmaku,
			fmt.Sprintf("{\\pos(%f,%f)\\c&H%x%x%x&\\alpha&H30\\fscx50\\fscy50\\bord1.5\\b1\\q2}%s",
				comment.X, float64(comment.Y)*(size+spacing),
				comment.B, comment.G, comment.R,
				comment.Message))
		comment.X -= w / DURATION * speed * INTERVAL
		if comment.Y < len(rows) {
			rows[comment.Y] = math.Max(rows[comment.Y], comment.X+float64(comment.GraphemeClusterCount)*size+spacing)
		}
	}
	data := C.CString(strings.Join(danmaku, "\n"))
	if err = C.osd_overlay(mpv, data, C.int64_t(w), C.int64_t(h)); err < 0 {
		log.Print(C.GoString(C.bridge_mpv_error_string(err)))
	}
	C.free(unsafe.Pointer(data))
}

func reset(mpv *C.mpv_handle, danmaku []danmaku) {
	for i := range danmaku {
		danmaku[i].X = INVALID_X
		danmaku[i].Y = INVALID_Y
	}
}

func loaded(mpv *C.mpv_handle, n int) {
	text := fmt.Sprintf("Loaded %d danmaku comment", n)
	if n > 1 {
		text += "s"
	}
	showText(mpv, text)
}

func showText(mpv *C.mpv_handle, text string) {
	data := C.CString(text)
	if err := C.show_text(mpv, data); err < 0 {
		log.Print(C.GoString(C.bridge_mpv_error_string(err)))
	}
	C.free(unsafe.Pointer(data))
}

func getPropertyDouble(mpv *C.mpv_handle, name string) (float64, C.int) {
	n := C.CString(name)
	var data C.double
	err := C.bridge_mpv_get_property(mpv, n, C.MPV_FORMAT_DOUBLE, unsafe.Pointer(&data))
	if err < 0 {
		log.Print(C.GoString(C.bridge_mpv_error_string(err)))
	}
	C.free(unsafe.Pointer(n))
	return float64(data), err
}

func getPropertyFlag(mpv *C.mpv_handle, name string) (bool, C.int) {
	n := C.CString(name)
	var data C.int
	err := C.bridge_mpv_get_property(mpv, n, C.MPV_FORMAT_FLAG, unsafe.Pointer(&data))
	if err < 0 {
		log.Print(C.GoString(C.bridge_mpv_error_string(err)))
	}
	C.free(unsafe.Pointer(n))
	return data != 0, err
}

func getPropertyString(mpv *C.mpv_handle, name string) (string, C.int) {
	n := C.CString(name)
	var data *C.char
	err := C.bridge_mpv_get_property(mpv, n, C.MPV_FORMAT_STRING, unsafe.Pointer(&data))
	C.free(unsafe.Pointer(n))
	if err < 0 {
		log.Print(C.GoString(C.bridge_mpv_error_string(err)))
		return "", err
	}
	value := C.GoString(data)
	C.bridge_mpv_free(unsafe.Pointer(data))
	return value, 0
}

func main() {
}
