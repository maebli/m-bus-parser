#ifndef MBUS_PARSER_H
#define MBUS_PARSER_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MbusResult MbusResult;

#define MBUS_ABI_VERSION 1
#define MBUS_FORMAT_JSON 0
#define MBUS_FORMAT_YAML 1
#define MBUS_FORMAT_CSV 2
#define MBUS_FORMAT_XML 3
#define MBUS_FORMAT_TABLE 4
#define MBUS_FORMAT_MERMAID 5
#define MBUS_FORMAT_ANNOTATED 6
#define MBUS_FORMAT_ANNOTATED_TEXT 7

uint32_t mbus_abi_version(void);
const char *mbus_version(void);
MbusResult *mbus_parse(const uint8_t *data, size_t data_length,
                       const uint8_t *key, size_t key_length);
MbusResult *mbus_render(const uint8_t *data, size_t data_length,
                        const uint8_t *key, size_t key_length, int32_t format);
MbusResult *mbus_parse_records(const uint8_t *data, size_t data_length);
const char *mbus_result_text(const MbusResult *result);
size_t mbus_result_text_length(const MbusResult *result);
const char *mbus_result_error_code(const MbusResult *result);
size_t mbus_result_error_code_length(const MbusResult *result);
void mbus_result_free(MbusResult *result);

#ifdef __cplusplus
}
#endif

#endif
