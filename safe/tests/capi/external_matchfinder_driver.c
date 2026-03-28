#define main upstream_external_matchfinder_main
#include "../../../original/libzstd-1.5.5+dfsg2/tests/external_matchfinder.c"
#undef main

int main(void)
{
    return upstream_external_matchfinder_main();
}
