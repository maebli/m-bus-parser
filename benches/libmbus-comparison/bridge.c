/* A small ownership boundary. Parser/record/XML allocation and cleanup are
 * timed; fixtures and the stack frame itself are not heap-allocated. */
#include "mbus-protocol.h"
#include "mbus-protocol-aux.h"

int comparison_frame(const unsigned char *bytes, size_t length) {
    mbus_frame frame = {0};
    /* The pinned mbus_parse implementation reads but never modifies input. */
    return mbus_parse(&frame, (unsigned char *)bytes, length);
}

char *comparison_xml(const unsigned char *bytes, size_t length) {
    mbus_frame frame = {0};
    mbus_frame_data data = {0};
    if (mbus_parse(&frame, (unsigned char *)bytes, length) != 0)
        return NULL;
    int status = mbus_frame_data_parse(&frame, &data);
    char *xml = status == 0 ? mbus_frame_data_xml_normalized(&data) : NULL;
    if (data.data_var.record)
        mbus_data_record_free(data.data_var.record);
    return xml;
}

void comparison_free(char *xml) { free(xml); }

struct comparison_decoded { uint32_t records, errors; };

static void consume_record(mbus_record *record, struct comparison_decoded *result) {
    if (record) {
        /* Make the complete public result observable without rendering XML. */
        __asm__ volatile ("" : : "r"(record) : "memory");
        result->records++;
        mbus_record_free(record);
    } else {
        result->errors++;
    }
}

struct comparison_decoded comparison_decode(const unsigned char *bytes, size_t length) {
    struct comparison_decoded result = {0};
    mbus_frame frame = {0};
    mbus_frame_data data = {0};
    if (mbus_parse(&frame, (unsigned char *)bytes, length) != 0) {
        result.errors++;
        return result;
    }
    if (mbus_frame_data_parse(&frame, &data) != 0) {
        result.errors++;
    } else if (data.type == MBUS_DATA_TYPE_FIXED) {
        consume_record(mbus_parse_fixed_record(data.data_fix.status, data.data_fix.cnt1_type, data.data_fix.cnt1_val), &result);
        consume_record(mbus_parse_fixed_record(data.data_fix.status, data.data_fix.cnt2_type, data.data_fix.cnt2_val), &result);
    } else if (data.type == MBUS_DATA_TYPE_VARIABLE) {
        for (mbus_data_record *record = data.data_var.record; record; record = record->next)
            consume_record(mbus_parse_variable_record(record), &result);
    }
    if (data.data_var.record) mbus_data_record_free(data.data_var.record);
    return result;
}
