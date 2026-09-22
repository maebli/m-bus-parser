#include <stddef.h>
#include <stdint.h>
#include <sys/stat.h>
#include <errno.h>

struct comparison_decoded { uint32_t records, errors; };
extern struct comparison_decoded comparison_decode(const unsigned char *, size_t);
#include "fixtures.h"

/* No interrupts or background tasks. The watermark covers only the decoder
 * invocation, including its callees and allocator. Reporting uses the boot stack. */
static uint32_t decode_stack[16384] __attribute__((aligned(8)));
static uint8_t heap[256 * 1024] __attribute__((aligned(8)));
static size_t heap_end;
static volatile struct comparison_decoded result;
static volatile size_t frame_index;

static void semihost(unsigned operation, const void *argument) {
    register unsigned r0 __asm__("r0") = operation;
    register const void *r1 __asm__("r1") = argument;
    __asm__ volatile("bkpt 0xab" : "+r"(r0) : "r"(r1) : "memory");
}
static void print_number(uint32_t number) {
    char text[16];
    char *end = text + sizeof(text) - 1;
    *end = 0;
    do { *--end = '0' + number % 10; number /= 10; } while(number);
    semihost(4, end);
}
static void report(const char *key, uint32_t number) {
    semihost(4, key); print_number(number); semihost(4, "\n");
}
static void decode_one(void) {
    const struct fixture *frame = &fixtures[frame_index];
    result = comparison_decode(frame->data, frame->length);
}
/* Switch to the watermark stack without spilling any reporting/harness state
 * onto it. r4/r5 are callee-saved by the ABI, including across decode_one. */
__attribute__((naked, noinline, noipa)) static void on_stack(void (*function)(void), void *top) {
    __asm__ volatile(
        "push {r4, r5, r6, lr}\n"
        "mov r4, sp\n"
        "mov sp, r1\n"
        "blx r0\n"
        "mov sp, r4\n"
        "pop {r4, r5, r6, pc}\n"
    );
}

void *_sbrk(ptrdiff_t increment) {
    if (increment < 0 || (size_t)increment > sizeof(heap) - heap_end) {
        errno = ENOMEM; return (void *)-1;
    }
    void *pointer = &heap[heap_end];
    heap_end += increment;
    return pointer;
}
int _write(int fd, const void *data, size_t length) { (void)fd; (void)data; return length; }
int _read(int fd, void *data, size_t length) { (void)fd; (void)data; (void)length; return 0; }
int _close(int fd) { (void)fd; return -1; }
int _fstat(int fd, struct stat *st) { (void)fd; st->st_mode = S_IFCHR; return 0; }
int _isatty(int fd) { (void)fd; return 1; }
int _lseek(int fd, int offset, int whence) { (void)fd; (void)offset; (void)whence; return 0; }
int _getpid(void) { return 1; }
int _kill(int pid, int signal) { (void)pid; (void)signal; return -1; }
void _exit(int status) {
    const uint32_t args[] = {0x20026, (uint32_t)status};
    semihost(0x20, args);
    while(1) {}
}
extern uint32_t _sidata[], _sdata[], _edata[], _sbss[], _ebss[];
void _start(void) {
    for (uint32_t *dst = _sdata, *src = _sidata; dst < _edata;) *dst++ = *src++;
    for (uint32_t *dst = _sbss; dst < _ebss;) *dst++ = 0;
    size_t max_stack = 0;
    uint32_t records = 0, errors = 0;
    for (frame_index = 0; frame_index < sizeof(fixtures)/sizeof(fixtures[0]); frame_index++) {
        for (size_t i = 0; i < sizeof(decode_stack)/sizeof(decode_stack[0]); i++) decode_stack[i] = 0xa55ac33c;
        on_stack(decode_one, decode_stack + sizeof(decode_stack)/sizeof(decode_stack[0]));
        size_t untouched = 0;
        while (untouched < sizeof(decode_stack)/sizeof(decode_stack[0]) && decode_stack[untouched] == 0xa55ac33c) untouched++;
        if (!untouched) _exit(2); /* fail rather than report stack overflow */
        size_t used = sizeof(decode_stack) - untouched * sizeof(uint32_t);
        if (used > max_stack) max_stack = used;
        records += result.records; errors += result.errors;
    }
    report("stack=", max_stack);
    report("heap_reserved=", heap_end);
    report("records=", records);
    report("errors=", errors);
    _exit(0);
}
__attribute__((section(".vectors"), used)) const uintptr_t vectors[] = {0x20400000, (uintptr_t)_start};
