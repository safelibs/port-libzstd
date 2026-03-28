#define main upstream_paramgrill_main
#include "../../../original/libzstd-1.5.5+dfsg2/tests/paramgrill.c"
#undef main

int main(void)
{
    const char* args[] = {
        "paramgrill",
        "-i1",
        "-s64K"
    };

    return upstream_paramgrill_main((int)(sizeof(args) / sizeof(args[0])), args);
}
