// Independent Weston 13 harness: send a run-time address as wl_output model data.
// The client receives it through the real compositor connection, without argv,
// environment access, a socket bridge, or an additional capability.
#include <arpa/inet.h>
#include <stdio.h>
#include <stdlib.h>
struct weston_compositor;
struct weston_head;
// Weston 13.0.0: include/libweston/libweston.h and libweston/backend.h.
extern struct weston_head *weston_compositor_iterate_heads(struct weston_compositor *, struct weston_head *);
extern void weston_head_set_monitor_strings(struct weston_head *, const char *, const char *, const char *);
int wet_module_init(struct weston_compositor *compositor, int *argc, char *argv[]) {
    (void)argc;
    (void)argv;
    const char *address = getenv("LKJSCRIPT_PROOF_TCP_ADDRESS");
    struct in_addr parsed;
    if (!address || inet_pton(AF_INET, address, &parsed) != 1) {
        fputs("anonymous frame proof: missing numeric IPv4 harness input\n", stderr);
        return -1;
    }
    struct weston_head *head = NULL;
    unsigned count = 0;
    while ((head = weston_compositor_iterate_heads(compositor, head))) {
        weston_head_set_monitor_strings(head, "lkjscript proof harness", address, "anonymous-frame");
        ++count;
    }
    fprintf(stderr, "anonymous frame proof: runtime model=%s heads=%u\n", address, count);
    return count == 1 ? 0 : -1;
}
