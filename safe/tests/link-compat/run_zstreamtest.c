#include <stddef.h>

int upstream_zstreamtest_main(int argc, char** argv);

int main(void)
{
    char arg0[] = "upstream_zstreamtest";
    char arg1[] = "-t2";
    char* argv[] = { arg0, arg1, NULL };
    return upstream_zstreamtest_main(2, argv);
}
