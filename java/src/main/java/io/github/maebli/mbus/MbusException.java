package io.github.maebli.mbus;

public final class MbusException extends IllegalArgumentException {
    private static final long serialVersionUID = 1L;
    private final String code;

    public MbusException(String code, String message) {
        super("[" + code + "] " + message);
        this.code = code;
    }

    public String code() {
        return code;
    }
}
