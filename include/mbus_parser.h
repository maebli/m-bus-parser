#ifndef MBUS_PARSER_H
#define MBUS_PARSER_H

#ifdef __cplusplus
extern "C" {
#endif

#ifdef __cplusplus
}
#endif


typedef enum {
    ParseOk,
    ParseError,
} ParseStatus;


ParseStatus parse_mbus(const uint8_t* data, size_t length);

#endif // MBUS_PARSER_H