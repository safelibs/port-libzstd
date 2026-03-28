#define main upstream_invalid_dictionaries_main
#include "../../../original/libzstd-1.5.5+dfsg2/tests/invalidDictionaries.c"
#undef main

int main(void)
{
    return upstream_invalid_dictionaries_main(0, (const char**)0);
}
