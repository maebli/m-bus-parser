package io.github.maebli.mbus;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SymbolLookup;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandle;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.HexFormat;
import java.util.Objects;

public final class MbusParser {
    private final MethodHandle render;
    private final MethodHandle parseRecords;
    private final MethodHandle resultText;
    private final MethodHandle resultTextLength;
    private final MethodHandle resultErrorCode;
    private final MethodHandle resultErrorCodeLength;
    private final MethodHandle resultFree;

    public MbusParser() {
        this(NativeLibrary.path());
    }

    @SuppressWarnings("restricted")
    public MbusParser(Path library) {
        if (ValueLayout.ADDRESS.byteSize() != Long.BYTES) {
            throw new UnsupportedOperationException("The M-Bus binding requires a 64-bit JVM");
        }
        SymbolLookup symbols = SymbolLookup.libraryLookup(Objects.requireNonNull(library), Arena.ofAuto());
        MethodHandle abiVersion = function(symbols, "mbus_abi_version", FunctionDescriptor.of(ValueLayout.JAVA_INT));
        try {
            if ((int) abiVersion.invokeExact() != 1) {
                throw new UnsatisfiedLinkError("The native library has an incompatible M-Bus ABI");
            }
        } catch (RuntimeException | Error error) {
            throw error;
        } catch (Throwable error) {
            throw new IllegalStateException("Cannot check the native M-Bus ABI", error);
        }
        render = function(symbols, "mbus_render", FunctionDescriptor.of(ValueLayout.ADDRESS,
                ValueLayout.ADDRESS, ValueLayout.JAVA_LONG, ValueLayout.ADDRESS, ValueLayout.JAVA_LONG,
                ValueLayout.JAVA_INT));
        parseRecords = function(symbols, "mbus_parse_records", FunctionDescriptor.of(ValueLayout.ADDRESS,
                ValueLayout.ADDRESS, ValueLayout.JAVA_LONG));
        FunctionDescriptor pointerResult = FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS);
        FunctionDescriptor lengthResult = FunctionDescriptor.of(ValueLayout.JAVA_LONG, ValueLayout.ADDRESS);
        resultText = function(symbols, "mbus_result_text", pointerResult);
        resultTextLength = function(symbols, "mbus_result_text_length", lengthResult);
        resultErrorCode = function(symbols, "mbus_result_error_code", pointerResult);
        resultErrorCodeLength = function(symbols, "mbus_result_error_code_length", lengthResult);
        resultFree = function(symbols, "mbus_result_free", FunctionDescriptor.ofVoid(ValueLayout.ADDRESS));
    }

    public String parse(byte[] telegram) {
        return parse(telegram, null);
    }

    public String parse(byte[] telegram, byte[] key) {
        return render(telegram, OutputFormat.JSON, key);
    }

    public String parseHex(String telegram) {
        return parse(HexFormat.of().parseHex(telegram));
    }

    public String render(byte[] telegram, OutputFormat format) {
        return render(telegram, format, null);
    }

    public String render(byte[] telegram, OutputFormat format, byte[] key) {
        Objects.requireNonNull(telegram, "telegram");
        Objects.requireNonNull(format, "format");
        if (key != null && key.length != 16) {
            throw new MbusException("option.invalid", "The key must contain exactly 16 bytes");
        }
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment data = copy(arena, telegram);
            MemorySegment nativeKey = key == null ? MemorySegment.NULL : copy(arena, key);
            long keyLength = key == null ? 0L : key.length;
            try {
                MemorySegment result = (MemorySegment) render.invokeExact(data, (long) telegram.length,
                        nativeKey, keyLength, format.nativeValue());
                return readAndFree(result);
            } finally {
                if (key != null) {
                    nativeKey.fill((byte) 0);
                }
            }
        } catch (RuntimeException | Error error) {
            throw error;
        } catch (Throwable error) {
            throw new IllegalStateException("Cannot call the native M-Bus parser", error);
        }
    }

    public String parseRecords(byte[] records) {
        Objects.requireNonNull(records, "records");
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment result = (MemorySegment) parseRecords.invokeExact(copy(arena, records), (long) records.length);
            return readAndFree(result);
        } catch (RuntimeException | Error error) {
            throw error;
        } catch (Throwable error) {
            throw new IllegalStateException("Cannot call the native M-Bus parser", error);
        }
    }

    private String readAndFree(MemorySegment result) throws Throwable {
        if (result.equals(MemorySegment.NULL)) {
            throw new OutOfMemoryError("The native M-Bus parser returned no result");
        }
        try {
            MemorySegment text = (MemorySegment) resultText.invokeExact(result);
            long length = (long) resultTextLength.invokeExact(result);
            String output = readUtf8(text, length);
            MemorySegment errorCode = (MemorySegment) resultErrorCode.invokeExact(result);
            if (!errorCode.equals(MemorySegment.NULL)) {
                long codeLength = (long) resultErrorCodeLength.invokeExact(result);
                throw new MbusException(readUtf8(errorCode, codeLength), output);
            }
            return output;
        } finally {
            resultFree.invokeExact(result);
        }
    }

    @SuppressWarnings("restricted")
    private static String readUtf8(MemorySegment address, long length) {
        return new String(address.reinterpret(length).toArray(ValueLayout.JAVA_BYTE), StandardCharsets.UTF_8);
    }

    private static MemorySegment copy(Arena arena, byte[] bytes) {
        return bytes.length == 0 ? MemorySegment.NULL : arena.allocateFrom(ValueLayout.JAVA_BYTE, bytes);
    }

    @SuppressWarnings("restricted")
    private static MethodHandle function(SymbolLookup symbols, String name, FunctionDescriptor descriptor) {
        return Linker.nativeLinker().downcallHandle(symbols.findOrThrow(name), descriptor);
    }
}
