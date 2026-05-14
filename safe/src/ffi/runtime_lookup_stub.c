#if defined(__GNUC__)
__attribute__((visibility("hidden")))
#endif
void *dlsyx(void *handle, const char *symbol) {
    (void)handle;
    (void)symbol;
    return 0;
}
