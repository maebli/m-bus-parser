ENTRY(_start)
MEMORY {
    FLASH (rx) : ORIGIN = 0x00000000, LENGTH = 4M
    RAM (rwx) : ORIGIN = 0x20000000, LENGTH = 4M
}
SECTIONS {
    .vectors : { KEEP(*(.vectors)) } > FLASH
    .text : { *(.text*) *(.rodata*) } > FLASH
    .ARM.exidx : { *(.ARM.exidx*) } > FLASH
    .fixtures : { KEEP(*(.fixtures)) } > FLASH
    .data : { _sdata = .; *(.data*) _edata = .; } > RAM AT> FLASH
    _sidata = LOADADDR(.data);
    .bss (NOLOAD) : { _sbss = .; *(.bss*) *(COMMON) _ebss = .; } > RAM
    /DISCARD/ : { *(.comment) *(.note*) }
}
