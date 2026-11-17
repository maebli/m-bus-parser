package io.github.maebli.mbus;

public enum OutputFormat {
    JSON(0), YAML(1), CSV(2), XML(3), TABLE(4), MERMAID(5), ANNOTATED(6), ANNOTATED_TEXT(7);

    private final int nativeValue;

    OutputFormat(int nativeValue) {
        this.nativeValue = nativeValue;
    }

    int nativeValue() {
        return nativeValue;
    }
}
