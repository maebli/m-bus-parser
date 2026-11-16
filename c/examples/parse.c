#include "mbus_parser.h"
#include <stdio.h>

int main(void) {
    const uint8_t frame[] = {0x10, 0x40, 0x01, 0x41, 0x16};
    if (mbus_abi_version() != MBUS_ABI_VERSION) {
        return 1;
    }
    MbusResult *result = mbus_parse(frame, sizeof(frame), NULL, 0);
    const char *error_code = mbus_result_error_code(result);
    int status = error_code == NULL ? 0 : 1;
    FILE *stream = status == 0 ? stdout : stderr;
    if (error_code != NULL) {
        fprintf(stream, "[%s] ", error_code);
    }
    fwrite(mbus_result_text(result), 1, mbus_result_text_length(result), stream);
    fputc('\n', stream);
    mbus_result_free(result);
    return status;
}
