#include "mpv/client.h"

mpv_event *bridge_mpv_wait_event(mpv_handle *mpv, double timeout) {
  return mpv_wait_event(mpv, timeout);
}

int bridge_mpv_observe_property(mpv_handle *mpv, uint64_t reply_userdata,
                                const char *name, mpv_format format) {
  return mpv_observe_property(mpv, reply_userdata, name, format);
}

const char *bridge_mpv_error_string(int error) {
  return mpv_error_string(error);
}

int bridge_mpv_get_property(mpv_handle *mpv, const char *name,
                            mpv_format format, void *data) {
  return mpv_get_property(mpv, name, format, data);
}

void bridge_mpv_free(void *data) { mpv_free(data); }

int emit_danmaku(mpv_handle *mpv, const char *data) {
  const char *args[] = {"script-message", "emit-danmaku", data, NULL};
  return mpv_command(mpv, args);
}
