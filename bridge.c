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

int osd_overlay(mpv_handle *mpv, char *data, int64_t w, int64_t h) {
  char *keys[] = {"name", "id", "format", "data", "res_x", "res_y"};
  mpv_node values[] = {{.format = MPV_FORMAT_STRING, .u.string = "osd-overlay"},
                       {.format = MPV_FORMAT_INT64, .u.int64 = 0},
                       {.format = MPV_FORMAT_STRING, .u.string = "ass-events"},
                       {.format = MPV_FORMAT_STRING, .u.string = data},
                       {.format = MPV_FORMAT_INT64, .u.int64 = w},
                       {.format = MPV_FORMAT_INT64, .u.int64 = h}};
  mpv_node_list list = {.num = 6, .values = values, .keys = keys};
  mpv_node args = {.format = MPV_FORMAT_NODE_MAP, .u.list = &list};
  return mpv_command_node(mpv, &args, NULL);
}

int remove_overlay(mpv_handle *mpv) {
  char *keys[] = {"name", "id", "format", "data"};
  mpv_node values[] = {{.format = MPV_FORMAT_STRING, .u.string = "osd-overlay"},
                       {.format = MPV_FORMAT_INT64, .u.int64 = 0},
                       {.format = MPV_FORMAT_STRING, .u.string = "none"},
                       {.format = MPV_FORMAT_STRING, .u.string = ""}};
  mpv_node_list list = {.num = 4, .values = values, .keys = keys};
  mpv_node args = {.format = MPV_FORMAT_NODE_MAP, .u.list = &list};
  return mpv_command_node(mpv, &args, NULL);
}

int show_text(mpv_handle *mpv, const char *text) {
  const char *args[] = {"show-text", text, NULL};
  return mpv_command(mpv, args);
}
